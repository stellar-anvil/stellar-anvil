use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;
use std::time::Instant;
use crate::output::{self, fmt_num, color_delta};

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

pub async fn run(args: CompareArgs, _verbose: bool, quiet: bool) -> Result<()> {
    let start = Instant::now();

    if !quiet {
        output::section("Comparing Profile Reports");
        output::value("baseline",  &args.baseline.display().to_string());
        output::value("candidate", &args.candidate.display().to_string());
        output::value("threshold", &format!("{}%", args.threshold));
    }

    let baseline_raw  = std::fs::read_to_string(&args.baseline)
        .map_err(|e| anyhow::anyhow!("cannot read baseline '{}': {}", args.baseline.display(), e))?;
    let candidate_raw = std::fs::read_to_string(&args.candidate)
        .map_err(|e| anyhow::anyhow!("cannot read candidate '{}': {}", args.candidate.display(), e))?;

    let baseline:  Vec<ProfileEntry> = serde_json::from_str(&baseline_raw)
        .map_err(|e| anyhow::anyhow!("invalid baseline JSON: {}", e))?;
    let candidate: Vec<ProfileEntry> = serde_json::from_str(&candidate_raw)
        .map_err(|e| anyhow::anyhow!("invalid candidate JSON: {}", e))?;

    // Dynamic label column width, capped to keep total ≤ 78 chars.
    // Remaining width after label: 8+1 + 8+1 + 9+2 + 8+1 + 8+1 + 9 = 56 + margins
    // We target: label_w + 56 ≤ 78 → label_w ≤ 22.  Floor at 20.
    let label_w = baseline
        .iter()
        .chain(candidate.iter())
        .map(|e| e.label.len())
        .max()
        .unwrap_or(20)
        .clamp(20, 26);

    let col = 8usize;  // numeric column width
    let delta_w = 9usize; // "+1,234.5%" max

    if !quiet {
        println!();
        output::heavy_divider();
        println!(
            "  {:lw$}  {:>cw$}  {:>cw$}  {:>dw$}   {:>cw$}  {:>cw$}  {:>dw$}",
            "Invocation".bold(),
            "Base CPU".bold(),
            "New CPU".bold(),
            "ΔCPU%".bold(),
            "Base Mem".bold(),
            "New Mem".bold(),
            "ΔMem%".bold(),
            lw = label_w,
            cw = col,
            dw = delta_w,
        );
        output::divider();
    }

    let mut regressions  = 0usize;
    let mut improvements = 0usize;
    let mut md_rows      = Vec::new();

    for base in &baseline {
        let cand = candidate.iter().find(|c| c.label == base.label);
        let Some(cand) = cand else { continue };

        let cpu_delta = delta_pct(base.cpu, cand.cpu);
        let mem_delta = delta_pct(base.mem, cand.mem);

        let cpu_flag = color_delta(cpu_delta);
        let mem_flag = color_delta(mem_delta);

        // Truncate label to column width
        let label = if base.label.len() > label_w {
            format!("{}…", &base.label[..label_w - 1])
        } else {
            base.label.clone()
        };

        if !quiet {
            println!(
                "  {:lw$}  {:>cw$}  {:>cw$}  {}   {:>cw$}  {:>cw$}  {}",
                label,
                fmt_num(base.cpu).dimmed().to_string(),
                fmt_num(cand.cpu).cyan().to_string(),
                cpu_flag,
                fmt_num(base.mem).dimmed().to_string(),
                fmt_num(cand.mem).yellow().to_string(),
                mem_flag,
                lw = label_w,
                cw = col,
            );
        }

        if cpu_delta > args.threshold || mem_delta > args.threshold {
            regressions += 1;
        } else if cpu_delta < -2.0 || mem_delta < -2.0 {
            improvements += 1;
        }

        md_rows.push(format!(
            "| `{}` | {} | {} | {:+.1}% | {} | {} | {:+.1}% |",
            base.label,
            fmt_num(base.cpu),
            fmt_num(cand.cpu),
            cpu_delta,
            fmt_num(base.mem),
            fmt_num(cand.mem),
            mem_delta,
        ));
    }

    let elapsed = start.elapsed();

    if !quiet {
        output::divider();
        output::heavy_divider();

        println!();
        output::value("regressions",  &regressions.to_string());
        output::value("improvements", &improvements.to_string());
        output::value("elapsed",      &format!("{:.2}s", elapsed.as_secs_f64()));

        // Color legend
        output::legend(&[
            ("■", "cyan = new value"),
            ("■", "green = improvement"),
            ("■", "yellow = minor change"),
            ("■", "red = regression"),
        ]);
    }

    if let Some(md_path) = &args.markdown {
        let mut md = String::from("## Stellar Anvil — Regression Report\n\n");
        md.push_str("| Invocation | Base CPU | New CPU | ΔCPU% | Base Mem | New Mem | ΔMem% |\n");
        md.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
        for row in &md_rows {
            md.push_str(row);
            md.push('\n');
        }
        std::fs::write(md_path, md)
            .map_err(|e| anyhow::anyhow!("cannot write markdown to '{}': {}", md_path.display(), e))?;
        if !quiet {
            output::ok(&format!("Markdown report → {}", md_path.display()));
        }
    }

    if regressions > 0 {
        anyhow::bail!(
            "{} regression(s) exceeded {}% threshold",
            regressions,
            args.threshold
        );
    }

    if !quiet {
        output::ok(&format!("No regressions above {}% threshold", args.threshold));
    }
    Ok(())
}

fn delta_pct(base: u64, candidate: u64) -> f64 {
    if base == 0 {
        return 0.0;
    }
    ((candidate as f64 - base as f64) / base as f64) * 100.0
}
