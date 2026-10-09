use anyhow::Result;
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use crate::output;

/// Arguments for `stellar-anvil test`
#[derive(Args, Debug)]
pub struct TestArgs {
    /// Package to test (defaults to all workspace members)
    #[arg(short, long)]
    pub package: Option<String>,

    /// Filter tests by name substring
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Output format: pretty (default), json, tap
    #[arg(long, default_value = "pretty")]
    pub format: String,

    /// Export a profile report alongside test results
    #[arg(long)]
    pub profile: bool,

    /// Path to write JSON results (optional)
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Number of threads for parallel test execution
    #[arg(long, default_value = "4")]
    pub threads: usize,

    /// Show stdout/stderr from passing tests
    #[arg(long)]
    pub nocapture: bool,
}

pub async fn run(args: TestArgs, verbose: bool) -> Result<()> {
    output::section("Running Tests");

    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if let Some(pkg) = &args.package {
        cmd.args(["--package", pkg]);
    }

    cmd.args(["--features", "testutils"]);

    if args.nocapture {
        cmd.arg("--").arg("--nocapture");
    }

    if let Some(filter) = &args.filter {
        if !args.nocapture {
            cmd.arg("--");
        }
        cmd.arg(filter);
    }

    if verbose {
        output::value("command", &format!("{:?}", cmd));
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Running cargo test...");

    let start = Instant::now();
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let output_result = cmd.output()?;

    spinner.finish_and_clear();

    let elapsed = start.elapsed();

    let stdout = String::from_utf8_lossy(&output_result.stdout);
    let stderr = String::from_utf8_lossy(&output_result.stderr);

    let results = parse_cargo_test_output(&stdout);

    match args.format.as_str() {
        "json" => print_json_results(&results, elapsed.as_secs_f64()),
        "tap"  => print_tap_results(&results),
        _      => print_pretty_results(&results, &stdout, &stderr, elapsed.as_secs_f64(), verbose),
    }

    if let Some(out_path) = &args.output {
        let json = serde_json::to_string_pretty(&results)?;
        std::fs::write(out_path, json)?;
        output::ok(&format!("Results written to {}", out_path.display()));
    }

    if !output_result.status.success() {
        anyhow::bail!("test suite failed");
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub ignored: bool,
    pub duration_ms: Option<u64>,
}

fn parse_cargo_test_output(output: &str) -> Vec<TestResult> {
    let mut results = Vec::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("test ") {
            if let Some((name, status)) = rest.rsplit_once(" ... ") {
                let passed  = status.trim() == "ok";
                let ignored = status.trim() == "ignored";
                results.push(TestResult {
                    name: name.trim().to_string(),
                    passed,
                    ignored,
                    duration_ms: None,
                });
            }
        }
    }

    results
}

fn print_pretty_results(
    results: &[TestResult],
    raw_stdout: &str,
    raw_stderr: &str,
    elapsed_secs: f64,
    verbose: bool,
) {
    let passed  = results.iter().filter(|r| r.passed && !r.ignored).count();
    let failed  = results.iter().filter(|r| !r.passed && !r.ignored).count();
    let ignored = results.iter().filter(|r| r.ignored).count();

    for r in results {
        if r.ignored {
            println!("  {} {}", "-".dimmed(), r.name.dimmed());
        } else if r.passed {
            println!("  {} {}", "✓".green(), r.name);
        } else {
            println!("  {} {}", "✗".red().bold(), r.name.red());
        }
    }

    if failed > 0 || verbose {
        if !raw_stderr.is_empty() {
            println!("\n{}", "── stderr ─────────────────────────────".dimmed());
            for line in raw_stderr.lines() {
                println!("  {}", line.dimmed());
            }
        }
        if verbose && !raw_stdout.is_empty() {
            println!("\n{}", "── stdout ─────────────────────────────".dimmed());
            for line in raw_stdout.lines() {
                println!("  {}", line.dimmed());
            }
        }
    }

    println!("\n{}", "─".repeat(60).dimmed());

    let summary = if failed == 0 {
        format!(
            "{} {} passed, {} ignored  ({:.2}s)",
            "✓".green().bold(),
            passed,
            ignored,
            elapsed_secs,
        )
    } else {
        format!(
            "{} {} passed  {} failed  {} ignored  ({:.2}s)",
            "✗".red().bold(),
            passed,
            failed.to_string().red().bold(),
            ignored,
            elapsed_secs,
        )
    };

    println!("  {}", summary);
    println!();
}

fn print_json_results(results: &[TestResult], elapsed_secs: f64) {
    #[derive(serde::Serialize)]
    struct JsonOutput<'a> {
        elapsed_secs: f64,
        results: &'a [TestResult],
        passed: usize,
        failed: usize,
        ignored: usize,
    }
    let out = JsonOutput {
        elapsed_secs,
        results,
        passed:  results.iter().filter(|r| r.passed && !r.ignored).count(),
        failed:  results.iter().filter(|r| !r.passed && !r.ignored).count(),
        ignored: results.iter().filter(|r| r.ignored).count(),
    };
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn print_tap_results(results: &[TestResult]) {
    println!("TAP version 14");
    println!("1..{}", results.len());
    for (i, r) in results.iter().enumerate() {
        if r.passed {
            println!("ok {} - {}", i + 1, r.name);
        } else if r.ignored {
            println!("ok {} - {} # SKIP", i + 1, r.name);
        } else {
            println!("not ok {} - {}", i + 1, r.name);
        }
    }
}
