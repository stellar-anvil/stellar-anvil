# Contributing to Stellar Anvil

Thanks for your interest in contributing. Stellar Anvil is enrolled in the
[Stellar Wave Program](https://drips.network) — contributions earn points
towards the reward pool.

## Table of contents

- [Getting started](#getting-started)
- [How to file an issue](#how-to-file-an-issue)
- [How to submit a pull request](#how-to-submit-a-pull-request)
- [Commit message format](#commit-message-format)
- [Code style](#code-style)
- [Running the test suite](#running-the-test-suite)
- [Crate structure](#crate-structure)
- [Code of conduct](#code-of-conduct)

---

## Getting started

```bash
# 1. Fork and clone
git clone https://github.com/stellar-anvil/stellar-anvil
cd stellar-anvil

# 2. Install stellar-cli (required for snapshot commands)
cargo install --locked stellar-cli

# 3. Build everything
cargo build

# 4. Run all tests
cargo test --features testutils

# 5. Install the CLI locally
cargo install --path crates/anvil-cli
```

Rust stable is required. The toolchain is pinned in `rust-toolchain.toml` —
`rustup` will pick it up automatically.

---

## How to file an issue

Use the GitHub issue tracker. Before opening a new issue:

1. Search existing issues to avoid duplicates.
2. Use the appropriate template (bug report or feature request).
3. Include a minimal reproducible example for bug reports.

Good first issues are labeled `good first issue`.

---

## How to submit a pull request

1. **Open an issue first** for non-trivial changes so we can discuss the approach.
2. Create a branch from `main`: `git checkout -b feat/my-feature`.
3. Make your changes, keeping commits focused.
4. Ensure CI passes locally before pushing:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --features testutils -- -D warnings
   cargo test --all --features testutils
   ```
5. Push and open a PR against `main`.
6. Fill in the PR template — summary, testing notes, and any breaking changes.

PRs are squash-merged. Keep the PR title concise (≤70 chars); details go in
the description.

---

## Commit message format

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>

[optional body]

[optional footer]
```

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `ci`

**Scopes** (optional): `core`, `profiler`, `multi`, `auth`, `cli`, `examples`

Examples:
```
feat(profiler): add P95 latency column to ProfileReport
fix(snapshot): use canonical JSON in diff to prevent phantom changes
docs(auth): add SEP-10 usage example to lib.rs
```

---

## Code style

- Run `cargo fmt --all` before every commit.
- Fix all `cargo clippy` warnings — CI runs with `-D warnings`.
- Keep functions small and well-documented with `///` doc comments.
- Public API items require a doc comment. Private helpers are encouraged but not required.
- Prefer `anyhow::bail!` for fatal errors in CLI commands; use typed `AnvilError` variants in library code.

---

## Running the test suite

```bash
# All workspace tests
cargo test --all --features testutils

# Single crate
cargo test -p anvil-core --features testutils

# Specific example
cargo test -p counter --features testutils

# With stdout/stderr visible
cargo test --all --features testutils -- --nocapture
```

---

## Crate structure

```
crates/
  anvil-core      # SandboxEnv, time travel, accounts, snapshots
  anvil-profiler  # Profiler, ProfileReport, BudgetThreshold, ProfileComparison
  anvil-multi     # ContractRegistry, Scenario, CallTrace, CallSequence
  anvil-auth      # MockWallet, MockSep10Server, MockInvoker, AuthPolicy
  anvil-cli       # stellar-anvil binary
examples/
  counter         # Basic: SandboxEnv, time travel, profiling
  multi_contract  # Token + Vault: Registry, Scenario, MockWallet
  defi_vault      # TimeLock: SEP-10, AuthPolicy, CallSequence
```

See [`docs/architecture.md`](docs/architecture.md) for a deeper walkthrough.

---

## Code of conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
By participating you agree to abide by its terms.
