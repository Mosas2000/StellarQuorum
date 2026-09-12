#![no_std]
use soroban_sdk::{contract, contractclient, contractimpl, contracttype, contracterror, Address, Env, String};

/// Subset of the QUORUM token interface the governor depends on.
///
/// Generates `TokenClient`, used to cross-invoke the token contract at the
/// address held in `Config::token`.
#[contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn total_supply(env: Env) -> i128;
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GovernanceError {
    AlreadyInitialized      = 1,
    Unauthorized            = 2,
    ProposalNotFound        = 3,
    VotingNotActive         = 4,
    AlreadyVoted            = 5,
    QuorumNotReached        = 6,
    ProposalNotPassed       = 7,
    AlreadyExecuted         = 8,
    BelowProposalThreshold  = 9,
    InvalidVoteChoice       = 10,
    VotingPeriodEnded       = 11,
    TimelockNotExpired      = 12,
    Overflow                = 13,
}

/// Basis-point denominator: `quorum_bps` of 500 means 5% of total supply.
const BPS_DENOMINATOR: i128 = 10_000;

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ProposalStatus {
    Pending, Active, Passed, Failed, Queued, Executed, Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub for_votes: i128,
    pub against_votes: i128,
    pub abstain_votes: i128,
    pub snapshot_ledger: u32,
    pub start_ledger: u32,
    pub end_ledger: u32,
    pub queue_ledger: u32,
    pub quorum_required: i128,
    pub status: ProposalStatus,
}

#[contracttype]
pub struct Config {
    pub token: Address,
    pub quorum_bps: u32,
    pub voting_period: u32,
    pub timelock_period: u32,
    pub proposal_threshold: i128,
    pub admin: Address,
}

#[contracttype]
pub enum DataKey {
    Config,
    ProposalCount,
    Proposal(u64),
    HasVoted(u64, Address),
    Delegate(Address),
}

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    pub fn initialize(env: Env, admin: Address, token: Address, quorum_bps: u32, voting_period: u32, timelock_period: u32, proposal_threshold: i128) -> Result<(), GovernanceError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(GovernanceError::AlreadyInitialized);
        }
        admin.require_auth();
        let config = Config { token, quorum_bps, voting_period, timelock_period, proposal_threshold, admin };
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage().instance().set(&DataKey::ProposalCount, &0u64);
        Ok(())
    }

    pub fn create_proposal(env: Env, proposer: Address, title: String, description: String) -> Result<u64, GovernanceError> {
        proposer.require_auth();
        let count: u64 = env.storage().instance().get(&DataKey::ProposalCount).unwrap_or(0);
        let id = count + 1;
        let current = env.ledger().sequence();
        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        let quorum_required = Self::quorum_for_supply(
            TokenClient::new(&env, &config.token).total_supply(),
            config.quorum_bps,
        )?;
        let proposal = Proposal {
            id, proposer, title, description,
            for_votes: 0, against_votes: 0, abstain_votes: 0,
            snapshot_ledger: current,
            start_ledger: current + 1,
            end_ledger: current + 1 + config.voting_period,
            queue_ledger: 0,
            quorum_required,
            status: ProposalStatus::Active,
        };
        env.storage().persistent().set(&DataKey::Proposal(id), &proposal);
        env.storage().instance().set(&DataKey::ProposalCount, &id);
        Ok(id)
    }

    pub fn vote(env: Env, voter: Address, proposal_id: u64, support: u32) -> Result<(), GovernanceError> {
        voter.require_auth();
        if support > 2 { return Err(GovernanceError::InvalidVoteChoice); }
        if env.storage().persistent().has(&DataKey::HasVoted(proposal_id, voter.clone())) {
            return Err(GovernanceError::AlreadyVoted);
        }
        let mut proposal: Proposal = env.storage().persistent()
            .get(&DataKey::Proposal(proposal_id)).ok_or(GovernanceError::ProposalNotFound)?;
        if env.ledger().sequence() > proposal.end_ledger { return Err(GovernanceError::VotingPeriodEnded); }
        if proposal.status != ProposalStatus::Active { return Err(GovernanceError::VotingNotActive); }
        let voting_power: i128 = 0; // TODO: get from token contract at snapshot_ledger
        match support {
            0 => proposal.against_votes += voting_power,
            1 => proposal.for_votes += voting_power,
            _ => proposal.abstain_votes += voting_power,
        }
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().persistent().set(&DataKey::HasVoted(proposal_id, voter), &support);
        Ok(())
    }

    pub fn finalize(env: Env, proposal_id: u64) -> Result<ProposalStatus, GovernanceError> {
        let mut proposal: Proposal = env.storage().persistent()
            .get(&DataKey::Proposal(proposal_id)).ok_or(GovernanceError::ProposalNotFound)?;
        if env.ledger().sequence() <= proposal.end_ledger { return Err(GovernanceError::VotingNotActive); }
        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        let total = proposal.for_votes + proposal.against_votes + proposal.abstain_votes;
        let quorum_ok = total >= proposal.quorum_required;
        let majority_for = proposal.for_votes > proposal.against_votes;
        proposal.status = if quorum_ok && majority_for {
            proposal.queue_ledger = env.ledger().sequence() + config.timelock_period;
            ProposalStatus::Queued
        } else { ProposalStatus::Failed };
        let status = proposal.status.clone();
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        Ok(status)
    }

    pub fn execute(env: Env, proposal_id: u64) -> Result<(), GovernanceError> {
        let mut proposal: Proposal = env.storage().persistent()
            .get(&DataKey::Proposal(proposal_id)).ok_or(GovernanceError::ProposalNotFound)?;
        if proposal.status != ProposalStatus::Queued { return Err(GovernanceError::ProposalNotPassed); }
        if env.ledger().sequence() < proposal.queue_ledger { return Err(GovernanceError::TimelockNotExpired); }
        proposal.status = ProposalStatus::Executed;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        // TODO: dispatch on-chain actions encoded in proposal
        Ok(())
    }

    pub fn get_proposal(env: Env, id: u64) -> Result<Proposal, GovernanceError> {
        env.storage().persistent().get(&DataKey::Proposal(id)).ok_or(GovernanceError::ProposalNotFound)
    }

    pub fn get_proposal_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::ProposalCount).unwrap_or(0)
    }

    pub fn has_voted(env: Env, proposal_id: u64, voter: Address) -> bool {
        env.storage().persistent().has(&DataKey::HasVoted(proposal_id, voter))
    }

    pub fn get_config(env: Env) -> Config {
        env.storage().instance().get(&DataKey::Config).unwrap()
    }

    pub fn cancel(env: Env, caller: Address, proposal_id: u64) -> Result<(), GovernanceError> {
        caller.require_auth();
        let mut proposal: Proposal = env.storage().persistent()
            .get(&DataKey::Proposal(proposal_id)).ok_or(GovernanceError::ProposalNotFound)?;
        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();
        if caller != proposal.proposer && caller != config.admin {
            return Err(GovernanceError::Unauthorized);
        }
        proposal.status = ProposalStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &proposal);
        Ok(())
    }
}
/// Internal helpers — outside `#[contractimpl]` so they are not exported as
/// contract functions.
impl GovernanceContract {
    /// Quorum threshold for a given circulating supply: `supply * bps / 10000`.
    ///
    /// Integer division truncates, so the threshold is never rounded up beyond
    /// what the supply supports. Uses checked arithmetic because a large supply
    /// multiplied by `quorum_bps` can exceed `i128::MAX`.
    fn quorum_for_supply(total_supply: i128, quorum_bps: u32) -> Result<i128, GovernanceError> {
        total_supply
            .checked_mul(i128::from(quorum_bps))
            .map(|scaled| scaled / BPS_DENOMINATOR)
            .ok_or(GovernanceError::Overflow)
    }
}

#[cfg(test)]
mod test;

// TODO: add delegate() for voting power delegation
// TODO: add get_past_votes(address, ledger) for snapshot-based power
