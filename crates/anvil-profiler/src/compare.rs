use crate::report::ProfileReport;
use colored::Colorize;
use serde::{Deserialize, Serialize};

// ── shared formatting ────────────────────────────────────────────────────────

const LINE_W: usize = 78;

fn fmt_num(n: u64) -> String {
    anvil_core::output::fmt_num(n)
}

fn delta_color(pct: f64) -> String {
    let s = format!("{:>+8.1}%", pct);
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

fn divider() {
    println!("{}", "─".repeat(LINE_W).dimmed());
}

fn heavy(label: &str) {
    if label.is_empty() {
        println!("{}", "═".repeat(LINE_W).dimmed());
    } else {
        let prefix = format!("═══ {} ", label);
        let fill   = LINE_W.saturating_sub(prefix.len());
        println!("{}{}", prefix.cyan().bold(), "═".repeat(fill).dimmed());
    }
}

// ─────────────────────────────────────────────────────────────────────────────

/// Side-by-side comparison of two [`ProfileReport`]s.
///
/// # Example
/// ```ignore
/// let comparison = ProfileComparison::new("v1 vs v2", &baseline, &candidate);
/// comparison.print();
/// comparison.assert_no_regression(10.0);
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
        if baseline == 0 { return 0.0; }
        ((candidate as f64 - baseline as f64) / baseline as f64) * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct ProfileComparison {
    pub title: String,
    pub deltas: Vec<CallDelta>,
}

impl ProfileComparison {
    pub fn new(
        title: impl Into<String>,
        baseline: &ProfileReport,
        candidate: &ProfileReport,
    ) -> Self {
        let mut deltas = Vec::new();
        for base in &baseline.results {
            if let Some(cand) = candidate.results.iter().find(|r| r.label == base.label) {
                deltas.push(CallDelta {
                    label:         base.label.clone(),
                    baseline_cpu:  base.cpu_instructions,
                    candidate_cpu: cand.cpu_instructions,
                    cpu_delta_pct: CallDelta::pct(base.cpu_instructions, cand.cpu_instructions),
                    baseline_mem:  base.memory_bytes,
                    candidate_mem: cand.memory_bytes,
                    mem_delta_pct: CallDelta::pct(base.memory_bytes, cand.memory_bytes),
                });
            }
        }
        Self { title: title.into(), deltas }
    }

    pub fn print(&self) {
        // Dynamic label column, clamped so total row ≤ 78 chars.
        // Fixed cols: 8 + 8 + 9 + 8 + 8 + 9 = 50, spaces ~8 = 58.
        // label_w ≤ 78 - 58 - 2 = 18; floor at 16.
        let label_w: usize = self.deltas
            .iter()
            .map(|d| d.label.len())
            .max()
            .unwrap_or(16)
            .clamp(16, 22);

        let col     = 8usize;
        let delta_w = 9usize;

        heavy(&format!("Regression Report: {}", self.title));
        println!(
            "  {:lw$}  {:>cw$}  {:>cw$}  {:>dw$}   {:>cw$}  {:>cw$}  {:>dw$}",
            "Invocation".bold(),
            "Base CPU".bold(),
            "New CPU".bold(),
            "Δ CPU%".bold(),
            "Base Mem".bold(),
            "New Mem".bold(),
            "Δ Mem%".bold(),
            lw = label_w,
            cw = col,
            dw = delta_w,
        );
        divider();

        for d in &self.deltas {
            let cpu_flag = delta_color(d.cpu_delta_pct);
            let mem_flag = delta_color(d.mem_delta_pct);

            let label = if d.label.len() > label_w {
                format!("{}…", &d.label[..label_w - 1])
            } else {
                d.label.clone()
            };

            println!(
                "  {:lw$}  {:>cw$}  {:>cw$}  {}   {:>cw$}  {:>cw$}  {}",
                label,
                fmt_num(d.baseline_cpu).dimmed().to_string(),
                fmt_num(d.candidate_cpu).cyan().to_string(),
                cpu_flag,
                fmt_num(d.baseline_mem).dimmed().to_string(),
                fmt_num(d.candidate_mem).yellow().to_string(),
                mem_flag,
                lw = label_w,
                cw = col,
            );
        }

        heavy("");

        // Color legend
        println!(
            "  {}  {}  {}  {}",
            "■ cyan = new value".cyan().dimmed(),
            "■ green = improvement".green().dimmed(),
            "■ yellow = minor change".yellow().dimmed(),
            "■ red = regression".red().dimmed(),
        );
        println!();
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
            title:  &'a str,
            deltas: &'a Vec<CallDelta>,
        }
        let data = Export { title: &self.title, deltas: &self.deltas };
        std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }
}
