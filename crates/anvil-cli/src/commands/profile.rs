use anyhow::Result;
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process::Command;
use crate::output::{self, fmt_num};

#[derive(Args, Debug)]
pub struct ProfileArgs {
    /// Package to profile
    #[arg(short, long)]
    pub package: Option<String>,

    /// Filter tests by name (only profiled tests will be measured)
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Write a JSON profile report to this file
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Write a Markdown report to this file (useful for PR comments)
    #[arg(long)]
    pub markdown: Option<PathBuf>,

    /// Fail if any call exceeds this CPU instruction count
    #[arg(long)]
    pub max_cpu: Option<u64>,

    /// Fail if any call exceeds this memory byte count
    #[arg(long)]
    pub max_memory: Option<u64>,

    /// Fail if total CPU across all calls exceeds this limit
    #[arg(long)]
    pub max_total_cpu: Option<u64>,
}

pub async fn run(args: ProfileArgs, verbose: bool, quiet: bool) -> Result<()> {
    if !quiet {
        output::section("Profiling Contract Invocations");
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if let Some(pkg) = &args.package {
        cmd.args(["--package", pkg]);
    }

    cmd.args(["--features", "testutils"]);
    cmd.env("STELLAR_ANVIL_PROFILE", "1");

    let mut test_args = vec!["--nocapture".to_string()];
    if let Some(filter) = &args.filter {
        test_args.push(filter.clone());
    }
    cmd.arg("--").args(&test_args);

    if verbose {
        output::value("command", &format!("{:?}", cmd));
    }

    // Progress spinner
    let spinner = ProgressBar::new_spinner();
    if !quiet {
        spinner.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        spinner.set_message("Compiling and running profiled tests…");
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    }

    let start = std::time::Instant::now();
    let result = cmd.output()?;
    let elapsed = start.elapsed();

    if !quiet {
        spinner.finish_and_clear();
    }

    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    // Extract anvil profile lines from stdout.
    // Tests using anvil-profiler emit lines prefixed with "ANVIL_PROFILE|"
    let profile_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("ANVIL_PROFILE|"))
        .collect();

    if profile_lines.is_empty() {
        if !quiet {
            output::warn("No ANVIL_PROFILE lines found in output.");
            output::warn("Make sure tests use anvil_profiler::Profiler and STELLAR_ANVIL_PROFILE is set.");
            println!("{}", stderr.dimmed());
        }
    } else {
        let parsed = parse_profile_lines(&profile_lines);

        if !quiet {
            print_profile_table(&parsed, elapsed.as_secs_f64());
        }

        // Threshold checks — these emit errors even in quiet mode
        let mut threshold_failed = false;

        if let Some(max_cpu) = args.max_cpu {
            for r in &parsed {
                if r.cpu > max_cpu {
                    output::fail(&format!(
                        "'{}' exceeded CPU limit: {} > {}",
                        r.label, fmt_num(r.cpu), fmt_num(max_cpu)
                    ));
                    threshold_failed = true;
                }
            }
        }

        if let Some(max_mem) = args.max_memory {
            for r in &parsed {
                if r.mem > max_mem {
                    output::fail(&format!(
                        "'{}' exceeded memory limit: {} > {}",
                        r.label, fmt_num(r.mem), fmt_num(max_mem)
                    ));
                    threshold_failed = true;
                }
            }
        }

        if let Some(max_total) = args.max_total_cpu {
            let total: u64 = parsed.iter().map(|r| r.cpu).sum();
            if total > max_total {
                output::fail(&format!(
                    "Total CPU exceeded limit: {} > {}",
                    fmt_num(total), fmt_num(max_total)
                ));
                threshold_failed = true;
            }
        }

        // Export
        if let Some(report_path) = &args.report {
            let json = serde_json::to_string_pretty(&parsed)?;
            std::fs::write(report_path, json)?;
            if !quiet {
                output::ok(&format!("JSON report → {}", report_path.display()));
            }
        }

        if let Some(md_path) = &args.markdown {
            let md = build_markdown_report(&parsed);
            std::fs::write(md_path, md)?;
            if !quiet {
                output::ok(&format!("Markdown report → {}", md_path.display()));
            }
        }

        if threshold_failed {
            anyhow::bail!("profile thresholds exceeded");
        }
    }

    if !result.status.success() {
        anyhow::bail!("test run failed during profiling");
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct ParsedProfileLine {
    pub label: String,
    pub cpu: u64,
    pub mem: u64,
    pub fee_estimate: u64,
}

fn parse_profile_lines(lines: &[&str]) -> Vec<ParsedProfileLine> {
    // Expected format: ANVIL_PROFILE|<label>|cpu=<n>|mem=<n>
    lines
        .iter()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 {
                return None;
            }
            let label = parts[1].to_string();
            let cpu = parts[2]
                .strip_prefix("cpu=")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let mem = parts[3]
                .strip_prefix("mem=")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let fee_estimate = (cpu / 10_000) * 25 + (mem / 1_024);
            Some(ParsedProfileLine { label, cpu, mem, fee_estimate })
        })
        .collect()
}

fn print_profile_table(results: &[ParsedProfileLine], elapsed: f64) {
    if results.is_empty() {
        return;
    }

    // Dynamic label column width — longest label + 2, min 36, max 48.
    let label_w = results
        .iter()
        .map(|r| r.label.len())
        .max()
        .unwrap_or(36)
        .clamp(36, 48)
        + 2;

    // Fixed numeric column widths that keep total ≤ 78 chars.
    // label_w + 14 + 12 + 14 + 4 spaces = label_w + 44
    let num_cpu_w  = 14usize;
    let num_mem_w  = 12usize;
    let num_fee_w  = 14usize;
    let total_w    = label_w + num_cpu_w + num_mem_w + num_fee_w + 6; // 6 = spacing

    println!();
    output::heavy_divider();
    println!(
        "  {:label_w$} {:>num_cpu_w$} {:>num_mem_w$} {:>num_fee_w$}",
        "Invocation".bold(),
        "CPU insns".bold(),
        "Mem bytes".bold(),
        "Fee (stroops)".bold(),
        label_w = label_w,
        num_cpu_w = num_cpu_w,
        num_mem_w = num_mem_w,
        num_fee_w = num_fee_w,
    );
    output::divider();

    for r in results {
        // Truncate label to column width to prevent layout breakage.
        let label = if r.label.len() > label_w {
            format!("{}…", &r.label[..label_w - 1])
        } else {
            r.label.clone()
        };
        println!(
            "  {:label_w$} {:>num_cpu_w$} {:>num_mem_w$} {:>num_fee_w$}",
            label,
            fmt_num(r.cpu).cyan().to_string(),
            fmt_num(r.mem).yellow().to_string(),
            fmt_num(r.fee_estimate).magenta().to_string(),
            label_w = label_w,
            num_cpu_w = num_cpu_w,
            num_mem_w = num_mem_w,
            num_fee_w = num_fee_w,
        );
    }

    let total_cpu: u64 = results.iter().map(|r| r.cpu).sum();
    let total_mem: u64 = results.iter().map(|r| r.mem).sum();
    let total_fee: u64 = results.iter().map(|r| r.fee_estimate).sum();

    output::divider();
    println!(
        "  {:label_w$} {:>num_cpu_w$} {:>num_mem_w$} {:>num_fee_w$}",
        "TOTAL".bold(),
        fmt_num(total_cpu).cyan().bold().to_string(),
        fmt_num(total_mem).yellow().bold().to_string(),
        fmt_num(total_fee).magenta().bold().to_string(),
        label_w = label_w,
        num_cpu_w = num_cpu_w,
        num_mem_w = num_mem_w,
        num_fee_w = num_fee_w,
    );
    output::heavy_divider();

    // Color legend
    println!(
        "  {}  {}  {}  {}",
        format!("■ cyan = CPU instructions").cyan().dimmed(),
        format!("■ yellow = memory bytes").yellow().dimmed(),
        format!("■ magenta = fee estimate").magenta().dimmed(),
        format!("({:.2}s)", elapsed).dimmed(),
    );
    println!();

    let _ = total_w; // suppress unused warning
}

fn build_markdown_report(results: &[ParsedProfileLine]) -> String {
    let mut md = String::from("## Stellar Anvil — Profile Report\n\n");
    md.push_str("| Invocation | CPU insns | Mem bytes | Fee est. (stroops) |\n");
    md.push_str("|---|---:|---:|---:|\n");
    for r in results {
        md.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            r.label,
            fmt_num(r.cpu),
            fmt_num(r.mem),
            fmt_num(r.fee_estimate),
        ));
    }
    let total_cpu: u64 = results.iter().map(|r| r.cpu).sum();
    let total_fee: u64 = results.iter().map(|r| r.fee_estimate).sum();
    md.push_str(&format!(
        "| **TOTAL** | **{}** | — | **{}** |\n",
        fmt_num(total_cpu),
        fmt_num(total_fee),
    ));
    md
}
