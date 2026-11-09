#![cfg(test)]

use soroban_sdk::Env;
use anvil_core::{SandboxEnv, AnvilAssert};
use anvil_profiler::{Profiler, BudgetThreshold};
use crate::{CounterContract, CounterContractClient};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn setup() -> (SandboxEnv, CounterContractClient<'static>) {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");

    let env = sandbox.env.clone();
    let contract_id = env.register(CounterContract, ());

    // Leak env for 'static lifetime required by generated client
    let env_ref: &'static Env = Box::leak(Box::new(env.clone()));
    let client = CounterContractClient::new(env_ref, &contract_id);

    env.mock_all_auths();
    client.init(&admin.address);

    (sandbox, client)
}

// ─────────────────────────────────────────────────────────────────────────────
// Basic functionality
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_initial_value_is_zero() {
    let (_sandbox, client) = setup();
    AnvilAssert::eq("initial counter", 0u32, client.value());
}

#[test]
fn test_increment_increases_by_one() {
    let (_sandbox, client) = setup();
    AnvilAssert::eq("after first increment", 1u32, client.increment());
    AnvilAssert::eq("after second increment", 2u32, client.increment());
    AnvilAssert::eq("stored value", 2u32, client.value());
}

#[test]
fn test_increment_by_admin_step() {
    let (mut sandbox, client) = setup();
    let admin = sandbox.account("admin");
    sandbox.env.mock_all_auths();

    let result = client.increment_by(&admin.address, &10u32);
    AnvilAssert::eq("increment_by(10)", 10u32, result);
}

#[test]
fn test_reset_by_admin() {
    let (mut sandbox, client) = setup();
    let admin = sandbox.account("admin");
    sandbox.env.mock_all_auths();

    client.increment();
    client.increment();
    AnvilAssert::eq("before reset", 2u32, client.value());

    client.reset(&admin.address);
    AnvilAssert::eq("after reset", 0u32, client.value());
}

#[test]
#[should_panic]
fn test_reset_by_non_admin_fails() {
    let (mut sandbox, client) = setup();
    let attacker = sandbox.account("attacker");
    sandbox.env.mock_all_auths();
    client.reset(&attacker.address);
}

// ─────────────────────────────────────────────────────────────────────────────
// Time-travel
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_value_persists_across_ledger_advance() {
    let (mut sandbox, client) = setup();

    client.increment();
    client.increment();

    // Advance 1000 ledger sequences (~83 minutes)
    sandbox.advance_ledger(1000);

    AnvilAssert::eq("value after ledger advance", 2u32, client.value());
    AnvilAssert::eq("sequence number", 1100u32, sandbox.current_sequence());
}

#[test]
fn test_warp_to_future_timestamp() {
    let (mut sandbox, client) = setup();

    let original_ts = sandbox.current_timestamp();
    sandbox.warp_to_timestamp(original_ts + 3_600); // 1 hour forward

    client.increment();
    AnvilAssert::eq("value after time warp", 1u32, client.value());
    AnvilAssert::is_true(
        "timestamp advanced",
        sandbox.current_timestamp() > original_ts,
    );
}

#[test]
fn test_checkpoint_and_restore() {
    let (mut sandbox, client) = setup();

    client.increment();
    client.increment();
    AnvilAssert::eq("before checkpoint", 2u32, client.value());

    let checkpoint = sandbox.checkpoint();
    sandbox.advance_ledger(500);

    // Restore — ledger goes back, but Soroban env state is in-memory
    // so we're primarily testing that restore() doesn't crash and
    // the checkpoint fields are correct
    sandbox.restore(checkpoint);
    AnvilAssert::eq("sequence after restore", 100u32, sandbox.current_sequence());
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiling
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_profiled_invocations() {
    let (_sandbox, client) = setup();
    let env = client.env.clone();

    let mut profiler = Profiler::new(&env).verbose();

    profiler.measure("counter::increment()", || client.increment());
    profiler.measure("counter::increment()", || client.increment());
    profiler.measure("counter::value()", || client.value());

    let report = profiler.into_report("CounterContract");
    report.print();

    // Assert each call stays well within budget
    let threshold = BudgetThreshold::default()
        .max_cpu(5_000_000)
        .max_memory(500_000);

    report.assert_within(&threshold);
}
