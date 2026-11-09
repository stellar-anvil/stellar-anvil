//! # anvil-multi
//!
//! Multi-contract interaction harness for [Soroban](https://soroban.stellar.org) smart contracts.
//! Part of the [Stellar Anvil](https://github.com/stellar-anvil/stellar-anvil) toolkit.
//!
//! ## What this crate provides
//!
//! | Type | Purpose |
//! |---|---|
//! | [`ContractRegistry`] | Named address book with dependency-aware deployment ordering |
//! | [`Scenario`] | Named, stepped multi-contract interaction runner |
//! | [`CallTrace`] / [`CallNode`] | Cross-contract call tree recording |
//! | [`CallSequence`] | Assert call ordering properties (e.g. order-independence) |
//!
//! ## Quick start
//!
//! ### Register contracts and run a scenario
//!
//! ```ignore
//! use anvil_multi::{ContractRegistry, Scenario};
//!
//! let mut registry = ContractRegistry::new(&env);
//! registry.add("token", token_id.clone(), &[]);
//! registry.add("vault", vault_id.clone(), &["token"]); // vault depends on token
//! registry.print_tree();
//!
//! let mut scenario = Scenario::new("DeFi round-trip", &env, registry).verbose();
//!
//! scenario.step("alice deposits 2000", |env, reg| {
//!     VaultClient::new(env, reg.get("vault").unwrap()).deposit(&alice, &2_000i128);
//! });
//! scenario.step("alice withdraws", |env, reg| {
//!     let net = VaultClient::new(env, reg.get("vault").unwrap()).withdraw(&alice);
//!     assert!(net > 0);
//! });
//!
//! let results = scenario.run();
//! Scenario::print_summary("DeFi round-trip", &results);
//! ```
//!
//! ### Test call ordering
//!
//! ```ignore
//! use anvil_multi::CallSequence;
//!
//! let mut seq = CallSequence::new();
//! // record calls, then assert properties
//! seq.assert_order_independent();
//! ```

pub mod registry;
pub mod scenario;
pub mod call_trace;
pub mod sequence;

pub use registry::{ContractRegistry, ContractEntry};
pub use scenario::{Scenario, ScenarioStep, StepResult};
pub use call_trace::{CallTrace, CallNode};
pub use sequence::CallSequence;
