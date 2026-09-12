use super::*;
use quorum_token::{QuorumToken, QuorumTokenClient};
use soroban_sdk::testutils::Address as _;

const QUORUM_BPS: u32 = 500; // 5%
const VOTING_PERIOD: u32 = 100;
const TIMELOCK_PERIOD: u32 = 50;
const PROPOSAL_THRESHOLD: i128 = 0;

/// Registers a QUORUM token and a governor wired to it, minting `initial_supply`
/// to the admin.
fn deploy(env: &Env, initial_supply: i128, quorum_bps: u32) -> (Address, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);

    let token_id = env.register(QuorumToken, ());
    QuorumTokenClient::new(env, &token_id).initialize(
        &admin,
        &String::from_str(env, "Quorum"),
        &String::from_str(env, "QUORUM"),
        &7,
        &initial_supply,
    );

    let governance_id = env.register(GovernanceContract, ());
    GovernanceContractClient::new(env, &governance_id).initialize(
        &admin,
        &token_id,
        &quorum_bps,
        &VOTING_PERIOD,
        &TIMELOCK_PERIOD,
        &PROPOSAL_THRESHOLD,
    );

    (admin, token_id, governance_id)
}

fn propose(env: &Env, governance_id: &Address, proposer: &Address) -> Proposal {
    let governance = GovernanceContractClient::new(env, governance_id);
    let id = governance.create_proposal(
        proposer,
        &String::from_str(env, "Raise the quorum threshold"),
        &String::from_str(env, "Move quorum_bps from 500 to 750."),
    );
    governance.get_proposal(&id)
}

#[test]
fn quorum_required_is_derived_from_token_total_supply() {
    let env = Env::default();
    let (_, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let proposer = Address::generate(&env);

    let proposal = propose(&env, &governance_id, &proposer);

    // 5% of 1_000_000
    assert_eq!(proposal.quorum_required, 50_000);
}

#[test]
fn quorum_required_tracks_supply_changes_between_proposals() {
    let env = Env::default();
    let (admin, token_id, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let proposer = Address::generate(&env);

    let first = propose(&env, &governance_id, &proposer);
    assert_eq!(first.quorum_required, 50_000);

    // Minting raises circulating supply, so later proposals need a higher bar.
    QuorumTokenClient::new(&env, &token_id).mint(&admin, &1_000_000);

    let second = propose(&env, &governance_id, &proposer);
    assert_eq!(second.quorum_required, 100_000);
}

#[test]
fn quorum_bps_of_zero_yields_no_threshold() {
    let env = Env::default();
    let (_, _, governance_id) = deploy(&env, 1_000_000, 0);
    let proposer = Address::generate(&env);

    assert_eq!(propose(&env, &governance_id, &proposer).quorum_required, 0);
}

#[test]
fn quorum_for_supply_truncates_fractional_thresholds() {
    // 5% of 199 is 9.95 — truncated down so the threshold never exceeds supply.
    assert_eq!(
        GovernanceContract::quorum_for_supply(199, QUORUM_BPS),
        Ok(9)
    );
    assert_eq!(GovernanceContract::quorum_for_supply(0, QUORUM_BPS), Ok(0));
}

#[test]
fn quorum_for_supply_rejects_overflow_instead_of_panicking() {
    assert_eq!(
        GovernanceContract::quorum_for_supply(i128::MAX, 10_000),
        Err(GovernanceError::Overflow)
    );
}
