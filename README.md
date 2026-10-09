# ⚡ Stellar Anvil

**The missing testing and simulation framework for Soroban smart contracts.**

Stellar Anvil is a Foundry/Hardhat-inspired developer toolkit for the [Stellar](https://stellar.org) ecosystem. It fills the biggest gap in the Soroban developer experience: a fast, local, batteries-included test framework with time travel, gas profiling, multi-contract scenarios, and wallet auth mocks — all without touching a live network.

> Built for the [Stellar Wave Program](https://drips.network) on Drips.

---

## Why Stellar Anvil?

The existing Soroban test story is: write raw Rust unit tests, call `cargo test`, hope for the best. There is no equivalent of Foundry's `vm.warp()`, no gas profiler, no cross-contract scenario runner, no mock wallet. Every team on the Stellar Wave program reinvents these primitives from scratch.

Stellar Anvil fixes that.

| Capability | Without Anvil | With Anvil |
|---|---|---|
| Time travel | Manual ledger struct mutation | `sandbox.advance_time(3600)` |
| Gas profiling | Read raw budget numbers | `Profiler` + colored table + Markdown export |
| Regression detection | None | `ProfileComparison::assert_no_regression(10.0)` |
| Multi-contract tests | Ad-hoc wiring | `Scenario` + `ContractRegistry` |
| Wallet mocks | `env.mock_all_auths()` everywhere | `MockWallet` with connect/disconnect/reject states |
| SEP-10 auth tests | Not tested | `MockSep10Server` full challenge/response cycle |
| Auth policy checks | Manual auth inspection | `AuthPolicy::must_have_authorized(alice).assert()` |
| Ledger snapshots | Manual `stellar snapshot create` | `stellar-anvil snapshot create/inspect/diff` |
| Call order testing | None | `CallSequence::assert_order_independent()` |
| Test output | Raw cargo output | Pretty, JSON, or TAP with timing |

---

## Crate Structure

```
stellar-anvil/
├── crates/
│   ├── anvil-core       # SandboxEnv, time travel, accounts, assertions, snapshots
│   ├── anvil-profiler   # CPU/mem profiler, budget thresholds, regression comparison
│   ├── anvil-multi      # ContractRegistry, Scenario, CallTrace, CallSequence
│   ├── anvil-auth       # MockWallet, MockSep10Server, MockInvoker, AuthPolicy
│   └── anvil-cli        # `stellar-anvil` binary
└── examples/
    ├── counter          # Basic: SandboxEnv, time travel, profiling
    ├── multi_contract   # Token + Vault: Registry, Scenario, MockWallet, regression
    └── defi_vault       # TimeLock: SEP-10, AuthPolicy, CallSequence, snapshot export
```

---

## Installation

Add to your contract's `Cargo.toml`:

```toml
[dev-dependencies]
anvil-core     = { git = "https://github.com/your-org/stellar-anvil" }
anvil-profiler = { git = "https://github.com/your-org/stellar-anvil" }
anvil-multi    = { git = "https://github.com/your-org/stellar-anvil" }
anvil-auth     = { git = "https://github.com/your-org/stellar-anvil" }
```

Install the CLI:

```bash
cargo install --git https://github.com/your-org/stellar-anvil anvil-cli
```

---

## Quick Start

### 1. SandboxEnv — named accounts, time travel, budget tracking

```rust
use anvil_core::SandboxEnv;

let mut sandbox = SandboxEnv::new();

// Named accounts — idempotent, same address on repeated calls
let alice = sandbox.account("alice");
let admin = sandbox.account("admin");

// Deploy and call your contract...
let contract_id = sandbox.env.register(MyContract, ());

// Time travel
sandbox.advance_time(7 * 24 * 3_600);   // 1 week forward
sandbox.warp_to_sequence(5_000);          // jump to ledger seq 5000

// Checkpoint and restore
let cp = sandbox.checkpoint();
sandbox.advance_ledger(500);
sandbox.restore(cp);

// Budget tracking
sandbox.reset_budget();
client.some_fn();
println!("CPU used: {}", sandbox.cpu_used());
```

### 2. Profiler — measure CPU, memory, and fee estimates

```rust
use anvil_profiler::{Profiler, BudgetThreshold};

let mut profiler = Profiler::new(&env).verbose();

profiler.measure("transfer(alice→bob, 1000)", || {
    token_client.transfer(&alice, &bob, &1000_i128)
});
profiler.measure("balance_of(alice)", || {
    token_client.balance(&alice)
});

let report = profiler.into_report("TokenContract");
report.print();                              // colored table to stdout
report.export_markdown("profile.md").ok();  // for PR comments
report.export_json("profile.json").ok();    // for CI comparison

// Assert budget constraints
let threshold = BudgetThreshold::default()
    .max_cpu(5_000_000)
    .max_memory(500_000);
report.assert_within(&threshold);
```

### 3. Regression detection — catch perf regressions in CI

```rust
use anvil_profiler::ProfileComparison;

// Load two saved profile.json files
let baseline  = ProfileReport::load("baseline.json")?;
let candidate = ProfileReport::load("candidate.json")?;

let comparison = ProfileComparison::new("v1 vs v2", &baseline, &candidate);
comparison.print();
comparison.assert_no_regression(10.0); // fail if any call got >10% worse
```

### 4. Scenario — named multi-contract interaction steps

```rust
use anvil_multi::{ContractRegistry, Scenario};

let mut registry = ContractRegistry::new(&env);
registry.add("token", token_id.clone(), &[]);
registry.add("vault", vault_id.clone(), &["token"]);
registry.print_tree();

let mut scenario = Scenario::new("DeFi round-trip", &env, registry).verbose();

scenario.step("alice deposits 2000", |env, reg| {
    let vault = VaultClient::new(env, reg.get("vault").unwrap());
    vault.deposit(&alice, &2_000i128);
});

scenario.step("bob deposits 1000", |env, reg| {
    VaultClient::new(env, reg.get("vault").unwrap()).deposit(&bob, &1_000i128);
});

scenario.step("alice withdraws", |env, reg| {
    let net = VaultClient::new(env, reg.get("vault").unwrap()).withdraw(&alice);
    assert!(net > 0);
});

let results = scenario.run();
Scenario::print_summary("DeFi round-trip", &results);
```

### 5. MockWallet — simulate Freighter states

```rust
use anvil_auth::{WalletBuilder, InvokerMode, MockInvoker};

// Happy path — wallet connected
let mut alice = WalletBuilder::new("alice").with_balance(100).build(&env);
alice.mock_auth(&env);
vault.deposit(&alice.address, &500i128);

// User rejects next transaction
alice.reject_next(1);
// alice.mock_auth(&env) would now panic with a clear rejection message

// Disconnected wallet
let mut bob = WalletBuilder::new("bob").disconnected().build(&env);
// bob.mock_auth(&env) panics: "wallet 'bob' is disconnected"

// DenyAll invoker — test unauthorized access paths
let mut attacker = MockInvoker::new("attacker", &env)
    .with_mode(InvokerMode::DenyAll);
attacker.invoke_expect_fail(&env, || vault.set_fee(&attacker.address, &0u32));
```

### 6. SEP-10 auth simulation

```rust
use anvil_auth::MockSep10Server;

let mut server = MockSep10Server::new().verbose();

// Full happy path
server.assert_auth_flow("GALICE...");

// Test edge cases
let challenge = server.issue_challenge("GBOB...", 0).unwrap(); // 0s = already expired
std::thread::sleep(Duration::from_millis(10));
assert_eq!(
    server.verify_challenge(&challenge, "GBOB...").unwrap_err(),
    Sep10Error::ChallengeExpired
);

// Replay protection
let ch  = server.issue_challenge("GALICE...", 300).unwrap();
let ses = server.verify_challenge(&ch, "GALICE...").unwrap();
server.validate_session(&ses.token).unwrap();             // first use: ok
server.validate_session(&ses.token).unwrap_err();         // second use: TokenReplayed
```

### 7. AuthPolicy — declarative signer assertions

```rust
use anvil_auth::AuthPolicy;

vault.deposit(&alice, &500i128);

AuthPolicy::new(&env)
    .must_have_authorized(&alice)
    .must_not_have_authorized(&admin)
    .assert();
```

### 8. Ledger snapshots

```bash
# Capture a mainnet contract and its dependencies
stellar-anvil snapshot create \
    --address CCONTRACTID... \
    --network mainnet \
    --out snapshot.json

# Inspect what was captured
stellar-anvil snapshot inspect snapshot.json

# Diff two snapshots to see what changed
stellar-anvil snapshot diff snapshot_before.json snapshot_after.json
```

Then in your test:

```rust
// Load snapshot as the starting ledger state
let env = Env::from_ledger_snapshot_file("snapshot.json");
let client = MyContractClient::new(&env, &contract_id);
// now test against real mainnet data
```

---

## CLI Reference

```
stellar-anvil <COMMAND>

Commands:
  test        Run contract tests with rich output
  snapshot    Capture/inspect/diff ledger snapshots
  profile     Profile CPU and memory usage
  compare     Compare two profile reports for regressions
  info        Print environment info (stellar-cli, rustc, cargo versions)

Options:
  -v, --verbose    Enable verbose output
  -h, --help       Print help
      --version    Print version
```

### `stellar-anvil test`

```bash
# Run all tests
stellar-anvil test

# Filter by name
stellar-anvil test --filter vault

# JSON output (for CI)
stellar-anvil test --format json --output results.json

# TAP output
stellar-anvil test --format tap

# Show stdout/stderr from passing tests
stellar-anvil test --nocapture
```

### `stellar-anvil profile`

```bash
# Profile all tests
stellar-anvil profile

# Export reports
stellar-anvil profile --report profile.json --markdown profile.md

# Fail CI if any call exceeds limits
stellar-anvil profile \
    --max-cpu 5000000 \
    --max-memory 500000 \
    --max-total-cpu 20000000
```

### `stellar-anvil compare`

```bash
# Compare baseline vs candidate, fail if >10% regression
stellar-anvil compare baseline.json candidate.json --threshold 10

# Export Markdown for PR comment
stellar-anvil compare baseline.json candidate.json --markdown regression.md
```

---

## Running the Examples

```bash
# Counter — basic features
cargo test -p counter --features testutils

# Multi-contract — token + vault, scenarios, wallet mocks
cargo test -p multi-contract --features testutils

# DeFi vault — time-lock, SEP-10, auth policy, call sequence
cargo test -p defi-vault --features testutils

# Run all examples at once
cargo test --features testutils
```

---

## Contributing

Stellar Anvil is enrolled in the **Stellar Wave Program** on Drips. Contributions earn points towards the reward pool. Good first issues are labeled `good first issue` in the tracker.

**Development setup:**

```bash
# Clone
git clone https://github.com/your-org/stellar-anvil
cd stellar-anvil

# Install stellar-cli (required for snapshot commands)
cargo install --locked stellar-cli

# Build all crates
cargo build

# Run all tests
cargo test --features testutils

# Install the CLI locally
cargo install --path crates/anvil-cli
```

**Before submitting a PR:**

```bash
cargo fmt --all
cargo clippy --all-targets --features testutils -- -D warnings
cargo test --all --features testutils
```

---

## Roadmap

- [ ] `anvil fuzz` — property-based fuzzing wrapper over soroban's built-in fuzzer
- [ ] `anvil coverage` — line coverage report for Soroban contracts
- [ ] Snapshot diffing with XDR-aware field decoding
- [ ] VS Code extension for inline profile annotations
- [ ] GitHub Actions workflow template for CI integration
- [ ] `anvil watch` — re-run affected tests on file save

---

## License

MIT — see [LICENSE](LICENSE).

---

## Acknowledgements

Built on top of the excellent [soroban-sdk](https://github.com/stellar/rs-soroban-sdk) testutils, the [Stellar CLI](https://github.com/stellar/stellar-cli), and inspired by [Foundry](https://github.com/foundry-rs/foundry) and [Hardhat](https://hardhat.org).

Participating in the [Stellar Wave Program](https://drips.network) — fund open-source Stellar infrastructure at [drips.network](https://drips.network).
