# Stellar Anvil — Architecture Guide

This document explains how the crates fit together, the key design decisions,
and how to extend the framework.

---

## High-level picture

```
┌─────────────────────────────────────────────────────────────┐
│                      Your contract tests                     │
└──────┬──────────┬──────────┬──────────┬──────────────────────┘
       │          │          │          │
  anvil-core  anvil-profiler anvil-multi anvil-auth
       │          │          │          │
       └──────────┴──────────┴──────────┘
                       │
               soroban-sdk testutils
                       │
               stellar-anvil CLI  (anvil-cli)
```

All four library crates depend on `anvil-core` for the `SandboxEnv` wrapper,
shared error types, and output utilities (`fmt_num`, `configure_color`).
They are independent of each other — you can use any subset.

---

## Crate responsibilities

### `anvil-core`

**Role:** The substrate everything else builds on.

Key types:

| Type | File | What it does |
|---|---|---|
| `SandboxEnv` | `sandbox.rs` | Wraps `soroban_sdk::Env`, adds named accounts, time-travel, checkpoint/restore, and budget accessors |
| `Account` / `AccountBuilder` | `accounts.rs` | Named test address with alias string; idempotent via `sandbox.account("alice")` |
| `LedgerSnapshot` | `snapshot.rs` | Load / save `latestLedger` + `entries` JSON (delegates to `stellar-cli` for creation) |
| `AnvilAssert` | `assertions.rs` | Fluent assertion helpers that produce readable panic messages |
| `AnvilError` | `error.rs` | Typed error enum covering all cross-crate error conditions |
| `output` | `output.rs` | `configure_color`, `fmt_num` — shared across the workspace |

**Design note:** `SandboxEnv` deliberately does not wrap `soroban_sdk::Env` in
a newtype — it holds `pub env: Env` so users can call SDK methods directly.
This avoids the need to re-expose every SDK method and lets the sandbox grow
independently.

---

### `anvil-profiler`

**Role:** Measure and compare resource budgets.

Key types:

| Type | File | What it does |
|---|---|---|
| `Profiler` | `profile.rs` | Wraps closures in `env.budget().reset_default()` / capture cycle |
| `ProfileResult` | `profile.rs` | Single measurement: label, CPU insns, memory bytes, succeeded flag |
| `ProfileReport` | `report.rs` | Vec of results with print/export/assert_within helpers |
| `BudgetThreshold` | `thresholds.rs` | Per-call and total limits; `check()` returns a `ThresholdResult` |
| `ProfileComparison` | `compare.rs` | Diff two reports; `assert_no_regression(pct)` panics on violations |

**Protocol for CLI integration:** Tests that want to emit profile data to the
CLI emit lines to stdout with the prefix `ANVIL_PROFILE|`. The CLI's `profile`
command sets `STELLAR_ANVIL_PROFILE=1` before running `cargo test --nocapture`
and collects those lines. This is intentionally low-tech — no IPC, no sockets,
just stdout parsing — keeping the library fully usable without the CLI.

---

### `anvil-multi`

**Role:** Orchestrate interactions between multiple deployed contracts.

Key types:

| Type | File | What it does |
|---|---|---|
| `ContractRegistry` | `registry.rs` | Named address book with topological sort (`deployment_order()`) and `print_tree()` |
| `ContractEntry` | `registry.rs` | Name, address, optional version, dependency list |
| `Scenario` | `scenario.rs` | Named step runner; catches panics per-step, records CPU/mem, supports `step_fails()` |
| `ScenarioStep` | `scenario.rs` | Single closure with a label and expected-success flag |
| `StepResult` | `scenario.rs` | Outcome of one step: succeeded, CPU, mem, optional error string |
| `CallTrace` / `CallNode` | `call_trace.rs` | Tree structure recording cross-contract calls |
| `CallSequence` | `sequence.rs` | Assert ordering properties on a recorded call trace |

**Design note:** `Scenario::run()` uses `std::panic::catch_unwind` so a single
failing step can be recorded without aborting the whole scenario (when
`continue_on_failure()` is set). The final `print_summary` call then panics if
any step failed, which properly fails the test.

---

### `anvil-auth`

**Role:** Mock wallet states and authentication flows without touching a live network.

Key types:

| Type | File | What it does |
|---|---|---|
| `MockWallet` / `WalletBuilder` | `wallet.rs` | Simulates Freighter: connected / disconnected / locked / `reject_next(n)` |
| `WalletSet` | `wallet.rs` | Named collection of wallets; `mock_all_auths()` fires them all |
| `MockSep10Server` | `sep10.rs` | Issues and verifies SEP-10 challenges; replay protection via token revocation |
| `Sep10Challenge` / `Sep10Session` / `Sep10Error` | `sep10.rs` | Challenge/response value types |
| `MockInvoker` | `invoker.rs` | Controlled address with `AllowAll` / `DenyAll` / `Custom` auth mode |
| `AuthPolicy` | `policy.rs` | Declarative `must_have_authorized` / `must_not_have_authorized` assertions over `env.auths()` |

**Design note:** All panic messages from this crate are prefixed with
`"anvil-auth"` in bold red/yellow so they are instantly recognizable in test
output and clearly distinguished from contract panics.

---

### `anvil-cli`

**Role:** Developer-facing binary wrapping the library crates.

```
src/
  main.rs          # clap root, --no-color / --quiet / --verbose, banner
  output.rs        # configure_color, fmt_num, section, divider, color_delta, legend
  commands/
    test.rs        # stellar-anvil test
    profile.rs     # stellar-anvil profile
    compare.rs     # stellar-anvil compare
    snapshot.rs    # stellar-anvil snapshot create/inspect/diff
    info.rs        # stellar-anvil info
```

The CLI does not re-implement any core logic — it shells out to `cargo test`
and `stellar` for their respective operations and post-processes the output.

---

## Data flow: profile regression in CI

```
PR branch
  └─ cargo test -p defi-vault ... --nocapture
       └─ stdout: ANVIL_PROFILE|deposit|cpu=123456|mem=78901
                  ANVIL_PROFILE|withdraw|cpu=98765|mem=54321
  └─ stellar-anvil profile parses those lines → candidate_profile.json

main branch (checked out for baseline)
  └─ same test → baseline_profile.json

stellar-anvil compare baseline_profile.json candidate_profile.json --threshold 15
  └─ exits non-zero if any call regressed > 15%
  └─ writes regression_report.md

GitHub Actions github-script step posts regression_report.md as a PR comment
```

---

## Adding a new crate

1. Create `crates/anvil-<name>/` with a `Cargo.toml` following the pattern in
   existing crates (inherit workspace fields, add `publish = false`, keywords,
   categories).
2. Add `"crates/anvil-<name>"` to the `members` array in the root `Cargo.toml`.
3. Add a `pub mod anvil_<name>` re-export path if needed.
4. Write a `//!` crate-level doc comment in `src/lib.rs` with a type table and
   quick-start example (see existing crates for the pattern).
5. Add the crate as a dependency of `anvil-cli` if CLI integration is needed.

---

## Key design principles

1. **No network in tests.** Everything runs offline against `soroban_sdk::Env`.
   The only exception is `stellar-anvil snapshot create`, which calls `stellar-cli`.

2. **Panics are the test failure mechanism.** Soroban test assertions panic;
   `Scenario`, `AuthPolicy`, `BudgetThreshold`, and `ProfileComparison` all
   follow the same convention so test output is consistent.

3. **Library without CLI.** Every library crate is fully usable in plain
   `cargo test` without the CLI binary. The CLI adds formatting and workflow
   convenience, not functionality.

4. **Progressive disclosure.** `SandboxEnv::new()` works with zero config.
   Opt in to verbose output, named contracts, profiling, and auth mocks as
   needed — nothing is mandatory.
