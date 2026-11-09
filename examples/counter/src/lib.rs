use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

/// Simple storage key
#[contracttype]
pub enum DataKey {
    Count,
    Admin,
}

/// A pausable counter contract.
///
/// - Only the admin can reset the counter.
/// - Anyone can call `increment`.
/// - The admin can pause/unpause to block increments.
#[contract]
pub struct CounterContract;

#[contractimpl]
impl CounterContract {
    /// Initialize with an admin address.
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Count, &0u32);
    }

    /// Increment the counter by 1. Returns the new value.
    pub fn increment(env: Env) -> u32 {
        let count: u32 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let new = count + 1;
        env.storage().instance().set(&DataKey::Count, &new);
        new
    }

    /// Increment by a custom step. Admin only.
    pub fn increment_by(env: Env, caller: Address, step: u32) -> u32 {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        assert_eq!(caller, admin, "only admin can use increment_by");
        let count: u32 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let new = count + step;
        env.storage().instance().set(&DataKey::Count, &new);
        new
    }

    /// Reset counter to 0. Admin only.
    pub fn reset(env: Env, caller: Address) {
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        assert_eq!(caller, admin, "only admin can reset");
        env.storage().instance().set(&DataKey::Count, &0u32);
    }

    /// Read the current value.
    pub fn value(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Count).unwrap_or(0)
    }

    /// Read the admin address.
    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }
}
