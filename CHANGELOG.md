# Changelog

All notable changes to Stellar Anvil are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.0] — 2026-11-09

Initial release of the Stellar Anvil testing and simulation framework for
Soroban smart contracts.

### Added

#### anvil-core
- `SandboxEnv` — named accounts, time travel (`advance_time`, `warp_to_sequence`,
  `warp_to_timestamp`, `advance_ledger`), checkpoint/restore, and CPU/memory
  budget tracking
- `AccountBuilder` — fluent API for creating test addresses with aliases
- `LedgerSnapshot` — load and save ledger state for snapshot-based testing
- `AnvilAssert` — chainable assertion helpers
- `AnvilError` — structured error type used across the workspace
- `output::configure_color` — process-global color control respecting `NO_COLOR`
  and `TERM=dumb`
- `output::fmt_num` — thousands-separator number formatting shared by all crates

#### anvil-profiler
- `Profiler` — wraps `soroban_sdk::Env` to measure CPU instructions, memory
  bytes, and estimated fee (stroops) for any closure
- `ProfileReport` — aggregated results with `print()`, `export_json()`,
  `export_markdown()`, and `assert_within()` helpers
- `BudgetThreshold` — per-call and total CPU/memory/fee limits with clear
  failure messages
- `ProfileComparison` — side-by-side delta table with `assert_no_regression(pct)`

#### anvil-multi
- `ContractRegistry` — named address book with topological dependency ordering
  and `print_tree()`
- `Scenario` — named, stepped multi-contract interaction runner with
  `step()`, `step_fails()`, `continue_on_failure()`, and `print_summary()`
- `CallTrace` / `CallNode` — cross-contract call tree recording
- `CallSequence` — assert call ordering properties

#### anvil-auth
- `MockWallet` / `WalletBuilder` — simulate Freighter wallet states: connected,
  disconnected, locked, and `reject_next(n)`
- `WalletSet` — manage a named collection of wallets for a test
- `MockSep10Server` — full SEP-10 challenge/response cycle with expiry,
  replay protection, and `assert_auth_flow()`
- `MockInvoker` — invoke contracts as a controlled address with `DenyAll` /
  `AllowAll` / `Custom` auth policies
- `AuthPolicy` — declarative `must_have_authorized` / `must_not_have_authorized`
  assertions

#### anvil-cli (`stellar-anvil` binary)
- `stellar-anvil test` — run contract tests with pretty/JSON/TAP output,
  filter, and `--nocapture`
- `stellar-anvil snapshot create/inspect/diff` — capture and compare ledger
  snapshots; diff uses canonical JSON to prevent phantom changes
- `stellar-anvil profile` — profile CPU/memory with threshold enforcement and
  Markdown export
- `stellar-anvil compare` — regression detection with configurable threshold
  and Markdown PR comment export
- `stellar-anvil info` — print tool and environment versions
- Global `--no-color`, `--quiet`, `--verbose` flags

#### Examples
- `counter` — basic `SandboxEnv`, time travel, and profiling
- `multi_contract` — token + vault with `ContractRegistry`, `Scenario`,
  and `MockWallet`
- `defi_vault` — time-lock vault with SEP-10 auth, `AuthPolicy`, and
  `CallSequence`

[Unreleased]: https://github.com/stellar-anvil/stellar-anvil/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/stellar-anvil/stellar-anvil/releases/tag/v0.1.0
