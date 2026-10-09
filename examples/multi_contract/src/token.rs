use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum TokenKey {
    Balance(Address),
    Admin,
    TotalSupply,
}

/// Minimal fungible token contract.
#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn init(env: Env, admin: Address, initial_supply: i128) {
        admin.require_auth();
        env.storage().instance().set(&TokenKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&TokenKey::Balance(admin.clone()), &initial_supply);
        env.storage()
            .instance()
            .set(&TokenKey::TotalSupply, &initial_supply);
    }

    pub fn balance(env: Env, address: Address) -> i128 {
        env.storage()
            .instance()
            .get(&TokenKey::Balance(address))
            .unwrap_or(0)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&TokenKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        assert!(amount > 0, "amount must be positive");

        let from_bal = Self::balance(env.clone(), from.clone());
        assert!(from_bal >= amount, "insufficient balance");

        let to_bal = Self::balance(env.clone(), to.clone());

        env.storage()
            .instance()
            .set(&TokenKey::Balance(from), &(from_bal - amount));
        env.storage()
            .instance()
            .set(&TokenKey::Balance(to), &(to_bal + amount));
    }

    /// Mint new tokens. Admin only.
    pub fn mint(env: Env, admin: Address, to: Address, amount: i128) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&TokenKey::Admin).unwrap();
        assert_eq!(admin, stored_admin, "not admin");

        let bal = Self::balance(env.clone(), to.clone());
        env.storage()
            .instance()
            .set(&TokenKey::Balance(to), &(bal + amount));

        let supply: i128 = env
            .storage()
            .instance()
            .get(&TokenKey::TotalSupply)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&TokenKey::TotalSupply, &(supply + amount));
    }
}
