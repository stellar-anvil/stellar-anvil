use crate::call_trace::CallTrace;
use crate::registry::ContractRegistry;
use colored::Colorize;
use soroban_sdk::Env;

/// The result of executing a single scenario step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub label: String,
    pub succeeded: bool,
    pub cpu_instructions: u64,
    pub memory_bytes: u64,
    pub trace: Option<CallTrace>,
    pub error: Option<String>,
}

impl StepResult {
    pub fn success(label: impl Into<String>, cpu: u64, mem: u64) -> Self {
        Self {
            label: label.into(),
            succeeded: true,
            cpu_instructions: cpu,
            memory_bytes: mem,
            trace: None,
            error: None,
        }
    }

    pub fn failure(label: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            succeeded: false,
            cpu_instructions: 0,
            memory_bytes: 0,
            trace: None,
            error: Some(error.into()),
        }
    }
}

/// A single executable step in a `Scenario`.
pub struct ScenarioStep {
    pub label: String,
    pub expected_success: bool,
    pub action: Box<dyn FnOnce(&Env, &ContractRegistry) + 'static>,
}

impl ScenarioStep {
    pub fn new(
        label: impl Into<String>,
        action: impl FnOnce(&Env, &ContractRegistry) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            expected_success: true,
            action: Box::new(action),
        }
    }

    pub fn expect_failure(mut self) -> Self {
        self.expected_success = false;
        self
    }
}

/// A named sequence of cross-contract interaction steps.
///
/// # Example
/// ```ignore
/// let mut scenario = Scenario::new("DeFi round-trip", &env, registry);
///
/// scenario.step("alice deposits 1000", |env, reg| {
///     let vault = VaultClient::new(env, reg.get("vault").unwrap());
///     vault.deposit(&alice.address(), &1000_i128);
/// });
///
/// let results = scenario.run();
/// scenario.print_summary(&results);
/// ```
pub struct Scenario {
    pub name: String,
    env: Env,
    registry: ContractRegistry,
    steps: Vec<ScenarioStep>,
    stop_on_failure: bool,
    verbose: bool,
}

impl Scenario {
    pub fn new(name: impl Into<String>, env: &Env, registry: ContractRegistry) -> Self {
        Self {
            name: name.into(),
            env: env.clone(),
            registry,
            steps: Vec::new(),
            stop_on_failure: true,
            verbose: false,
        }
    }

    pub fn continue_on_failure(mut self) -> Self {
        self.stop_on_failure = false;
        self
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    pub fn step(
        &mut self,
        label: impl Into<String>,
        action: impl FnOnce(&Env, &ContractRegistry) + 'static,
    ) -> &mut Self {
        self.steps.push(ScenarioStep::new(label, action));
        self
    }

    pub fn step_fails(
        &mut self,
        label: impl Into<String>,
        action: impl FnOnce(&Env, &ContractRegistry) + 'static,
    ) -> &mut Self {
        self.steps
            .push(ScenarioStep::new(label, action).expect_failure());
        self
    }

    pub fn run(mut self) -> Vec<StepResult> {
        println!(
            "\n{} {}",
            "▶ Scenario:".cyan().bold(),
            self.name.white().bold()
        );
        println!("{}", "─".repeat(60).dimmed());

        let mut results = Vec::new();

        for step in self.steps.drain(..) {
            let label = step.label.clone();
            let expected_success = step.expected_success;

            self.env.budget().reset_default();

            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (step.action)(&self.env, &self.registry);
            }));

            let cpu = self.env.budget().cpu_instruction_cost();
            let mem = self.env.budget().memory_bytes_cost();

            match outcome {
                Ok(_) => {
                    let ok = if expected_success { "✓" } else { "!" };
                    let color_label = if expected_success {
                        label.green().to_string()
                    } else {
                        label.yellow().to_string()
                    };
                    if self.verbose || !expected_success {
                        println!(
                            "  {}  {}  [cpu={} mem={}]",
                            ok.green(),
                            color_label,
                            cpu.to_string().cyan(),
                            mem.to_string().yellow(),
                        );
                    } else {
                        println!("  {}  {}", "✓".green(), label);
                    }
                    results.push(StepResult::success(&label, cpu, mem));
                }
                Err(e) => {
                    let err_msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "unknown panic".to_string()
                    };

                    if expected_success {
                        println!(
                            "  {}  {}  — {}",
                            "✗".red().bold(),
                            label.red(),
                            err_msg.dimmed(),
                        );
                    } else {
                        println!(
                            "  {}  {} (expected failure)",
                            "~".yellow(),
                            label.yellow(),
                        );
                    }
                    results.push(StepResult::failure(&label, &err_msg));

                    if self.stop_on_failure && expected_success {
                        println!(
                            "\n{} scenario aborted at step '{}'",
                            "anvil".red().bold(),
                            label
                        );
                        break;
                    }
                }
            }
        }

        results
    }

    pub fn print_summary(name: &str, results: &[StepResult]) {
        println!("\n{}", "─".repeat(60).dimmed());
        let passed = results.iter().filter(|r| r.succeeded).count();
        let failed = results.len() - passed;
        let status = if failed == 0 {
            format!("{} PASSED", "✓".green().bold())
        } else {
            format!("{} {} FAILED", "✗".red().bold(), failed)
        };
        println!(
            "  Scenario '{}' — {}/{} steps {}",
            name.white().bold(),
            passed,
            results.len(),
            status,
        );

        let total_cpu: u64 = results.iter().map(|r| r.cpu_instructions).sum();
        let total_mem: u64 = results.iter().map(|r| r.memory_bytes).sum();
        println!(
            "  Total CPU: {}  Total Mem: {}",
            total_cpu.to_string().cyan(),
            total_mem.to_string().yellow(),
        );
        println!();

        if failed > 0 {
            let failed_labels: Vec<&str> =
                results.iter().filter(|r| !r.succeeded).map(|r| r.label.as_str()).collect();
            panic!(
                "anvil: scenario '{}' failed — steps: [{}]",
                name,
                failed_labels.join(", ")
            );
        }
    }
}
