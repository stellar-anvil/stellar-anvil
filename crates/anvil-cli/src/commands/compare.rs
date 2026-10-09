use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;
use crate::output;

#[derive(Args, Debug)]
pub struct CompareArgs {
    /// Baseline profile JSON (e.g. from main branch)
    pub baseline: PathBuf,

    /// Candidate profile JSON (e.g. from your feature branch)
    pub candidate: PathBuf,

    /// Fail if any call regressed by more than this percent (0–100)
    #[arg(long, default_value = "10")]
    pub threshold: f64,

    /// Output a Markdown comparison to this file
    #[arg(long)]
    pub markdown: Option<PathBuf>,
}

#[derive(Debug, serde::Deserialize)]
struct ProfileEntry {
    label: String,
    cpu: u64,
    mem: u64,
    #[serde(default)]
    fee_estimate: u64,
}

pub async fn run(args: CompareArgs, _verbose: bool) -> Result<()> {
    output::section("Comparing Profile Reports");
    output::value("baseline",  &args.baseline.display().to_string());
    output::value("candidate", &args.candidate.display().to_string());
    output::value("threshold", &format!("{}%", args.threshold));

    let baseline_raw  = std::fs::read_to_string(&args.baseline)?;
    let candidate_raw = std::fs::read_to_string(&args.candidate)?;

    let baseline:  Vec<ProfileEntry> = serde_json::from_str(&baseline_raw)?;
    let candidate: Vec<ProfileEntry> = serde_json::from_str(&candidate_raw)?;

    println!(
        "\n  {:<44} {:>10} {:>10} {:>8}   {:>10} {:>10} {:>8}",
        "Invocation".bold(),
        "Base CPU".bold(),
        "New CPU".bold(),
        "ΔCPU%".bold(),
        "Base Mem".bold(),
        "New Mem".bold(),
        "ΔMem%".bold(),
    );
    println!("  {}", "─".repeat(104).dimmed());

    let mut regressions = 0usize;
    let mut improvements = 0usize;
    let mut md_rows = Vec::new();

    for base in &baseline {
        let cand = candidate.iter().find(|c| c.label == base.label);
        let Some(cand) = cand else { continue };

        let cpu_delta = delta_pct(base.cpu, cand.cpu);
        let mem_delta = delta_pct(base.mem, cand.mem);

        let cpu_flag = color_delta(cpu_delta);
        let mem_flag = color_delta(mem_delta);

        println!(
            "  {:<44} {:>10} {:>10} {}   {:>10} {:>10} {}",
            base.label,
            base.cpu.to_string().dimmed(),
            cand.cpu.to_string().cyan(),
            cpu_flag,
            base.mem.to_string().dimmed(),
            cand.mem.to_string().yellow(),
            mem_flag,
        );

        if cpu_delta > args.threshold || mem_delta > args.threshold {
            regressions += 1;
        } else if cpu_delta < -2.0 || mem_delta < -2.0 {
            improvements += 1;
        }

        md_rows.push(format!(
            "| `{}` | {} | {} | {:+.1}% | {} | {} | {:+.1}% |",
            base.label, base.cpu, cand.cpu, cpu_delta, base.mem, cand.mem, mem_delta,
        ));
    }

    println!("  {}", "─".repeat(104).dimmed());

    println!();
    output::value("regressions",  &regressions.to_string());
    output::value("improvements", &improvements.to_string());

    if let Some(md_path) = &args.markdown {
        let mut md = String::from("## Stellar Anvil — Regression Report\n\n");
        md.push_str("| Invocation | Base CPU | New CPU | ΔCPU% | Base Mem | New Mem | ΔMem% |\n");
        md.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
        for row in &md_rows {
            md.push_str(row);
            md.push('\n');
        }
        std::fs::write(md_path, md)?;
        output::ok(&format!("Markdown report → {}", md_path.display()));
    }

    if regressions > 0 {
        anyhow::bail!(
            "{} regression(s) exceeded {}% threshold",
            regressions,
            args.threshold
        );
    }

    output::ok(&format!(
        "No regressions above {}% threshold",
        args.threshold
    ));
    Ok(())
}

fn delta_pct(base: u64, candidate: u64) -> f64 {
    if base == 0 {
        return 0.0;
    }
    ((candidate as f64 - base as f64) / base as f64) * 100.0
}

fn color_delta(pct: f64) -> String {
    let s = format!("{:>+7.1}%", pct);
    if pct > 10.0 {
        s.red().bold().to_string()
    } else if pct > 2.0 {
        s.yellow().to_string()
    } else if pct < -2.0 {
        s.green().to_string()
    } else {
        s.dimmed().to_string()
    }
}
