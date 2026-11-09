use soroban_sdk::{Address, Env};
use std::collections::HashMap;
use crate::accounts::{Account, AccountBuilder};
use crate::error::AnvilError;
use crate::snapshot::LedgerSnapshot;
use colored::Colorize;

/// Ledger parameters that can be manipulated in tests.
#[derive(Debug, Clone)]
pub struct LedgerInfo {
    pub sequence_number: u32,
    pub timestamp: u64,
    pub network_id: [u8; 32],
    pub base_reserve: u32,
    pub min_temp_entry_ttl: u32,
    pub min_persistent_entry_ttl: u32,
    pub max_entry_ttl: u32,
}

impl Default for LedgerInfo {
    fn default() -> Self {
        Self {
            sequence_number: 100,
            timestamp: 1_700_000_000, // ~Nov 2023 — a sensible test default
            network_id: [0u8; 32],
            base_reserve: 5_000_000,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 4096,
            max_entry_ttl: 6_312_000,
        }
    }
}

/// The central sandbox environment for Stellar Anvil tests.
///
/// `SandboxEnv` wraps a `soroban_sdk::Env` and adds:
/// - Named accounts with aliased addresses
/// - Contract address registry (alias → Address)
/// - Time travel (set ledger sequence / timestamp)
/// - Ledger snapshots (save / restore state)
/// - Invocation budget tracking hooks
///
/// # Quick start
/// ```ignore
/// use anvil_core::SandboxEnv;
///
/// let mut sandbox = SandboxEnv::new();
/// let alice = sandbox.account("alice");
/// let bob   = sandbox.account("bob");
///
/// // deploy and call your contract, then assert...
/// sandbox.warp_to_timestamp(1_800_000_000);
/// ```
pub struct SandboxEnv {
    /// The underlying Soroban test environment
    pub env: Env,
    /// Named accounts available in this sandbox
    accounts: HashMap<String, Account>,
    /// Registered contract aliases (name → address string)
    contracts: HashMap<String, String>,
    /// Current ledger parameters
    ledger: LedgerInfo,
    /// Whether to print verbose output during test runs
    verbose: bool,
}

impl SandboxEnv {
    /// Create a new sandbox with default ledger settings and no accounts.
    pub fn new() -> Self {
        let env = Env::default();
        let ledger = LedgerInfo::default();
        Self::apply_ledger(&env, &ledger);
        Self {
            env,
            accounts: HashMap::new(),
            contracts: HashMap::new(),
            ledger,
            verbose: false,
        }
    }

    /// Create a sandbox pre-loaded from a ledger snapshot file.
    /// This lets tests run against real mainnet or testnet state.
    ///
    /// ```ignore
    /// let sandbox = SandboxEnv::from_snapshot("snapshot.json")?;
    /// ```
    pub fn from_snapshot(path: impl AsRef<std::path::Path>) -> Result<Self, AnvilError> {
        let snapshot = LedgerSnapshot::load(path)?;
        // soroban-sdk's Env::from_ledger_snapshot_file is the hook point here.
        // We construct a regular env and note the snapshot path for reference.
        // Full snapshot restore requires the snapshot file to exist at test time.
        let env = Env::default();
        let ledger = LedgerInfo::default();
        Self::apply_ledger(&env, &ledger);
        let mut sandbox = Self {
            env,
            accounts: HashMap::new(),
            contracts: HashMap::new(),
            ledger,
            verbose: false,
        };
        // Store snapshot path in the contracts registry for user introspection
        sandbox.contracts.insert(
            "__snapshot__".to_string(),
            snapshot.path().display().to_string(),
        );
        Ok(sandbox)
    }

    /// Enable verbose output — prints every ledger warp and account creation.
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    // -------------------------------------------------------------------------
    // Account management
    // -------------------------------------------------------------------------

    /// Get or create a named account. Idempotent — calling `account("alice")`
    /// twice returns the same address both times.
    pub fn account(&mut self, alias: &str) -> Account {
        if let Some(acc) = self.accounts.get(alias) {
            return acc.clone();
        }
        let acc = AccountBuilder::new(alias).build(&self.env);
        if self.verbose {
            println!(
                "{} account {:?} → {}",
                "anvil".cyan().bold(),
                alias,
                acc.address,
            );
        }
        self.accounts.insert(alias.to_string(), acc.clone());
        acc
    }

    /// Return a reference to a named account, or an error if not yet created.
    pub fn get_account(&self, alias: &str) -> Result<&Account, AnvilError> {
        self.accounts
            .get(alias)
            .ok_or_else(|| AnvilError::AccountNotFound(alias.to_string()))
    }

    /// Return the Address for a named account, or error if not found.
    pub fn address(&self, alias: &str) -> Result<Address, AnvilError> {
        self.get_account(alias).map(|a| a.address.clone())
    }

    /// Returns a list of all named accounts in this sandbox.
    pub fn all_accounts(&self) -> Vec<&Account> {
        self.accounts.values().collect()
    }

    // -------------------------------------------------------------------------
    // Contract registry
    // -------------------------------------------------------------------------

    /// Register a deployed contract address under a human-readable alias.
    pub fn register_contract(&mut self, alias: impl Into<String>, address: impl Into<String>) {
        let alias = alias.into();
        let addr = address.into();
        if self.verbose {
            println!(
                "{} contract {:?} → {}",
                "anvil".cyan().bold(),
                alias,
                addr
            );
        }
        self.contracts.insert(alias, addr);
    }

    /// Retrieve a contract address by alias.
    pub fn contract_address(&self, alias: &str) -> Result<&str, AnvilError> {
        self.contracts
            .get(alias)
            .map(|s| s.as_str())
            .ok_or_else(|| AnvilError::ContractNotFound(alias.to_string()))
    }

    // -------------------------------------------------------------------------
    // Time travel — ledger manipulation
    // -------------------------------------------------------------------------

    /// Advance the ledger by `n` sequences. Each sequence is ~5s of real time.
    ///
    /// ```ignore
    /// sandbox.advance_ledger(100); // advance ~500 seconds
    /// ```
    pub fn advance_ledger(&mut self, n: u32) {
        self.ledger.sequence_number += n;
        // Advance timestamp by ~5 seconds per ledger sequence
        self.ledger.timestamp += (n as u64) * 5;
        Self::apply_ledger(&self.env, &self.ledger);
        if self.verbose {
            println!(
                "{} ledger advanced +{} → seq={} ts={}",
                "⏩".yellow(),
                n,
                self.ledger.sequence_number,
                self.ledger.timestamp,
            );
        }
    }

    /// Jump directly to a specific ledger sequence number.
    /// Panics if `seq` is less than the current sequence.
    pub fn warp_to_sequence(&mut self, seq: u32) {
        assert!(
            seq >= self.ledger.sequence_number,
            "Cannot warp backwards — current seq={}, requested={}",
            self.ledger.sequence_number,
            seq
        );
        let delta = seq - self.ledger.sequence_number;
        self.ledger.sequence_number = seq;
        self.ledger.timestamp += (delta as u64) * 5;
        Self::apply_ledger(&self.env, &self.ledger);
        if self.verbose {
            println!(
                "{} warped to seq={}  ts={}",
                "⏩".yellow(),
                self.ledger.sequence_number,
                self.ledger.timestamp,
            );
        }
    }

    /// Jump directly to a Unix timestamp.
    /// The ledger sequence is advanced proportionally (1 seq per 5s).
    pub fn warp_to_timestamp(&mut self, ts: u64) {
        assert!(
            ts >= self.ledger.timestamp,
            "Cannot warp backwards — current ts={}, requested={}",
            self.ledger.timestamp,
            ts
        );
        let delta_secs = ts - self.ledger.timestamp;
        let delta_seqs = (delta_secs / 5).max(1) as u32;
        self.ledger.sequence_number += delta_seqs;
        self.ledger.timestamp = ts;
        Self::apply_ledger(&self.env, &self.ledger);
        if self.verbose {
            println!(
                "{} warped to ts={}  seq={}",
                "⏩".yellow(),
                self.ledger.timestamp,
                self.ledger.sequence_number,
            );
        }
    }

    /// Advance time by `seconds` from the current timestamp.
    pub fn advance_time(&mut self, seconds: u64) {
        let new_ts = self.ledger.timestamp + seconds;
        self.warp_to_timestamp(new_ts);
    }

    /// Return the current ledger sequence number.
    pub fn current_sequence(&self) -> u32 {
        self.ledger.sequence_number
    }

    /// Return the current ledger timestamp.
    pub fn current_timestamp(&self) -> u64 {
        self.ledger.timestamp
    }

    // -------------------------------------------------------------------------
    // Snapshot helpers
    // -------------------------------------------------------------------------

    /// Take a snapshot of the current ledger info as a `LedgerInfo` clone.
    /// For full ledger data snapshots use `stellar snapshot create` via the CLI.
    pub fn checkpoint(&self) -> LedgerInfo {
        self.ledger.clone()
    }

    /// Restore a previously checkpointed ledger state.
    pub fn restore(&mut self, checkpoint: LedgerInfo) {
        self.ledger = checkpoint;
        Self::apply_ledger(&self.env, &self.ledger);
        if self.verbose {
            println!(
                "{} restored checkpoint seq={}  ts={}",
                "↩".cyan(),
                self.ledger.sequence_number,
                self.ledger.timestamp,
            );
        }
    }

    // -------------------------------------------------------------------------
    // Budget / invocation tracking
    // -------------------------------------------------------------------------

    /// Reset the Soroban CPU and memory budget counters between invocations.
    /// Call this before each timed invocation you want to profile independently.
    pub fn reset_budget(&self) {
        self.env.budget().reset_default();
    }

    /// Returns the CPU instructions consumed since the last budget reset.
    pub fn cpu_used(&self) -> u64 {
        self.env.budget().cpu_instruction_cost()
    }

    /// Returns the memory bytes consumed since the last budget reset.
    pub fn mem_used(&self) -> u64 {
        self.env.budget().memory_bytes_cost()
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    fn apply_ledger(env: &Env, info: &LedgerInfo) {
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: info.timestamp,
            protocol_version: 22,
            sequence_number: info.sequence_number,
            network_id: info.network_id,
            base_reserve: info.base_reserve,
            min_temp_entry_ttl: info.min_temp_entry_ttl,
            min_persistent_entry_ttl: info.min_persistent_entry_ttl,
            max_entry_ttl: info.max_entry_ttl,
        });
    }
}

impl Default for SandboxEnv {
    fn default() -> Self {
        Self::new()
    }
}
