use crate::report::ProfileReport;
use colored::Colorize;
use serde::{Deserialize, Serialize};

/// Budget limits that any profiled invocation must stay within.
///
/// # Example
/// ```ignore
/// let threshold = BudgetThreshold::default()
///     .max_cpu(500_000)
///     .max_memory(100_000);
///
/// report.assert_within(&threshold);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetThreshold {
    pub max_cpu_per_call: Option<u64>,
    pub max_memory_per_call: Option<u64>,
    pub max_fee_per_call: Option<u64>,
    pub max_total_cpu: Option<u64>,
    pub max_total_memory: Option<u64>,
}

impl Default for BudgetThreshold {
    fn default() -> Self {
        Self {
            max_cpu_per_call: None,
            max_memory_per_call: None,
            max_fee_per_call: None,
            max_total_cpu: None,
            max_total_memory: None,
        }
    }
}

impl BudgetThreshold {
    pub fn max_cpu(mut self, v: u64) -> Self {
        self.max_cpu_per_call = Some(v);
        self
    }

    pub fn max_memory(mut self, v: u64) -> Self {
        self.max_memory_per_call = Some(v);
        self
    }

    pub fn max_fee(mut self, v: u64) -> Self {
        self.max_fee_per_call = Some(v);
        self
    }

    pub fn max_total_cpu(mut self, v: u64) -> Self {
        self.max_total_cpu = Some(v);
        self
    }

    pub fn max_total_memory(mut self, v: u64) -> Self {
        self.max_total_memory = Some(v);
        self
    }

    pub fn check(&self, report: &ProfileReport) -> ThresholdResult {
        let mut failures: Vec<String> = Vec::new();

        for r in &report.results {
            if let Some(max_cpu) = self.max_cpu_per_call {
                if r.cpu_instructions > max_cpu {
                    failures.push(format!(
                        "  CPU limit exceeded — '{}': {} > {} insns",
                        r.label, r.cpu_instructions, max_cpu
                    ));
                }
            }
            if let Some(max_mem) = self.max_memory_per_call {
                if r.memory_bytes > max_mem {
                    failures.push(format!(
                        "  Memory limit exceeded — '{}': {} > {} bytes",
                        r.label, r.memory_bytes, max_mem
                    ));
                }
            }
            if let Some(max_fee) = self.max_fee_per_call {
                let fee = r.estimated_fee_stroops();
                if fee > max_fee {
                    failures.push(format!(
                        "  Fee limit exceeded — '{}': {} > {} stroops",
                        r.label, fee, max_fee
                    ));
                }
            }
        }

        if let Some(max_total_cpu) = self.max_total_cpu {
            let total = report.total_cpu();
            if total > max_total_cpu {
                failures.push(format!(
                    "  Total CPU limit exceeded: {} > {} insns",
                    total, max_total_cpu
                ));
            }
        }
        if let Some(max_total_mem) = self.max_total_memory {
            let total = report.total_memory();
            if total > max_total_mem {
                failures.push(format!(
                    "  Total memory limit exceeded: {} > {} bytes",
                    total, max_total_mem
                ));
            }
        }

        ThresholdResult {
            passed: failures.is_empty(),
            failures,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThresholdResult {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl ThresholdResult {
    pub fn format_failures(&self) -> String {
        if self.passed {
            return format!("{} All thresholds passed", "✓".green());
        }
        let mut out = format!("{} Budget thresholds exceeded:\n", "✗".red().bold());
        for f in &self.failures {
            out.push_str(&format!("{}\n", f.red()));
        }
        out
    }
}
