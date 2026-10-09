use crate::report::ProfileReport;
use colored::Colorize;
use serde::{Deserialize, Serialize};

/// Side-by-side comparison of two `ProfileReport`s.
///
/// # Example
/// ```ignore
/// let comparison = ProfileComparison::new("v1 vs v2", &baseline, &candidate);
/// comparison.print();
/// comparison.assert_no_regression(0.10);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallDelta {
    pub label: String,
    pub baseline_cpu: u64,
    pub candidate_cpu: u64,
    pub cpu_delta_pct: f64,
    pub baseline_mem: u64,
    pub candidate_mem: u64,
    pub mem_delta_pct: f64,
}

impl CallDelta {
    fn pct(baseline: u64, candidate: u64) -> f64 {
        if baseline == 0 {
            return 0.0;
        }
        ((candidate as f64 - baseline as f64) / baseline as f64) * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct ProfileComparison {
    pub title: String,
    pub deltas: Vec<CallDelta>,
}

impl ProfileComparison {
    pub fn new(title: impl Into<String>, baseline: &ProfileReport, candidate: &ProfileReport) -> Self {
        let mut deltas = Vec::new();

        for base in &baseline.results {
            if let Some(cand) = candidate.results.iter().find(|r| r.label == base.label) {
                deltas.push(CallDelta {
                    label: base.label.clone(),
                    baseline_cpu: base.cpu_instructions,
                    candidate_cpu: cand.cpu_instructions,
                    cpu_delta_pct: CallDelta::pct(base.cpu_instructions, cand.cpu_instructions),
                    baseline_mem: base.memory_bytes,
                    candidate_mem: cand.memory_bytes,
                    mem_delta_pct: CallDelta::pct(base.memory_bytes, cand.memory_bytes),
                });
            }
        }

        Self {
            title: title.into(),
            deltas,
        }
    }

    pub fn print(&self) {
        println!("\n{}", "═".repeat(90).cyan());
        println!(
            "  {} {}",
            "Stellar Anvil — Regression Report:".cyan().bold(),
            self.title.white().bold()
        );
        println!("{}", "═".repeat(90).cyan());
        println!(
            "  {:<42} {:>14} {:>14} {:>8}  {:>12} {:>12} {:>8}",
            "Invocation".bold(),
            "Base CPU".bold(),
            "New CPU".bold(),
            "Δ CPU%".bold(),
            "Base Mem".bold(),
            "New Mem".bold(),
            "Δ Mem%".bold(),
        );
        println!("{}", "─".repeat(90).dimmed());

        for d in &self.deltas {
            let cpu_flag = Self::delta_color(d.cpu_delta_pct);
            let mem_flag = Self::delta_color(d.mem_delta_pct);

            println!(
                "  {:<42} {:>14} {:>14} {}  {:>12} {:>12} {}",
                d.label,
                d.baseline_cpu.to_string().dimmed(),
                d.candidate_cpu.to_string().cyan(),
                cpu_flag,
                d.baseline_mem.to_string().dimmed(),
                d.candidate_mem.to_string().yellow(),
                mem_flag,
            );
        }
        println!("{}\n", "═".repeat(90).cyan());
    }

    fn delta_color(pct: f64) -> String {
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

    pub fn assert_no_regression(&self, max_pct: f64) {
        let regressions: Vec<&CallDelta> = self
            .deltas
            .iter()
            .filter(|d| d.cpu_delta_pct > max_pct || d.mem_delta_pct > max_pct)
            .collect();

        if regressions.is_empty() {
            println!(
                "{} No regressions detected (threshold: +{:.1}%)",
                "✓".green().bold(),
                max_pct
            );
            return;
        }

        eprintln!(
            "\n{} Regressions detected (threshold: +{:.1}%):",
            "✗".red().bold(),
            max_pct
        );
        for r in &regressions {
            eprintln!(
                "  {} — CPU: {:+.1}%  Mem: {:+.1}%",
                r.label.yellow(),
                r.cpu_delta_pct,
                r.mem_delta_pct,
            );
        }
        panic!("anvil: regression threshold exceeded");
    }

    pub fn export_json(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        #[derive(Serialize)]
        struct Export<'a> {
            title: &'a str,
            deltas: &'a Vec<CallDelta>,
        }
        let data = Export {
            title: &self.title,
            deltas: &self.deltas,
        };
        std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }
}
