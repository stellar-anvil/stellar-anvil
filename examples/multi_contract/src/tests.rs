#![cfg(test)]

use soroban_sdk::Env;
use anvil_core::{SandboxEnv, AnvilAssert};
use anvil_multi::{ContractRegistry, Scenario};
use anvil_auth::{MockWallet, WalletBuilder, MockInvoker, InvokerMode, AuthPolicy};
use anvil_profiler::{Profiler, BudgetThreshold, ProfileComparison};

use crate::token::{TokenContract, TokenContractClient};
use crate::vault::{VaultContract, VaultContractClient};

// ─────────────────────────────────────────────────────────────────────────────
// Test setup
// ─────────────────────────────────────────────────────────────────────────────

struct TestEnv {
    sandbox: SandboxEnv,
    token_id: soroban_sdk::Address,
    vault_id: soroban_sdk::Address,
    admin: anvil_core::accounts::Account,
    alice: anvil_core::accounts::Account,
    bob: anvil_core::accounts::Account,
}

fn setup() -> TestEnv {
    let mut sandbox = SandboxEnv::new();

    let admin = sandbox.account("admin");
    let alice = sandbox.account("alice");
    let bob   = sandbox.account("bob");

    let env = &sandbox.env;
    env.mock_all_auths();

    // Deploy token
    let token_id = env.register(TokenContract, ());
    let token    = TokenContractClient::new(env, &token_id);
    token.init(&admin.address, &1_000_000i128);

    // Mint to alice and bob
    token.mint(&admin.address, &alice.address, &10_000i128);
    token.mint(&admin.address, &bob.address,   &5_000i128);

    // Deploy vault (50 bps = 0.5% fee)
    let vault_id = env.register(VaultContract, ());
    let vault    = VaultContractClient::new(env, &vault_id);
    vault.init(&admin.address, &token_id, &50u32);

    // Register in sandbox
    sandbox.register_contract("token", token_id.to_string());
    sandbox.register_contract("vault", vault_id.to_string());

    TestEnv { sandbox, token_id, vault_id, admin, alice, bob }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_token_balances_after_mint() {
    let t = setup();
    let env = &t.sandbox.env;
    let token = TokenContractClient::new(env, &t.token_id);

    AnvilAssert::eq("alice balance", 10_000i128, token.balance(&t.alice.address));
    AnvilAssert::eq("bob balance",    5_000i128, token.balance(&t.bob.address));
}

#[test]
fn test_token_transfer() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();
    let token = TokenContractClient::new(env, &t.token_id);

    token.transfer(&t.alice.address, &t.bob.address, &1_000i128);

    AnvilAssert::eq("alice after transfer", 9_000i128, token.balance(&t.alice.address));
    AnvilAssert::eq("bob after transfer",   6_000i128, token.balance(&t.bob.address));
}

#[test]
fn test_vault_deposit_and_withdraw() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();
    let vault = VaultContractClient::new(env, &t.vault_id);

    vault.deposit(&t.alice.address, &2_000i128);
    AnvilAssert::eq("vault holds alice deposit", 2_000i128, vault.balance_of(&t.alice.address));
    AnvilAssert::eq("total deposits", 2_000i128, vault.total_deposits());

    // 0.5% fee on 2000 = 10, net = 1990
    let net = vault.withdraw(&t.alice.address);
    AnvilAssert::eq("net withdrawal (after 0.5% fee)", 1_990i128, net);
    AnvilAssert::eq("alice vault balance after withdraw", 0i128, vault.balance_of(&t.alice.address));
}

#[test]
fn test_vault_fee_rate_update_by_admin() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();
    let vault = VaultContractClient::new(env, &t.vault_id);

    vault.set_fee(&t.admin.address, &100u32); // 1%
    AnvilAssert::eq("new fee rate", 100u32, vault.fee_rate());
}

#[test]
#[should_panic]
fn test_vault_fee_rate_update_by_non_admin_fails() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();
    let vault = VaultContractClient::new(env, &t.vault_id);
    vault.set_fee(&t.alice.address, &0u32);
}

#[test]
#[should_panic]
fn test_vault_fee_cannot_exceed_10_percent() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();
    let vault = VaultContractClient::new(env, &t.vault_id);
    vault.set_fee(&t.admin.address, &1001u32); // > 1000 bps
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractRegistry + dependency tree
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_contract_registry_tree() {
    let t = setup();
    let env = &t.sandbox.env;

    let mut registry = ContractRegistry::new(env);
    registry.add("token", t.token_id.clone(), &[]);
    registry.add("vault", t.vault_id.clone(), &["token"]);
    registry.set_version("vault", "0.1.0");

    registry.print_tree();

    let order = registry.deployment_order().expect("no cycles");
    assert_eq!(order[0], "token", "token must come before vault");
    assert_eq!(order[1], "vault");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario — full DeFi round-trip
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_defi_scenario() {
    let t = setup();
    let env = t.sandbox.env.clone();
    env.mock_all_auths();

    let vault_id   = t.vault_id.clone();
    let alice_addr = t.alice.address.clone();
    let bob_addr   = t.bob.address.clone();

    let mut registry = ContractRegistry::new(&env);
    registry.add("token", t.token_id.clone(), &[]);
    registry.add("vault", vault_id.clone(),   &["token"]);

    let mut scenario = Scenario::new("DeFi round-trip", &env, registry).verbose();

    let vault_id_2  = vault_id.clone();
    let alice_addr2 = alice_addr.clone();
    let bob_addr2   = bob_addr.clone();
    let vault_id_3  = vault_id.clone();
    let alice_addr3 = alice_addr.clone();
    let vault_id_4  = vault_id.clone();
    let bob_addr3   = bob_addr.clone();
    let vault_id_5  = vault_id.clone();
    let alice_addr4 = alice_addr.clone();

    scenario.step("alice deposits 2000", move |env, _reg| {
        let vault = VaultContractClient::new(env, &vault_id_2);
        vault.deposit(&alice_addr2, &2_000i128);
    });

    scenario.step("bob deposits 1000", move |env, _reg| {
        let vault = VaultContractClient::new(env, &vault_id_3);
        vault.deposit(&bob_addr2, &1_000i128);
    });

    scenario.step("total deposits = 3000", move |env, _reg| {
        let vault = VaultContractClient::new(env, &vault_id_4);
        let total = vault.total_deposits();
        assert_eq!(total, 3_000i128, "expected 3000 total deposits");
    });

    scenario.step("alice withdraws (gets 1990)", move |env, _reg| {
        let vault = VaultContractClient::new(env, &vault_id_5);
        let net = vault.withdraw(&alice_addr4);
        assert_eq!(net, 1_990i128, "expected 1990 net");
    });

    scenario.step_fails("withdraw with zero balance panics", move |env, _reg| {
        let vault = VaultContractClient::new(env, &vault_id.clone());
        vault.withdraw(&bob_addr3);
    });

    let results = scenario.run();
    Scenario::print_summary("DeFi round-trip", &results);
}

// ─────────────────────────────────────────────────────────────────────────────
// MockWallet auth tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_mock_wallet_approve_flow() {
    let t = setup();
    let env = &t.sandbox.env;

    let mut alice_wallet = WalletBuilder::new("alice")
        .with_balance(100)
        .build(env);

    alice_wallet.mock_auth(env);

    let vault = VaultContractClient::new(env, &t.vault_id);
    vault.deposit(&alice_wallet.address, &500i128);

    alice_wallet.print_summary();
    AnvilAssert::eq("alice wallet vault balance", 500i128, vault.balance_of(&alice_wallet.address));
}

#[test]
#[should_panic(expected = "rejected the authorization request")]
fn test_mock_wallet_reject_flow() {
    let t = setup();
    let env = &t.sandbox.env;

    let mut alice_wallet = WalletBuilder::new("alice").build(env);
    alice_wallet.reject_next(1);

    alice_wallet.mock_auth(env);
}

#[test]
#[should_panic(expected = "disconnected")]
fn test_mock_wallet_disconnected_flow() {
    let t = setup();
    let env = &t.sandbox.env;

    let mut alice_wallet = WalletBuilder::new("alice").disconnected().build(env);
    alice_wallet.mock_auth(env);
}

// ─────────────────────────────────────────────────────────────────────────────
// MockInvoker tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invoker_approve_all() {
    let t = setup();
    let env = &t.sandbox.env;

    let mut invoker = MockInvoker::new("alice-invoker", env);
    invoker.address = t.alice.address.clone();

    let vault = VaultContractClient::new(env, &t.vault_id);
    invoker.invoke(env, || vault.deposit(&t.alice.address, &300i128));

    AnvilAssert::eq("alice vault deposit", 300i128, vault.balance_of(&t.alice.address));
    invoker.print_summary();
}

#[test]
fn test_invoker_deny_all() {
    let t = setup();
    let env = &t.sandbox.env;

    let mut attacker = MockInvoker::new("attacker", env)
        .with_mode(InvokerMode::DenyAll);

    let vault = VaultContractClient::new(env, &t.vault_id);

    attacker.invoke_expect_fail(env, || {
        vault.set_fee(&attacker.address, &0u32);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiler + regression comparison
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_vault_profile_and_thresholds() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();

    let vault = VaultContractClient::new(env, &t.vault_id);
    let mut profiler = Profiler::new(env).verbose();

    profiler.measure("vault::deposit(alice, 1000)", || {
        vault.deposit(&t.alice.address, &1_000i128)
    });
    profiler.measure("vault::balance_of(alice)", || {
        vault.balance_of(&t.alice.address)
    });
    profiler.measure("vault::withdraw(alice)", || {
        vault.withdraw(&t.alice.address)
    });
    profiler.measure("vault::total_deposits()", || {
        vault.total_deposits()
    });

    let report = profiler.into_report("VaultContract");
    report.print();

    let threshold = BudgetThreshold::default()
        .max_cpu(10_000_000)
        .max_memory(1_000_000);

    report.assert_within(&threshold);
}

#[test]
fn test_no_regression_between_baseline_and_candidate() {
    let t = setup();
    let env = &t.sandbox.env;
    env.mock_all_auths();

    let vault = VaultContractClient::new(env, &t.vault_id);

    let mut baseline_profiler = Profiler::new(env);
    baseline_profiler.measure("vault::deposit(500)", || vault.deposit(&t.alice.address, &500i128));
    baseline_profiler.measure("vault::balance_of(alice)", || vault.balance_of(&t.alice.address));
    let baseline = baseline_profiler.into_report("baseline");

    let t2 = setup();
    let env2 = &t2.sandbox.env;
    env2.mock_all_auths();
    let vault2 = VaultContractClient::new(env2, &t2.vault_id);

    let mut cand_profiler = Profiler::new(env2);
    cand_profiler.measure("vault::deposit(500)", || vault2.deposit(&t2.alice.address, &500i128));
    cand_profiler.measure("vault::balance_of(alice)", || vault2.balance_of(&t2.alice.address));
    let candidate = cand_profiler.into_report("candidate");

    let comparison = ProfileComparison::new("vault v0.1 vs v0.1 (no change)", &baseline, &candidate);
    comparison.print();
    comparison.assert_no_regression(5.0);
}
