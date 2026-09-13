#![no_std]
//! QUORUM — governance token for the Quorum protocol. SEP-41 compatible.
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, String, Vec};

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

/// A balance recorded at the ledger on which it changed.
///
/// Checkpoints are what make snapshot governance possible: a proposal records
/// the ledger it was created at, and voting power is read from the balance as
/// of that ledger rather than the live balance. Tokens acquired after the
/// snapshot therefore carry no weight, which is what blocks flash-loan and
/// buy-the-vote attacks.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub ledger: u32,
    pub balance: i128,
}

#[contracttype]
pub enum DataKey {
    Admin, Name, Symbol, Decimals, TotalSupply,
    Balance(Address),
    Allowance(Address, Address),
    Checkpoints(Address),
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
        Self::set_balance(&env, &admin, initial_supply);
        Ok(())
    }

    /// Balance of `owner` as of the end of `ledger`.
    ///
    /// Returns the most recent checkpoint at or before `ledger`, or 0 if the
    /// address held nothing that far back. Governance reads voting power
    /// through this, using a proposal's `snapshot_ledger`.
    pub fn get_past_balance(env: Env, owner: Address, ledger: u32) -> i128 {
        let checkpoints = Self::checkpoints(&env, &owner);

        // Binary search for the first checkpoint recorded after `ledger`; the
        // balance in effect is the one immediately before it.
        let mut low = 0u32;
        let mut high = checkpoints.len();
        while low < high {
            let mid = low + (high - low) / 2;
            match checkpoints.get(mid) {
                Some(checkpoint) if checkpoint.ledger <= ledger => low = mid + 1,
                _ => high = mid,
            }
        }

        if low == 0 {
            0
        } else {
            checkpoints.get(low - 1).map(|c| c.balance).unwrap_or(0)
        }
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
        Self::set_balance(&env, &from, from_bal - amount);
        let to_bal = Self::balance(env.clone(), to.clone());
        Self::set_balance(&env, &to, to_bal + amount);
        Ok(())
    }

    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), TokenError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let supply: i128 = Self::total_supply(env.clone());
        env.storage().instance().set(&DataKey::TotalSupply, &(supply + amount));
        let bal = Self::balance(env.clone(), to.clone());
        Self::set_balance(&env, &to, bal + amount);
        Ok(())
    }

    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        from.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let bal = Self::balance(env.clone(), from.clone());
        if bal < amount { return Err(TokenError::InsufficientBalance); }
        Self::set_balance(&env, &from, bal - amount);
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

/// Internal helpers — outside `#[contractimpl]` so they are not exported as
/// contract functions.
impl QuorumToken {
    fn checkpoints(env: &Env, owner: &Address) -> Vec<Checkpoint> {
        env.storage()
            .persistent()
            .get(&DataKey::Checkpoints(owner.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Writes `balance` for `owner` and records a checkpoint at the current
    /// ledger.
    ///
    /// Every balance mutation goes through here; a write that bypassed it would
    /// leave a gap that `get_past_balance` would silently read straight past,
    /// handing the holder the wrong voting power.
    fn set_balance(env: &Env, owner: &Address, balance: i128) {
        env.storage()
            .persistent()
            .set(&DataKey::Balance(owner.clone()), &balance);

        let ledger = env.ledger().sequence();
        let mut checkpoints = Self::checkpoints(env, owner);
        let len = checkpoints.len();

        // Several transfers can land in one ledger. Collapse them so each
        // ledger keeps exactly one entry — its closing balance — which is what
        // keeps the binary search in get_past_balance unambiguous.
        match checkpoints.get(len.saturating_sub(1)) {
            Some(last) if len > 0 && last.ledger == ledger => {
                checkpoints.set(len - 1, Checkpoint { ledger, balance });
            }
            _ => checkpoints.push_back(Checkpoint { ledger, balance }),
        }

        env.storage()
            .persistent()
            .set(&DataKey::Checkpoints(owner.clone()), &checkpoints);
    }
}

#[cfg(test)]
mod test;
