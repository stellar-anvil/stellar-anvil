use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

/// Storage keys for the time-locked vault.
#[contracttype]
pub enum VaultKey {
    Deposit(Address),
    LockUntil(Address), // timestamp after which withdrawal is allowed
    Admin,
    FeeRate,
    TotalDeposits,
    Paused,
}

/// A time-locked vault. Deposits are locked until a release timestamp.
/// Demonstrates time-travel tests and auth policy checks.
#[contract]
pub struct TimeLockVault;

#[contractimpl]
impl TimeLockVault {
    pub fn init(env: Env, admin: Address, fee_bps: u32) {
        admin.require_auth();
        env.storage().instance().set(&VaultKey::Admin, &admin);
        env.storage().instance().set(&VaultKey::FeeRate, &fee_bps);
        env.storage().instance().set(&VaultKey::TotalDeposits, &0i128);
        env.storage().instance().set(&VaultKey::Paused, &false);
    }

    /// Deposit `amount` tokens locked until `release_ts` (Unix timestamp).
    pub fn deposit(env: Env, depositor: Address, amount: i128, release_ts: u64) {
        depositor.require_auth();
        assert!(amount > 0, "amount must be positive");
        assert!(
            release_ts > env.ledger().timestamp(),
            "release_ts must be in the future"
        );

        let paused: bool = env
            .storage()
            .instance()
            .get(&VaultKey::Paused)
            .unwrap_or(false);
        assert!(!paused, "vault is paused");

        let current: i128 = env
            .storage()
            .instance()
            .get(&VaultKey::Deposit(depositor.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&VaultKey::Deposit(depositor.clone()), &(current + amount));
        env.storage()
            .instance()
            .set(&VaultKey::LockUntil(depositor), &release_ts);

        let total: i128 = env
            .storage()
            .instance()
            .get(&VaultKey::TotalDeposits)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&VaultKey::TotalDeposits, &(total + amount));
    }

    /// Withdraw tokens once the lock period has passed.
    pub fn withdraw(env: Env, depositor: Address) -> i128 {
        depositor.require_auth();

        let lock_until: u64 = env
            .storage()
            .instance()
            .get(&VaultKey::LockUntil(depositor.clone()))
            .unwrap_or(0);

        assert!(
            env.ledger().timestamp() >= lock_until,
            "funds are still locked"
        );

        let amount: i128 = env
            .storage()
            .instance()
            .get(&VaultKey::Deposit(depositor.clone()))
            .unwrap_or(0);
        assert!(amount > 0, "nothing to withdraw");

        let fee_bps: u32 = env
            .storage()
            .instance()
            .get(&VaultKey::FeeRate)
            .unwrap_or(0);
        let fee = (amount * fee_bps as i128) / 10_000;
        let net = amount - fee;

        env.storage()
            .instance()
            .set(&VaultKey::Deposit(depositor.clone()), &0i128);

        let total: i128 = env
            .storage()
            .instance()
            .get(&VaultKey::TotalDeposits)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&VaultKey::TotalDeposits, &(total - amount));

        net
    }

    pub fn balance_of(env: Env, depositor: Address) -> i128 {
        env.storage()
            .instance()
            .get(&VaultKey::Deposit(depositor))
            .unwrap_or(0)
    }

    pub fn lock_until(env: Env, depositor: Address) -> u64 {
        env.storage()
            .instance()
            .get(&VaultKey::LockUntil(depositor))
            .unwrap_or(0)
    }

    pub fn total_deposits(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&VaultKey::TotalDeposits)
            .unwrap_or(0)
    }

    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&VaultKey::Admin).unwrap();
        assert_eq!(admin, stored_admin, "not admin");
        env.storage().instance().set(&VaultKey::Paused, &true);
    }

    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&VaultKey::Admin).unwrap();
        assert_eq!(admin, stored_admin, "not admin");
        env.storage().instance().set(&VaultKey::Paused, &false);
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&VaultKey::Paused)
            .unwrap_or(false)
    }
}
