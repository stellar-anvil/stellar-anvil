use soroban_sdk::{Address, Env};

/// A named test account with an associated Soroban Address.
/// Accounts are created via `AccountBuilder` and live inside a `SandboxEnv`.
#[derive(Clone, Debug)]
pub struct Account {
    /// Human-readable alias (e.g. "alice", "admin", "attacker")
    pub alias: String,
    /// The Soroban Address for this account
    pub address: Address,
}

impl Account {
    /// Returns a reference to the account's address.
    pub fn address(&self) -> &Address {
        &self.address
    }
}

/// Fluent builder for creating named test accounts.
///
/// # Example
/// ```ignore
/// let alice = AccountBuilder::new("alice").build(&env);
/// ```
pub struct AccountBuilder {
    alias: String,
}

impl AccountBuilder {
    pub fn new(alias: impl Into<String>) -> Self {
        Self { alias: alias.into() }
    }

    /// Builds the account by generating a fresh address inside the given Env.
    pub fn build(self, env: &Env) -> Account {
        let address = Address::generate(env);
        Account {
            alias: self.alias,
            address,
        }
    }
}
