# Stellar Anvil — Examples

Three fully-worked example contracts that demonstrate the framework from simple
to complex. Each example is a standalone Cargo package in the workspace.

---

## counter

**Path:** `examples/counter/`  
**Run:** `cargo test -p counter --features testutils`

The simplest possible starting point. Demonstrates:

- `SandboxEnv::new()` and named accounts (`sandbox.account("alice")`)
- Deploying and calling a contract through the sandbox
- `advance_time()` and `warp_to_sequence()` time travel
- `checkpoint()` / `restore()` state snapshots
- `Profiler` measuring a single call and printing a report
- `BudgetThreshold` asserting a CPU limit

**Good first read** if you're new to the framework.

---

## multi_contract

**Path:** `examples/multi_contract/`  
**Run:** `cargo test -p multi-contract --features testutils`

A token + vault pair that interact across contract boundaries. Demonstrates:

- `ContractRegistry` with dependency declarations (`vault` depends on `token`)
- `registry.print_tree()` — visual dependency graph
- `Scenario` with multiple named steps and `print_summary`
- `MockWallet` connected / disconnected / reject states
- `WalletSet` managing multiple wallets at once
- `ProfileComparison` detecting regressions between two runs
- `CallTrace` recording cross-contract call order

**Good second read** once you're comfortable with `counter`.

---

## defi_vault

**Path:** `examples/defi_vault/`  
**Run:** `cargo test -p defi-vault --features testutils`

A time-lock DeFi vault with authorization requirements. Demonstrates:

- `MockSep10Server` — full challenge/response auth flow
- Expired challenge and replay-protection error paths
- `MockInvoker` with `DenyAll` mode testing unauthorized access
- `AuthPolicy` asserting correct signers on each invocation
- `CallSequence` asserting call ordering properties
- Exporting a `ProfileReport` to JSON for CI regression tracking
- Loading the exported JSON in a subsequent test for comparison

**Good reference** for auth-heavy contracts and CI integration patterns.

---

## Running all examples

```bash
cargo test --features testutils
```

This runs every example in the workspace. To run a single example with visible
output:

```bash
cargo test -p defi-vault --features testutils -- --nocapture
```

---

## Using the CLI with examples

```bash
# Profile the defi_vault example
stellar-anvil profile --package defi-vault

# Compare two saved profiles
stellar-anvil compare baseline_profile.json candidate_profile.json --threshold 10

# Run all tests with pretty output
stellar-anvil test
```
