use crate::profile::ProfileResult;
use crate::thresholds::{BudgetThreshold, ThresholdResult};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── shared formatting ────────────────────────────────────────────────────────

const LINE_W: usize = 78;

fn fmt_num(n: u64) -> String {
    anvil_core::output::fmt_num(n)
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

/// A complete profiling report for a test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileReport {
    pub title: String,
    pub results: Vec<ProfileResult>,
}

impl ProfileReport {
    pub fn new(title: impl Into<String>, results: Vec<ProfileResult>) -> Self {
        Self { title: title.into(), results }
    }

    pub fn total_cpu(&self) -> u64 {
        self.results.iter().map(|r| r.cpu_instructions).sum()
    }

    pub fn total_memory(&self) -> u64 {
        self.results.iter().map(|r| r.memory_bytes).sum()
    }

    pub fn total_estimated_fee(&self) -> u64 {
        self.results.iter().map(|r| r.estimated_fee_stroops()).sum()
    }

    pub fn max_cpu_result(&self) -> Option<&ProfileResult> {
        self.results.iter().max_by_key(|r| r.cpu_instructions)
    }

    pub fn min_cpu_result(&self) -> Option<&ProfileResult> {
        self.results.iter().min_by_key(|r| r.cpu_instructions)
    }

    pub fn avg_cpu(&self) -> u64 {
        if self.results.is_empty() { return 0; }
        self.total_cpu() / self.results.len() as u64
    }

    pub fn print(&self) {
        // Dynamic label column width, clamped so total row ≤ 78 chars.
        // Fixed numeric widths: 14 + 12 + 14 = 40, plus 3 spaces = 43.
        // label_w ≤ 78 - 43 - 2 (indent) = 33; floor at 24.
        let label_w: usize = self.results
            .iter()
            .map(|r| r.label.len())
            .max()
            .unwrap_or(24)
            .clamp(24, 33);

        let cpu_w = 14usize;
        let mem_w = 12usize;
        let fee_w = 14usize;

        heavy(&format!("Profile Report: {}", self.title));
        println!(
            "  {:lw$} {:>cw$} {:>mw$} {:>fw$}",
            "Invocation".bold(),
            "CPU insns".bold(),
            "Mem bytes".bold(),
            "Fee (stroops)".bold(),
            lw = label_w,
            cw = cpu_w,
            mw = mem_w,
            fw = fee_w,
        );
        divider();

        for r in &self.results {
            let status = if r.succeeded {
                "✓".green().to_string()
            } else if !r.expected_success {
                "~".yellow().to_string()
            } else {
                "✗".red().to_string()
            };

            // Truncate label if needed to prevent column overflow.
            let label = if r.label.len() > label_w {
                format!("{}…", &r.label[..label_w - 1])
            } else {
                r.label.clone()
            };

            println!(
                "  {} {:lw$} {:>cw$} {:>mw$} {:>fw$}",
                status,
                label,
                fmt_num(r.cpu_instructions).cyan().to_string(),
                fmt_num(r.memory_bytes).yellow().to_string(),
                fmt_num(r.estimated_fee_stroops()).magenta().to_string(),
                lw = label_w,
                cw = cpu_w,
                mw = mem_w,
                fw = fee_w,
            );
        }

        divider();
        println!(
            "  {:lw$} {:>cw$} {:>mw$} {:>fw$}",
            "TOTAL".bold(),
            fmt_num(self.total_cpu()).cyan().bold().to_string(),
            fmt_num(self.total_memory()).yellow().bold().to_string(),
            fmt_num(self.total_estimated_fee()).magenta().bold().to_string(),
            lw = label_w + 2, // +2 for the status icon indent
            cw = cpu_w,
            mw = mem_w,
            fw = fee_w,
        );
        println!(
            "  {:lw$} {:>cw$}",
            "AVG CPU / call".dimmed(),
            fmt_num(self.avg_cpu()).cyan().to_string(),
            lw = label_w + 2,
            cw = cpu_w,
        );

        if let Some(max) = self.max_cpu_result() {
            let label = if max.label.len() > label_w {
                format!("{}…", &max.label[..label_w - 1])
            } else {
                max.label.clone()
            };
            println!(
                "  {:lw$} {:>cw$}  ({})",
                "Most expensive".dimmed(),
                fmt_num(max.cpu_instructions).red().to_string(),
                label.dimmed(),
                lw = label_w + 2,
                cw = cpu_w,
            );
        }

        heavy("");

        // Color legend
        println!(
            "  {}  {}  {}",
            "■ cyan = CPU".cyan().dimmed(),
            "■ yellow = memory".yellow().dimmed(),
            "■ magenta = fee estimate".magenta().dimmed(),
        );
        println!();
    }

    pub fn assert_within(&self, threshold: &BudgetThreshold) -> ThresholdResult {
        let result = threshold.check(self);
        if !result.passed {
            eprintln!("{}", result.format_failures());
            panic!(
                "{} profile threshold exceeded — see above",
                "anvil".red().bold()
            );
        }
        result
    }

    pub fn export_json(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn export_markdown(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let mut md = format!("## Stellar Anvil — Profile: {}\n\n", self.title);
        md.push_str("| Invocation | CPU insns | Mem bytes | Fee (stroops) |\n");
        md.push_str("|---|---:|---:|---:|\n");
        for r in &self.results {
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                r.label,
                fmt_num(r.cpu_instructions),
                fmt_num(r.memory_bytes),
                fmt_num(r.estimated_fee_stroops()),
            ));
        }
        md.push_str(&format!(
            "| **TOTAL** | **{}** | **{}** | **{}** |\n",
            fmt_num(self.total_cpu()),
            fmt_num(self.total_memory()),
            fmt_num(self.total_estimated_fee()),
        ));
        std::fs::write(path, md)?;
        Ok(())
    }
}
