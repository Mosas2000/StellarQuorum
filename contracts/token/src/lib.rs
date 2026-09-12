#![no_std]
//! QUORUM — governance token for the Quorum protocol. SEP-41 compatible.
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, String};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TokenError {
    AlreadyInitialized    = 1,
    Unauthorized          = 2,
    InsufficientBalance   = 3,
    InsufficientAllowance = 4,
    InvalidAmount         = 5,
    Overflow              = 6,
}

#[contracttype]
pub enum DataKey {
    Admin, Name, Symbol, Decimals, TotalSupply,
    Balance(Address),
    Allowance(Address, Address),
}

#[contract]
pub struct QuorumToken;

#[contractimpl]
impl QuorumToken {
    pub fn initialize(env: Env, admin: Address, name: String, symbol: String, decimals: u32, initial_supply: i128) -> Result<(), TokenError> {
        if env.storage().instance().has(&DataKey::Admin) { return Err(TokenError::AlreadyInitialized); }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
        env.storage().instance().set(&DataKey::TotalSupply, &initial_supply);
        env.storage().persistent().set(&DataKey::Balance(admin), &initial_supply);
        Ok(())
    }

    pub fn balance(env: Env, owner: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Balance(owner)).unwrap_or(0)
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        from.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let from_bal = Self::balance(env.clone(), from.clone());
        if from_bal < amount { return Err(TokenError::InsufficientBalance); }
        env.storage().persistent().set(&DataKey::Balance(from), &(from_bal - amount));
        let to_bal = Self::balance(env.clone(), to.clone());
        env.storage().persistent().set(&DataKey::Balance(to), &(to_bal + amount));
        Ok(())
    }

    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), TokenError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let supply: i128 = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TotalSupply, &(supply + amount));
        let bal = Self::balance(env.clone(), to.clone());
        env.storage().persistent().set(&DataKey::Balance(to), &(bal + amount));
        Ok(())
    }

    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        from.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let bal = Self::balance(env.clone(), from.clone());
        if bal < amount { return Err(TokenError::InsufficientBalance); }
        env.storage().persistent().set(&DataKey::Balance(from), &(bal - amount));
        let supply = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TotalSupply, &(supply - amount));
        Ok(())
    }

    pub fn approve(env: Env, owner: Address, spender: Address, amount: i128) -> Result<(), TokenError> {
        owner.require_auth();
        env.storage().persistent().set(&DataKey::Allowance(owner.clone(), spender.clone()), &amount);
        Ok(())
    }

    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Allowance(owner, spender)).unwrap_or(0)
    }

    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), TokenError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        Ok(())
    }

    pub fn name(env: Env) -> String { env.storage().instance().get(&DataKey::Name).unwrap() }
    pub fn symbol(env: Env) -> String { env.storage().instance().get(&DataKey::Symbol).unwrap() }
    pub fn decimals(env: Env) -> u32 { env.storage().instance().get(&DataKey::Decimals).unwrap() }

    // TODO: implement delegate() for voting power delegation
    // TODO: implement get_past_votes(address, ledger) for snapshot governance
}
