use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum VaultKey {
    Deposit(Address),
    TokenContract,
    Admin,
    FeeRate, // basis points, e.g. 50 = 0.5%
    TotalDeposits,
}

/// A simple yield vault that holds tokens and charges a withdrawal fee.
/// Depends on a `TokenContract` for transfers.
#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn init(env: Env, admin: Address, token: Address, fee_bps: u32) {
        admin.require_auth();
        env.storage().instance().set(&VaultKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&VaultKey::TokenContract, &token);
        env.storage().instance().set(&VaultKey::FeeRate, &fee_bps);
        env.storage()
            .instance()
            .set(&VaultKey::TotalDeposits, &0i128);
    }

    /// Deposit `amount` tokens into the vault on behalf of `depositor`.
    /// In a real contract this would cross-call the token contract.
    /// Here we track balances internally to keep the example self-contained.
    pub fn deposit(env: Env, depositor: Address, amount: i128) {
        depositor.require_auth();
        assert!(amount > 0, "amount must be positive");

        let current: i128 = env
            .storage()
            .instance()
            .get(&VaultKey::Deposit(depositor.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&VaultKey::Deposit(depositor), &(current + amount));

        let total: i128 = env
            .storage()
            .instance()
            .get(&VaultKey::TotalDeposits)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&VaultKey::TotalDeposits, &(total + amount));
    }

    /// Withdraw all tokens for `depositor`, applying the fee.
    /// Returns the net amount after fee deduction.
    pub fn withdraw(env: Env, depositor: Address) -> i128 {
        depositor.require_auth();

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

        // Clear the deposit
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

    pub fn total_deposits(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&VaultKey::TotalDeposits)
            .unwrap_or(0)
    }

    pub fn set_fee(env: Env, admin: Address, fee_bps: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&VaultKey::Admin).unwrap();
        assert_eq!(admin, stored_admin, "not admin");
        assert!(fee_bps <= 1000, "fee cannot exceed 10%");
        env.storage().instance().set(&VaultKey::FeeRate, &fee_bps);
    }

    pub fn fee_rate(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&VaultKey::FeeRate)
            .unwrap_or(0)
    }
}
