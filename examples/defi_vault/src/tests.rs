#![cfg(test)]

use anvil_core::{SandboxEnv, AnvilAssert};
use anvil_auth::{MockSep10Server, WalletBuilder, AuthPolicy};
use anvil_multi::CallSequence;
use anvil_profiler::Profiler;
use crate::{TimeLockVault, TimeLockVaultClient};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn deploy(sandbox: &SandboxEnv) -> (soroban_sdk::Address, TimeLockVaultClient) {
    let env = &sandbox.env;
    let _admin = sandbox.env.mock_all_auths_returning_defaults();
    let contract_id = env.register(TimeLockVault, ());
    let client = TimeLockVaultClient::new(env, &contract_id);
    (contract_id, client)
}

// ─────────────────────────────────────────────────────────────────────────────
// Time-lock: deposit and early-withdraw should fail
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_early_withdrawal_rejected() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    sandbox.env.mock_all_auths();

    let (contract_id, client) = deploy(&sandbox);
    client.init(&admin.address, &100u32); // 1% fee

    let now = sandbox.current_timestamp();
    let release = now + 7 * 24 * 3_600; // 1 week from now

    client.deposit(&alice.address, &1_000i128, &release);
    AnvilAssert::eq("alice deposited", 1_000i128, client.balance_of(&alice.address));

    // Advance time — but NOT enough to unlock
    sandbox.advance_time(3 * 24 * 3_600); // 3 days

    // This should panic because still locked
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.withdraw(&alice.address)
    }));
    AnvilAssert::is_true("early withdraw rejected", result.is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Time-lock: time-travel past lock → withdraw succeeds
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_withdrawal_after_lock_period() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &100u32); // 1% fee

    let now = sandbox.current_timestamp();
    let release = now + 3_600; // 1 hour

    client.deposit(&alice.address, &2_000i128, &release);

    // Travel 1 hour + 1 second past lock
    sandbox.advance_time(3_601);

    let net = client.withdraw(&alice.address);
    // 1% fee on 2000 = 20, net = 1980
    AnvilAssert::eq("net withdrawal after lock", 1_980i128, net);
    AnvilAssert::eq("alice vault balance cleared", 0i128, client.balance_of(&alice.address));
}

// ─────────────────────────────────────────────────────────────────────────────
// Pause / unpause
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_pause_blocks_deposits() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &0u32);

    client.pause(&admin.address);
    AnvilAssert::is_true("vault is paused", client.is_paused());

    let now = sandbox.current_timestamp();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.deposit(&alice.address, &500i128, &(now + 3_600))
    }));
    AnvilAssert::is_true("deposit while paused fails", result.is_err());

    client.unpause(&admin.address);
    AnvilAssert::is_true("vault unpaused", !client.is_paused());

    // Deposit now succeeds
    client.deposit(&alice.address, &500i128, &(sandbox.current_timestamp() + 3_600));
    AnvilAssert::eq("deposit after unpause", 500i128, client.balance_of(&alice.address));
}

#[test]
#[should_panic]
fn test_non_admin_cannot_pause() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let attacker = sandbox.account("attacker");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &0u32);
    client.pause(&attacker.address); // should panic
}

// ─────────────────────────────────────────────────────────────────────────────
// SEP-10 auth flow simulation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sep10_full_auth_flow() {
    let mut server = MockSep10Server::new().verbose();
    server.assert_auth_flow("GALICE1234567890XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
}

#[test]
fn test_sep10_expired_challenge() {
    use anvil_auth::Sep10Error;

    let mut server = MockSep10Server::new();
    // Issue a challenge with 0s timeout — already expired
    let challenge = server.issue_challenge("GTEST...", 0).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let result = server.verify_challenge(&challenge, "GTEST...");
    assert_eq!(result.unwrap_err(), Sep10Error::ChallengeExpired);
}

#[test]
fn test_sep10_address_mismatch() {
    use anvil_auth::Sep10Error;

    let mut server = MockSep10Server::new();
    let challenge = server.issue_challenge("GALICE...", 300).unwrap();

    // Bob tries to use Alice's challenge
    let result = server.verify_challenge(&challenge, "GBOB...");
    assert_eq!(result.unwrap_err(), Sep10Error::AddressMismatch);
}

#[test]
fn test_sep10_replay_protection() {
    use anvil_auth::Sep10Error;

    let mut server = MockSep10Server::new();
    let challenge = server.issue_challenge("GALICE...", 300).unwrap();
    let session = server.verify_challenge(&challenge, "GALICE...").unwrap();

    // First use succeeds
    server.validate_session(&session.token).unwrap();

    // Second use — replay attack — should fail
    let result = server.validate_session(&session.token);
    assert_eq!(result.unwrap_err(), Sep10Error::TokenReplayed);
}

// ─────────────────────────────────────────────────────────────────────────────
// MockWallet — connect/disconnect/reject flows
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wallet_connects_and_signs() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &0u32);

    let mut alice_wallet = WalletBuilder::new("alice")
        .with_balance(50)
        .build(&sandbox.env);

    alice_wallet.mock_auth(&sandbox.env);
    let now = sandbox.current_timestamp();
    client.deposit(&alice_wallet.address, &1_000i128, &(now + 3_600));

    alice_wallet.print_summary();
    AnvilAssert::eq(
        "alice wallet deposit recorded",
        1_000i128,
        client.balance_of(&alice_wallet.address),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AuthPolicy — assert correct signers after vault operations
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_auth_policy_correct_signer() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &0u32);

    let now = sandbox.current_timestamp();
    client.deposit(&alice.address, &500i128, &(now + 3_600));

    AuthPolicy::new(&sandbox.env)
        .must_have_authorized(&alice.address)
        .must_not_have_authorized(&admin.address)
        .assert();
}

// ─────────────────────────────────────────────────────────────────────────────
// CallSequence — verify order independence of read-only calls
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_read_calls_are_order_independent() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &0u32);

    let now = sandbox.current_timestamp();
    client.deposit(&alice.address, &1_000i128, &(now + 3_600));

    let mut seq = CallSequence::new(&sandbox.env);
    let client_ref = &client;
    let alice_addr = alice.address.clone();

    seq.call("balance_of(alice)", move || {
        let _ = client_ref.balance_of(&alice_addr);
    });
    seq.call("total_deposits()", move || {
        let _ = client_ref.total_deposits();
    });
    seq.call("is_paused()", move || {
        let _ = client_ref.is_paused();
    });

    seq.replay();
}

// ─────────────────────────────────────────────────────────────────────────────
// Full profile run
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_time_lock_vault_profile() {
    let mut sandbox = SandboxEnv::new();
    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    sandbox.env.mock_all_auths();

    let (_, client) = deploy(&sandbox);
    client.init(&admin.address, &100u32);

    let now = sandbox.current_timestamp();
    let env = &sandbox.env;
    let mut profiler = Profiler::new(env).verbose();

    profiler.measure("vault::init(admin, 100bps)", || {
        client.is_paused()
    });

    profiler.measure("vault::deposit(alice, 1000, +1h)", || {
        client.deposit(&alice.address, &1_000i128, &(now + 3_600))
    });

    profiler.measure("vault::balance_of(alice)", || {
        client.balance_of(&alice.address)
    });

    profiler.measure("vault::total_deposits()", || {
        client.total_deposits()
    });

    sandbox.advance_time(3_601);

    profiler.measure("vault::withdraw(alice)", || {
        client.withdraw(&alice.address)
    });

    let report = profiler.into_report("TimeLockVault");
    report.print();
    report.export_json("timelock_vault_profile.json").ok();
    report.export_markdown("timelock_vault_profile.md").ok();
}
