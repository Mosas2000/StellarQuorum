#![no_std]
//! QUORUM — governance token for the Quorum protocol. SEP-41 compatible.
use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, String, Symbol, Vec};

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
    InvalidExpiration     = 7,
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

/// Emitted when tokens move between accounts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

/// Emitted when tokens are created. `total_supply` is the value after the mint,
/// so an indexer can track supply from events alone.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mint {
    pub to: Address,
    pub amount: i128,
    pub total_supply: i128,
}

/// Emitted when tokens are destroyed. `total_supply` is the value after the
/// burn.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Burn {
    pub from: Address,
    pub amount: i128,
    pub total_supply: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Approve {
    pub owner: Address,
    pub spender: Address,
    pub amount: i128,
    pub expiration_ledger: u32,
}

/// A stored allowance and the ledger it lapses after.
///
/// SEP-41 requires allowances to expire: a permanent approval left on a
/// compromised or abandoned spender is a standing claim on the owner's balance.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferred {
    pub previous_admin: Address,
    pub new_admin: Address,
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

        // The genesis allocation is a mint. Emitting it keeps supply
        // reconstructible from events alone, with no special-case for deploy.
        env.events().publish(
            (Symbol::new(&env, "mint"), admin.clone()),
            Mint { to: admin, amount: initial_supply, total_supply: initial_supply },
        );
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
        Self::move_balance(&env, &from, &to, amount)
    }

    /// Moves `amount` from `from` to `to` on behalf of `spender`, drawing on an
    /// allowance the owner granted with `approve`.
    ///
    /// Required by SEP-41: without it no other contract can spend an approved
    /// balance, which is what staking and delegation flows are built on.
    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        // The spender authorizes, not the owner — the owner already consented
        // by approving.
        spender.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }

        // Reads as zero if the approval has lapsed, so an expired allowance
        // fails here as InsufficientAllowance.
        let approval = Self::live_allowance(&env, &from, &spender);
        if approval.amount < amount { return Err(TokenError::InsufficientAllowance); }

        // Move first: it validates the balance and fails without touching the
        // allowance, so a rejected spend cannot consume allowance.
        Self::move_balance(&env, &from, &to, amount)?;

        // Debiting keeps the original expiry — spending part of an allowance
        // must not extend the remainder's life.
        env.storage().persistent().set(
            &DataKey::Allowance(from, spender),
            &AllowanceValue {
                amount: approval.amount - amount,
                expiration_ledger: approval.expiration_ledger,
            },
        );
        Ok(())
    }

    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), TokenError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let supply: i128 = Self::total_supply(env.clone());
        let total_supply = supply + amount;
        env.storage().instance().set(&DataKey::TotalSupply, &total_supply);
        let bal = Self::balance(env.clone(), to.clone());
        Self::set_balance(&env, &to, bal + amount);

        env.events().publish(
            (Symbol::new(&env, "mint"), to.clone()),
            Mint { to, amount, total_supply },
        );
        Ok(())
    }

    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        from.require_auth();
        if amount <= 0 { return Err(TokenError::InvalidAmount); }
        let bal = Self::balance(env.clone(), from.clone());
        if bal < amount { return Err(TokenError::InsufficientBalance); }
        Self::set_balance(&env, &from, bal - amount);
        let supply = Self::total_supply(env.clone());
        let total_supply = supply - amount;
        env.storage().instance().set(&DataKey::TotalSupply, &total_supply);

        env.events().publish(
            (Symbol::new(&env, "burn"), from.clone()),
            Burn { from, amount, total_supply },
        );
        Ok(())
    }

    /// Grants `spender` the right to move up to `amount` of `owner`'s balance,
    /// until `expiration_ledger` has passed.
    ///
    /// Per SEP-41, a live approval (`amount > 0`) must not expire in the past.
    /// A zero `amount` is a revocation and accepts any expiration, which is how
    /// an owner cancels an approval they can no longer honour.
    pub fn approve(env: Env, owner: Address, spender: Address, amount: i128, expiration_ledger: u32) -> Result<(), TokenError> {
        owner.require_auth();
        if amount < 0 { return Err(TokenError::InvalidAmount); }
        if amount > 0 && expiration_ledger < env.ledger().sequence() {
            return Err(TokenError::InvalidExpiration);
        }

        env.storage().persistent().set(
            &DataKey::Allowance(owner.clone(), spender.clone()),
            &AllowanceValue { amount, expiration_ledger },
        );

        env.events().publish(
            (Symbol::new(&env, "approve"), owner.clone(), spender.clone()),
            Approve { owner, spender, amount, expiration_ledger },
        );
        Ok(())
    }

    /// Amount `spender` may still draw from `owner`, or 0 once the approval has
    /// lapsed.
    ///
    /// An expired allowance reads as 0 rather than erroring, so callers cannot
    /// accidentally treat a lapsed approval as spendable.
    pub fn allowance(env: Env, owner: Address, spender: Address) -> i128 {
        Self::live_allowance(&env, &owner, &spender).amount
    }

    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), TokenError> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        env.events().publish(
            (Symbol::new(&env, "admin_transferred"), admin.clone()),
            AdminTransferred { previous_admin: admin, new_admin },
        );
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
    /// The stored allowance if it is still live, otherwise a zero allowance.
    ///
    /// Returning a zeroed value rather than `None` keeps the expiry check in
    /// one place: every spender path treats a lapsed approval as empty.
    fn live_allowance(env: &Env, owner: &Address, spender: &Address) -> AllowanceValue {
        match env
            .storage()
            .persistent()
            .get::<_, AllowanceValue>(&DataKey::Allowance(owner.clone(), spender.clone()))
        {
            Some(allowance) if allowance.expiration_ledger >= env.ledger().sequence() => allowance,
            _ => AllowanceValue { amount: 0, expiration_ledger: 0 },
        }
    }

    /// Debits `from`, credits `to`, and emits the transfer event.
    ///
    /// Shared by `transfer` and `transfer_from` so both paths apply the same
    /// balance checks, write the same checkpoints and emit the same event —
    /// an allowance spend is indistinguishable from a direct transfer to
    /// anything watching balances.
    fn move_balance(env: &Env, from: &Address, to: &Address, amount: i128) -> Result<(), TokenError> {
        let from_bal = Self::balance(env.clone(), from.clone());
        if from_bal < amount { return Err(TokenError::InsufficientBalance); }

        Self::set_balance(env, from, from_bal - amount);
        // Read `to` after debiting `from`, so a self-transfer nets to zero
        // instead of crediting a stale balance.
        let to_bal = Self::balance(env.clone(), to.clone());
        Self::set_balance(env, to, to_bal + amount);

        env.events().publish(
            (Symbol::new(env, "transfer"), from.clone(), to.clone()),
            Transfer { from: from.clone(), to: to.clone(), amount },
        );
        Ok(())
    }

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
