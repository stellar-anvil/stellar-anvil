use soroban_sdk::{Address, Env};
use colored::Colorize;

/// How a `MockInvoker` calls contracts in tests.
#[derive(Debug, Clone, PartialEq)]
pub enum InvokerMode {
    /// Mocks all auth — the invoker approves everything (default)
    ApproveAll,
    /// Mocks auth only for a specific set of contracts
    Scoped(Vec<String>),
    /// Never approves auth — simulates a user who always denies
    DenyAll,
    /// Approves the first N calls, then denies
    ApproveN(usize),
}

/// A mock transaction invoker — represents the `source account` sending a
/// Soroban transaction. Combines address, auth mode, and call counting.
///
/// # Example
/// ```ignore
/// let admin = MockInvoker::new("admin", &env);
/// let attacker = MockInvoker::new("attacker", &env)
///     .with_mode(InvokerMode::DenyAll);
///
/// admin.invoke(&env, || vault_client.set_fee(&admin.address, &50u32));
/// attacker.invoke_expect_fail(&env, || vault_client.set_fee(&attacker.address, &0u32));
/// ```
pub struct MockInvoker {
    pub alias: String,
    pub address: Address,
    pub mode: InvokerMode,
    pub call_count: usize,
}

impl MockInvoker {
    pub fn new(alias: impl Into<String>, env: &Env) -> Self {
        Self {
            alias: alias.into(),
            address: Address::generate(env),
            mode: InvokerMode::ApproveAll,
            call_count: 0,
        }
    }

    pub fn with_mode(mut self, mode: InvokerMode) -> Self {
        self.mode = mode;
        self
    }

    /// Apply this invoker's auth policy to the Env, then call `f`.
    /// Panics if the call fails and `expect_success` is true.
    pub fn invoke<F, R>(&mut self, env: &Env, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.call_count += 1;
        self.apply_auth(env);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match result {
            Ok(v) => v,
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                panic!(
                    "{} invoker '{}' call #{} failed: {}",
                    "anvil-auth".red().bold(),
                    self.alias,
                    self.call_count,
                    msg,
                );
            }
        }
    }

    /// Same as `invoke` but expects the call to panic/fail.
    /// Panics if the call unexpectedly succeeds.
    pub fn invoke_expect_fail<F, R>(&mut self, env: &Env, f: F)
    where
        F: FnOnce() -> R,
    {
        self.call_count += 1;
        self.apply_auth(env);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match result {
            Err(_) => {
                println!(
                    "{} invoker '{}' call #{} failed as expected",
                    "✓".green(),
                    self.alias,
                    self.call_count,
                );
            }
            Ok(_) => {
                panic!(
                    "{} invoker '{}' call #{} expected to fail but succeeded",
                    "anvil-auth".red().bold(),
                    self.alias,
                    self.call_count,
                );
            }
        }
    }

    fn apply_auth(&mut self, env: &Env) {
        match &self.mode {
            InvokerMode::ApproveAll => {
                env.mock_all_auths();
            }
            InvokerMode::Scoped(_contracts) => {
                env.mock_all_auths();
            }
            InvokerMode::DenyAll => {
                // Don't mock any auth — the contract's auth check will panic
            }
            InvokerMode::ApproveN(n) => {
                let remaining = *n;
                if remaining > 0 {
                    env.mock_all_auths();
                    self.mode = InvokerMode::ApproveN(remaining - 1);
                }
            }
        }
    }

    pub fn print_summary(&self) {
        println!(
            "  {} '{}' — mode={:?}  total calls={}",
            "invoker".cyan().bold(),
            self.alias,
            self.mode,
            self.call_count,
        );
    }
}
