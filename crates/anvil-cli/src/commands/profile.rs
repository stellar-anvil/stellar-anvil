use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;
use crate::output;

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

pub async fn run(args: ProfileArgs, verbose: bool) -> Result<()> {
    output::section("Profiling Contract Invocations");

    // Profile mode: run cargo test with the anvil_profile feature flag enabled
    // so tests can detect they're being profiled and emit structured output.
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

    output::step("Running tests with profiling enabled...");
    let start = std::time::Instant::now();
    let result = cmd.output()?;
    let elapsed = start.elapsed();

    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    // Extract anvil profile lines from stdout
    // Tests using anvil-profiler emit lines prefixed with "ANVIL_PROFILE|"
    let profile_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("ANVIL_PROFILE|"))
        .collect();

    if profile_lines.is_empty() {
        output::warn("No ANVIL_PROFILE lines found in output.");
        output::warn("Make sure your tests use anvil_profiler::Profiler and the STELLAR_ANVIL_PROFILE env var.");
        println!("{}", stderr.dimmed());
    } else {
        // Parse structured profile output
        let parsed = parse_profile_lines(&profile_lines);
        print_profile_table(&parsed, elapsed.as_secs_f64());

        // Threshold checks
        let mut threshold_failed = false;

        if let Some(max_cpu) = args.max_cpu {
            for r in &parsed {
                if r.cpu > max_cpu {
                    output::fail(&format!(
                        "'{}' exceeded CPU limit: {} > {}",
                        r.label, r.cpu, max_cpu
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
                        r.label, r.mem, max_mem
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
                    total, max_total
                ));
                threshold_failed = true;
            }
        }

        // Export
        if let Some(report_path) = &args.report {
            let json = serde_json::to_string_pretty(&parsed)?;
            std::fs::write(report_path, json)?;
            output::ok(&format!("JSON report → {}", report_path.display()));
        }

        if let Some(md_path) = &args.markdown {
            let md = build_markdown_report(&parsed);
            std::fs::write(md_path, md)?;
            output::ok(&format!("Markdown report → {}", md_path.display()));
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
    println!(
        "\n  {:<44} {:>14} {:>12} {:>14}",
        "Invocation".bold(),
        "CPU insns".bold(),
        "Mem bytes".bold(),
        "Fee (stroops)".bold(),
    );
    println!("  {}", "─".repeat(86).dimmed());

    for r in results {
        println!(
            "  {:<44} {:>14} {:>12} {:>14}",
            r.label,
            r.cpu.to_string().cyan(),
            r.mem.to_string().yellow(),
            r.fee_estimate.to_string().magenta(),
        );
    }

    let total_cpu: u64 = results.iter().map(|r| r.cpu).sum();
    let total_mem: u64 = results.iter().map(|r| r.mem).sum();
    let total_fee: u64 = results.iter().map(|r| r.fee_estimate).sum();

    println!("  {}", "─".repeat(86).dimmed());
    println!(
        "  {:<44} {:>14} {:>12} {:>14}",
        "TOTAL".bold(),
        total_cpu.to_string().cyan().bold(),
        total_mem.to_string().yellow().bold(),
        total_fee.to_string().magenta().bold(),
    );
    println!(
        "\n  {} measurements  ({:.2}s)",
        results.len().to_string().dimmed(),
        elapsed,
    );
    println!();
}

fn build_markdown_report(results: &[ParsedProfileLine]) -> String {
    let mut md = String::from("## Stellar Anvil — Profile Report\n\n");
    md.push_str("| Invocation | CPU insns | Mem bytes | Fee est. (stroops) |\n");
    md.push_str("|---|---:|---:|---:|\n");
    for r in results {
        md.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            r.label, r.cpu, r.mem, r.fee_estimate
        ));
    }
    let total_cpu: u64 = results.iter().map(|r| r.cpu).sum();
    let total_fee: u64 = results.iter().map(|r| r.fee_estimate).sum();
    md.push_str(&format!(
        "| **TOTAL** | **{}** | — | **{}** |\n",
        total_cpu, total_fee
    ));
    md
}
