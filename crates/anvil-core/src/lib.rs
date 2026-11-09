//! # anvil-core
//!
//! The foundation crate for Stellar Anvil — a Foundry/Hardhat-inspired testing
//! and simulation framework for [Soroban](https://soroban.stellar.org) smart contracts.
//!
//! ## What this crate provides
//!
//! | Type | Purpose |
//! |---|---|
//! | [`SandboxEnv`] | Central test environment with named accounts, time travel, and budget tracking |
//! | [`Account`] / [`AccountBuilder`] | Named, reusable test addresses |
//! | [`LedgerSnapshot`] | Load / save ledger state for snapshot-based testing |
//! | [`AnvilAssert`] | Chainable assertion helpers |
//! | [`AnvilError`] | Structured error type used across the workspace |
//!
//! ## Quick start
//!
//! ```ignore
//! use anvil_core::SandboxEnv;
//!
//! let mut sandbox = SandboxEnv::new();
//!
//! // Named accounts — idempotent, same address on repeated calls
//! let alice = sandbox.account("alice");
//! let bob   = sandbox.account("bob");
//!
//! // Deploy your contract
//! let contract_id = sandbox.env.register(MyContract, ());
//!
//! // Time travel — advance by 1 week
//! sandbox.advance_time(7 * 24 * 3_600);
//!
//! // Jump to a specific ledger sequence
//! sandbox.warp_to_sequence(5_000);
//!
//! // Checkpoint / restore
//! let cp = sandbox.checkpoint();
//! sandbox.advance_ledger(500);
//! sandbox.restore(cp);
//!
//! // Budget tracking
//! sandbox.reset_budget();
//! client.some_fn();
//! println!("CPU: {}  Mem: {}", sandbox.cpu_used(), sandbox.mem_used());
//! ```
//!
//! ## Color control
//!
//! All verbose output uses ANSI color via the [`colored`](https://docs.rs/colored) crate.
//! Call [`output::configure_color`] once at startup to disable color when needed:
//!
//! ```no_run
//! anvil_core::output::configure_color(true);  // disable color
//! ```
//!
//! The `stellar-anvil` CLI does this automatically when `--no-color` is passed,
//! `NO_COLOR` is set, or `TERM=dumb`.

pub mod sandbox;
pub mod snapshot;
pub mod assertions;
pub mod accounts;
pub mod error;
pub mod output;

pub use sandbox::SandboxEnv;
pub use snapshot::{LedgerSnapshot, SnapshotConfig};
pub use assertions::AnvilAssert;
pub use accounts::{Account, AccountBuilder};
pub use error::AnvilError;
