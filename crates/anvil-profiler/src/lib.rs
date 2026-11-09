//! # anvil-profiler
//!
//! CPU, memory, and fee profiling for [Soroban](https://soroban.stellar.org) smart contracts.
//! Part of the [Stellar Anvil](https://github.com/stellar-anvil/stellar-anvil) toolkit.
//!
//! ## What this crate provides
//!
//! | Type | Purpose |
//! |---|---|
//! | [`Profiler`] | Wraps a `soroban_sdk::Env` and measures the cost of any closure |
//! | [`ProfileReport`] | Aggregated results with print / export / assert helpers |
//! | [`BudgetThreshold`] | Per-call and total CPU/memory/fee limits |
//! | [`ProfileComparison`] | Side-by-side diff of two reports for regression detection |
//!
//! ## Quick start
//!
//! ### Measure and print
//!
//! ```ignore
//! use anvil_profiler::{Profiler, BudgetThreshold};
//!
//! let mut profiler = Profiler::new(&env).verbose();
//!
//! profiler.measure("transfer(alice→bob, 1000)", || {
//!     token_client.transfer(&alice, &bob, &1000_i128)
//! });
//! profiler.measure("balance_of(alice)", || {
//!     token_client.balance(&alice)
//! });
//!
//! let report = profiler.into_report("TokenContract");
//! report.print();                              // colored table to stdout
//! report.export_markdown("profile.md").ok();  // for PR comments
//! report.export_json("profile.json").ok();    // for CI comparison
//! ```
//!
//! ### Assert budget constraints
//!
//! ```ignore
//! let threshold = BudgetThreshold::default()
//!     .max_cpu(5_000_000)
//!     .max_memory(500_000);
//! report.assert_within(&threshold);
//! ```
//!
//! ### Detect regressions in CI
//!
//! ```ignore
//! use anvil_profiler::{ProfileReport, ProfileComparison};
//!
//! let baseline  = ProfileReport::load("baseline.json")?;
//! let candidate = ProfileReport::load("candidate.json")?;
//!
//! let cmp = ProfileComparison::new("v1 vs v2", &baseline, &candidate);
//! cmp.print();
//! cmp.assert_no_regression(10.0); // fail if any call got >10% worse
//! ```

pub mod profile;
pub mod report;
pub mod thresholds;
pub mod compare;

pub use profile::{ProfileResult, Profiler};
pub use report::ProfileReport;
pub use thresholds::{BudgetThreshold, ThresholdResult};
pub use compare::ProfileComparison;
