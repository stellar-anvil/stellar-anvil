use soroban_sdk::{Address, Env};
use std::collections::HashMap;
use colored::Colorize;

/// The connection state of a mock wallet.
#[derive(Debug, Clone, PartialEq)]
pub enum WalletState {
    Connected,
    Disconnected,
    WillReject(usize),
    Locked,
}

/// A mock Freighter-compatible wallet for use in Stellar Anvil tests.
///
/// # Example
/// ```ignore
/// let mut wallet = WalletBuilder::new("alice")
///     .with_balance(10_000_i128)
///     .build(&env);
///
/// wallet.mock_auth(&env);
/// wallet.reject_next(1);
/// ```
#[derive(Clone, Debug)]
pub struct MockWallet {
    pub alias: String,
    pub address: Address,
    pub state: WalletState,
    pub balance_stroops: i64,
    pub approved_contracts: Vec<String>,
    pub auth_call_count: usize,
    pub auth_reject_count: usize,
}

impl MockWallet {
    pub fn connect(&mut self) {
        self.state = WalletState::Connected;
    }

    pub fn disconnect(&mut self) {
        self.state = WalletState::Disconnected;
    }

    pub fn lock(&mut self) {
        self.state = WalletState::Locked;
    }

    pub fn reject_next(&mut self, n: usize) {
        self.state = WalletState::WillReject(n);
    }

    pub fn can_sign(&self) -> bool {
        matches!(self.state, WalletState::Connected)
    }

    /// Mock all authorizations from this wallet's address in the given Env.
    /// Panics with a clear message if the wallet is not connected.
    pub fn mock_auth(&mut self, env: &Env) {
        self.auth_call_count += 1;

        match &self.state {
            WalletState::Disconnected => {
                panic!(
                    "{} wallet '{}' is disconnected — cannot authorize",
                    "anvil-auth".red().bold(),
                    self.alias
                );
            }
            WalletState::Locked => {
                panic!(
                    "{} wallet '{}' is locked — cannot authorize",
                    "anvil-auth".red().bold(),
                    self.alias
                );
            }
            WalletState::WillReject(n) => {
                let remaining = *n;
                self.auth_reject_count += 1;
                if remaining <= 1 {
                    self.state = WalletState::Connected;
                } else {
                    self.state = WalletState::WillReject(remaining - 1);
                }
                panic!(
                    "{} wallet '{}' rejected the authorization request ({} remaining)",
                    "anvil-auth".yellow().bold(),
                    self.alias,
                    remaining - 1,
                );
            }
            WalletState::Connected => {
                env.mock_all_auths();
            }
        }
    }

    /// Mock auth only for a specific contract address.
    pub fn mock_auth_for(&mut self, env: &Env, _contract: &Address) {
        self.auth_call_count += 1;
        if !self.can_sign() {
            panic!(
                "{} wallet '{}' cannot sign (state: {:?})",
                "anvil-auth".red().bold(),
                self.alias,
                self.state
            );
        }
        env.mock_all_auths();
    }

    pub fn recorded_auths<'a>(&self, env: &'a Env) -> soroban_sdk::testutils::AuthorizedInvocations {
        env.auths()
    }

    pub fn assert_signed_count(&self, env: &Env, n: usize) {
        let auths = env.auths();
        let count = auths.iter().filter(|(addr, _)| addr == &self.address).count();
        if count != n {
            panic!(
                "{} wallet '{}': expected {} auth invocations, got {}",
                "anvil-auth".red().bold(),
                self.alias,
                n,
                count,
            );
        }
        println!(
            "{} wallet '{}' signed exactly {} invocation(s)",
            "✓".green(),
            self.alias,
            n,
        );
    }

    pub fn print_summary(&self) {
        println!(
            "  {} '{}' — state={:?}  calls={}  rejects={}  balance={} XLM",
            "wallet".cyan().bold(),
            self.alias,
            self.state,
            self.auth_call_count,
            self.auth_reject_count,
            self.balance_stroops / 10_000_000,
        );
    }
}

pub struct WalletBuilder {
    alias: String,
    balance_stroops: i64,
    approved_contracts: Vec<String>,
    initial_state: WalletState,
}

impl WalletBuilder {
    pub fn new(alias: impl Into<String>) -> Self {
        Self {
            alias: alias.into(),
            balance_stroops: 100 * 10_000_000,
            approved_contracts: Vec::new(),
            initial_state: WalletState::Connected,
        }
    }

    pub fn with_balance(mut self, xlm: i64) -> Self {
        self.balance_stroops = xlm * 10_000_000;
        self
    }

    pub fn disconnected(mut self) -> Self {
        self.initial_state = WalletState::Disconnected;
        self
    }

    pub fn approve_contracts(mut self, contracts: &[&str]) -> Self {
        self.approved_contracts = contracts.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn build(self, env: &Env) -> MockWallet {
        let address = Address::generate(env);
        MockWallet {
            alias: self.alias,
            address,
            state: self.initial_state,
            balance_stroops: self.balance_stroops,
            approved_contracts: self.approved_contracts,
            auth_call_count: 0,
            auth_reject_count: 0,
        }
    }
}

/// A set of named wallets for a test.
#[derive(Default)]
pub struct WalletSet {
    wallets: HashMap<String, MockWallet>,
}

impl WalletSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, wallet: MockWallet) {
        self.wallets.insert(wallet.alias.clone(), wallet);
    }

    pub fn get(&self, alias: &str) -> Option<&MockWallet> {
        self.wallets.get(alias)
    }

    pub fn get_mut(&mut self, alias: &str) -> Option<&mut MockWallet> {
        self.wallets.get_mut(alias)
    }

    pub fn mock_all_auths(&mut self, env: &Env) {
        for wallet in self.wallets.values_mut() {
            wallet.mock_auth(env);
        }
    }

    pub fn print_summary(&self) {
        println!("\n{}", "Wallet Set Summary".cyan().bold());
        for wallet in self.wallets.values() {
            wallet.print_summary();
        }
    }
}
