use soroban_sdk::Env;
use serde::{Deserialize, Serialize};
use colored::Colorize;

/// A single recorded invocation measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub label: String,
    pub cpu_instructions: u64,
    pub memory_bytes: u64,
    pub expected_success: bool,
    pub succeeded: bool,
}

impl ProfileResult {
    /// Rough fee estimate in stroops. ~25 stroops per 10k CPU insns + ~1 per KB memory.
    pub fn estimated_fee_stroops(&self) -> u64 {
        let cpu_fee = (self.cpu_instructions / 10_000) * 25;
        let mem_fee = (self.memory_bytes / 1_024) * 1;
        cpu_fee + mem_fee
    }
}

/// Wraps a `soroban_sdk::Env` to measure CPU + memory cost of any closure.
///
/// # Example
/// ```ignore
/// let profiler = Profiler::new(&env);
/// let result = profiler.measure("increment()", || client.increment());
/// println!("CPU: {}  MEM: {}", result.cpu_instructions, result.memory_bytes);
/// ```
pub struct Profiler<'a> {
    env: &'a Env,
    results: Vec<ProfileResult>,
    verbose: bool,
}

impl<'a> Profiler<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self {
            env,
            results: Vec::new(),
            verbose: false,
        }
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Measure the resource cost of calling `f`.
    /// Resets the budget before and captures it after.
    pub fn measure<F, R>(&mut self, label: impl Into<String>, f: F) -> ProfileResult
    where
        F: FnOnce() -> R,
    {
        let label = label.into();

        self.env.budget().reset_default();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = f();
        }));

        let cpu = self.env.budget().cpu_instruction_cost();
        let mem = self.env.budget().memory_bytes_cost();
        let succeeded = result.is_ok();

        let pr = ProfileResult {
            label: label.clone(),
            cpu_instructions: cpu,
            memory_bytes: mem,
            expected_success: true,
            succeeded,
        };

        if self.verbose {
            let status = if succeeded {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            println!(
                "  {} {:40}  cpu={:>12}  mem={:>10} B  fee≈{} stroops",
                status,
                label,
                cpu.to_string().cyan(),
                mem.to_string().yellow(),
                pr.estimated_fee_stroops().to_string().magenta(),
            );
        }

        self.results.push(pr.clone());
        pr
    }

    /// Measure an invocation that is expected to fail.
    pub fn measure_err<F, R>(&mut self, label: impl Into<String>, f: F) -> ProfileResult
    where
        F: FnOnce() -> R,
    {
        let label = label.into();
        self.env.budget().reset_default();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = f();
        }));

        let cpu = self.env.budget().cpu_instruction_cost();
        let mem = self.env.budget().memory_bytes_cost();

        let pr = ProfileResult {
            label: label.clone(),
            cpu_instructions: cpu,
            memory_bytes: mem,
            expected_success: false,
            succeeded: result.is_ok(),
        };

        if self.verbose {
            println!(
                "  {} {:40}  cpu={:>12}  mem={:>10} B  (expected error)",
                "~".yellow(),
                label,
                cpu.to_string().cyan(),
                mem.to_string().yellow(),
            );
        }

        self.results.push(pr.clone());
        pr
    }

    pub fn results(self) -> Vec<ProfileResult> {
        self.results
    }

    pub fn into_report(self, title: impl Into<String>) -> crate::report::ProfileReport {
        crate::report::ProfileReport::new(title, self.results)
    }
}
