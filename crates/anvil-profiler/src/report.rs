use crate::profile::ProfileResult;
use crate::thresholds::{BudgetThreshold, ThresholdResult};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A complete profiling report for a test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileReport {
    pub title: String,
    pub results: Vec<ProfileResult>,
}

impl ProfileReport {
    pub fn new(title: impl Into<String>, results: Vec<ProfileResult>) -> Self {
        Self {
            title: title.into(),
            results,
        }
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
        if self.results.is_empty() {
            return 0;
        }
        self.total_cpu() / self.results.len() as u64
    }

    pub fn print(&self) {
        println!("\n{}", "═".repeat(78).cyan());
        println!(
            "  {} {}",
            "Stellar Anvil — Profile Report:".cyan().bold(),
            self.title.white().bold()
        );
        println!("{}", "═".repeat(78).cyan());

        println!(
            "  {:<44} {:>14} {:>12} {:>10}",
            "Invocation".bold(),
            "CPU insns".bold(),
            "Mem bytes".bold(),
            "Fee (stroops)".bold(),
        );
        println!("{}", "─".repeat(78).dimmed());

        for r in &self.results {
            let status = if r.succeeded {
                "✓".green().to_string()
            } else if !r.expected_success {
                "~".yellow().to_string()
            } else {
                "✗".red().to_string()
            };

            println!(
                "  {} {:<42} {:>14} {:>12} {:>14}",
                status,
                r.label,
                r.cpu_instructions.to_string().cyan(),
                r.memory_bytes.to_string().yellow(),
                r.estimated_fee_stroops().to_string().magenta(),
            );
        }

        println!("{}", "─".repeat(78).dimmed());
        println!(
            "  {:<44} {:>14} {:>12} {:>14}",
            "TOTAL".bold(),
            self.total_cpu().to_string().cyan().bold(),
            self.total_memory().to_string().yellow().bold(),
            self.total_estimated_fee().to_string().magenta().bold(),
        );
        println!(
            "  {:<44} {:>14}",
            "AVG CPU / call".dimmed(),
            self.avg_cpu().to_string().cyan(),
        );

        if let Some(max) = self.max_cpu_result() {
            println!(
                "  {:<44} {:>14}  ({})",
                "Most expensive".dimmed(),
                max.cpu_instructions.to_string().red(),
                max.label.dimmed(),
            );
        }
        println!("{}\n", "═".repeat(78).cyan());
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
                r.cpu_instructions,
                r.memory_bytes,
                r.estimated_fee_stroops(),
            ));
        }
        md.push_str(&format!(
            "| **TOTAL** | **{}** | **{}** | **{}** |\n",
            self.total_cpu(),
            self.total_memory(),
            self.total_estimated_fee(),
        ));
        std::fs::write(path, md)?;
        Ok(())
    }
}
