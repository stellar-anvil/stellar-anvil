use soroban_sdk::Env;
use colored::Colorize;

/// A violation of an auth policy.
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub rule: String,
    pub details: String,
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Policy '{}' violated: {}", self.rule, self.details)
    }
}

/// Declarative auth policy for asserting what a contract did/didn't authorize.
///
/// # Example
/// ```ignore
/// env.mock_all_auths();
/// vault_client.deposit(&alice.address(), &1000_i128);
///
/// AuthPolicy::new(&env)
///     .must_have_authorized(&alice.address())
///     .must_not_have_authorized(&bob.address())
///     .assert();
/// ```
pub struct AuthPolicy<'a> {
    env: &'a Env,
    required: Vec<soroban_sdk::Address>,
    forbidden: Vec<soroban_sdk::Address>,
    exact_count: Option<usize>,
}

impl<'a> AuthPolicy<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self {
            env,
            required: Vec::new(),
            forbidden: Vec::new(),
            exact_count: None,
        }
    }

    /// Assert that this address signed at least once.
    pub fn must_have_authorized(mut self, addr: &soroban_sdk::Address) -> Self {
        self.required.push(addr.clone());
        self
    }

    /// Assert that this address never signed.
    pub fn must_not_have_authorized(mut self, addr: &soroban_sdk::Address) -> Self {
        self.forbidden.push(addr.clone());
        self
    }

    /// Assert that exactly `n` unique addresses signed.
    pub fn exactly_n_signers(mut self, n: usize) -> Self {
        self.exact_count = Some(n);
        self
    }

    /// Evaluate the policy and return any violations (does not panic).
    pub fn check(&self) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();
        let auths = self.env.auths();

        let signed_addrs: Vec<soroban_sdk::Address> =
            auths.iter().map(|(addr, _)| addr.clone()).collect();

        for required in &self.required {
            if !signed_addrs.contains(required) {
                violations.push(PolicyViolation {
                    rule: "must_have_authorized".into(),
                    details: format!("{} did not sign", required),
                });
            }
        }

        for forbidden in &self.forbidden {
            if signed_addrs.contains(forbidden) {
                violations.push(PolicyViolation {
                    rule: "must_not_have_authorized".into(),
                    details: format!("{} signed but should not have", forbidden),
                });
            }
        }

        if let Some(expected_n) = self.exact_count {
            let unique: std::collections::HashSet<String> = signed_addrs
                .iter()
                .map(|a| format!("{}", a))
                .collect();
            if unique.len() != expected_n {
                violations.push(PolicyViolation {
                    rule: "exactly_n_signers".into(),
                    details: format!("expected {} signers, got {}", expected_n, unique.len()),
                });
            }
        }

        violations
    }

    /// Evaluate and panic on any violation.
    pub fn assert(self) {
        let violations = self.check();
        if violations.is_empty() {
            println!("{} Auth policy passed", "✓".green().bold());
            return;
        }
        eprintln!("\n{} Auth policy violations:", "✗".red().bold());
        for v in &violations {
            eprintln!("  {}", v.to_string().red());
        }
        panic!("anvil-auth: auth policy violated ({} rules)", violations.len());
    }
}
