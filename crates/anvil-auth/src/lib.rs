//! # anvil-auth
//!
//! Wallet and authentication mocks for [Soroban](https://soroban.stellar.org) smart contracts.
//! Part of the [Stellar Anvil](https://github.com/stellar-anvil/stellar-anvil) toolkit.
//!
//! ## What this crate provides
//!
//! | Type | Purpose |
//! |---|---|
//! | [`MockWallet`] / [`WalletBuilder`] | Simulate Freighter wallet states (connected, locked, rejecting) |
//! | [`MockSep10Server`] | Full SEP-10 challenge/response auth flow in tests |
//! | [`MockInvoker`] | Invoke contracts as a controlled address with configurable auth policy |
//! | [`AuthPolicy`] | Declarative assertions on which addresses signed an invocation |
//!
//! ## Quick start
//!
//! ### Wallet states
//!
//! ```ignore
//! use anvil_auth::{WalletBuilder, WalletState};
//!
//! // Happy path
//! let mut alice = WalletBuilder::new("alice").with_balance(100).build(&env);
//! alice.mock_auth(&env);
//! vault.deposit(&alice.address, &500i128);
//!
//! // Simulate rejection
//! alice.reject_next(1);
//! // alice.mock_auth(&env) now panics: "wallet 'alice' rejected the authorization request"
//!
//! // Disconnected wallet
//! let bob = WalletBuilder::new("bob").disconnected().build(&env);
//! // bob.mock_auth(&env) panics: "wallet 'bob' is disconnected"
//! ```
//!
//! ### SEP-10 challenge/response
//!
//! ```ignore
//! use anvil_auth::MockSep10Server;
//! use std::time::Duration;
//!
//! let mut server = MockSep10Server::new().verbose();
//!
//! // Full happy-path flow
//! server.assert_auth_flow("GALICE...");
//!
//! // Expired challenge
//! let ch = server.issue_challenge("GBOB...", 0).unwrap(); // 0s TTL
//! std::thread::sleep(Duration::from_millis(10));
//! assert!(server.verify_challenge(&ch, "GBOB...").is_err());
//! ```
//!
//! ### Declarative signer assertions
//!
//! ```ignore
//! use anvil_auth::AuthPolicy;
//!
//! vault.deposit(&alice, &500i128);
//!
//! AuthPolicy::new(&env)
//!     .must_have_authorized(&alice)
//!     .must_not_have_authorized(&admin)
//!     .assert();
//! ```

pub mod wallet;
pub mod sep10;
pub mod invoker;
pub mod policy;

pub use wallet::{MockWallet, WalletBuilder, WalletState};
pub use sep10::{Sep10Challenge, Sep10Session, Sep10Error};
pub use invoker::{MockInvoker, InvokerMode};
pub use policy::{AuthPolicy, PolicyViolation};
