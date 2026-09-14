use super::*;
use quorum_token::{QuorumToken, QuorumTokenClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};

const QUORUM_BPS: u32 = 500; // 5%
const VOTING_PERIOD: u32 = 100;
const TIMELOCK_PERIOD: u32 = 50;
const PROPOSAL_THRESHOLD: i128 = 0;

/// Registers a QUORUM token and a governor wired to it, minting `initial_supply`
/// to the admin.
fn deploy(env: &Env, initial_supply: i128, quorum_bps: u32) -> (Address, Address, Address) {
    deploy_with_threshold(env, initial_supply, quorum_bps, PROPOSAL_THRESHOLD)
}

/// As `deploy`, with an explicit `proposal_threshold`.
fn deploy_with_threshold(
    env: &Env,
    initial_supply: i128,
    quorum_bps: u32,
    proposal_threshold: i128,
) -> (Address, Address, Address) {
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
        &proposal_threshold,
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

const THRESHOLD: i128 = 10_000;

#[test]
fn proposer_below_threshold_is_rejected() {
    let env = Env::default();
    let (_, _, governance_id) = deploy_with_threshold(&env, 1_000_000, QUORUM_BPS, THRESHOLD);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    let broke = Address::generate(&env);

    assert_eq!(
        governance.try_create_proposal(
            &broke,
            &String::from_str(&env, "Fund my thing"),
            &String::from_str(&env, "I hold no QUORUM."),
        ),
        Err(Ok(GovernanceError::BelowProposalThreshold))
    );
    assert_eq!(governance.get_proposal_count(), 0);
}

#[test]
fn proposer_holding_just_under_threshold_is_rejected() {
    let env = Env::default();
    let (admin, token_id, governance_id) =
        deploy_with_threshold(&env, 1_000_000, QUORUM_BPS, THRESHOLD);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    let almost = Address::generate(&env);
    QuorumTokenClient::new(&env, &token_id).transfer(&admin, &almost, &(THRESHOLD - 1));

    assert_eq!(
        governance.try_create_proposal(
            &almost,
            &String::from_str(&env, "One short"),
            &String::from_str(&env, "Holding threshold - 1."),
        ),
        Err(Ok(GovernanceError::BelowProposalThreshold))
    );
}

#[test]
fn proposer_at_threshold_succeeds() {
    let env = Env::default();
    let (admin, token_id, governance_id) =
        deploy_with_threshold(&env, 1_000_000, QUORUM_BPS, THRESHOLD);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    let holder = Address::generate(&env);

    // Exactly the threshold — the check is inclusive.
    QuorumTokenClient::new(&env, &token_id).transfer(&admin, &holder, &THRESHOLD);

    let id = governance.create_proposal(
        &holder,
        &String::from_str(&env, "Exactly enough"),
        &String::from_str(&env, "Holding exactly the threshold."),
    );
    assert_eq!(governance.get_proposal(&id).proposer, holder);
}

#[test]
fn zero_threshold_lets_any_address_propose() {
    let env = Env::default();
    let (_, _, governance_id) = deploy_with_threshold(&env, 1_000_000, QUORUM_BPS, 0);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    let id = governance.create_proposal(
        &Address::generate(&env),
        &String::from_str(&env, "Open season"),
        &String::from_str(&env, "No threshold configured."),
    );
    assert_eq!(id, 1);
}

const VOTE_FOR: u32 = 1;

#[test]
fn voting_power_is_read_at_the_snapshot_not_the_live_balance() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token_id, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(20);
    let proposal_id = governance.create_proposal(
        &admin,
        &String::from_str(&env, "Raise the quorum threshold"),
        &String::from_str(&env, "Move quorum_bps from 500 to 750."),
    );

    // Admin gives most of the supply away *after* the snapshot.
    env.ledger().set_sequence_number(30);
    let latecomer = Address::generate(&env);
    QuorumTokenClient::new(&env, &token_id).transfer(&admin, &latecomer, &400_000);

    governance.vote(&admin, &proposal_id, &VOTE_FOR);

    // Weight is the snapshot balance (1_000_000), not the live one (600_000).
    assert_eq!(governance.get_proposal(&proposal_id).for_votes, 1_000_000);
}

#[test]
fn tokens_acquired_after_the_snapshot_carry_no_weight() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token_id, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(20);
    let proposal_id = governance.create_proposal(
        &admin,
        &String::from_str(&env, "Raise the quorum threshold"),
        &String::from_str(&env, "Move quorum_bps from 500 to 750."),
    );

    // Buying in after the proposal opened must not buy influence — this is the
    // flash-loan path the snapshot exists to close.
    env.ledger().set_sequence_number(30);
    let latecomer = Address::generate(&env);
    QuorumTokenClient::new(&env, &token_id).transfer(&admin, &latecomer, &400_000);

    assert_eq!(
        governance.try_vote(&latecomer, &proposal_id, &VOTE_FOR),
        Err(Ok(GovernanceError::NoVotingPower))
    );
    assert_eq!(governance.get_proposal(&proposal_id).for_votes, 0);
    assert!(!governance.has_voted(&proposal_id, &latecomer));
}

#[test]
fn address_that_never_held_tokens_cannot_vote() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(20);
    let proposal_id = governance.create_proposal(
        &admin,
        &String::from_str(&env, "Raise the quorum threshold"),
        &String::from_str(&env, "Move quorum_bps from 500 to 750."),
    );

    assert_eq!(
        governance.try_vote(&Address::generate(&env), &proposal_id, &VOTE_FOR),
        Err(Ok(GovernanceError::NoVotingPower))
    );
}

const VOTE_AGAINST: u32 = 0;
const VOTE_ABSTAIN: u32 = 2;

/// Ledger the token is deployed and holders are funded at.
const GENESIS: u32 = 10;
/// Ledger proposals are opened at, so `snapshot_ledger` is GENESIS < L < voting.
const OPENED: u32 = 20;

// ─── Initialization ──────────────────────────────────────────────────────────

#[test]
fn initialize_stores_the_supplied_config() {
    let env = Env::default();
    let (admin, token_id, governance_id) =
        deploy_with_threshold(&env, 1_000_000, QUORUM_BPS, THRESHOLD);

    let config = GovernanceContractClient::new(&env, &governance_id).get_config();
    assert_eq!(config.admin, admin);
    assert_eq!(config.token, token_id);
    assert_eq!(config.quorum_bps, QUORUM_BPS);
    assert_eq!(config.voting_period, VOTING_PERIOD);
    assert_eq!(config.timelock_period, TIMELOCK_PERIOD);
    assert_eq!(config.proposal_threshold, THRESHOLD);
    assert_eq!(
        GovernanceContractClient::new(&env, &governance_id).get_proposal_count(),
        0
    );
}

#[test]
fn initialize_cannot_run_twice() {
    let env = Env::default();
    let (admin, token_id, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);

    assert_eq!(
        GovernanceContractClient::new(&env, &governance_id).try_initialize(
            &admin,
            &token_id,
            &QUORUM_BPS,
            &VOTING_PERIOD,
            &TIMELOCK_PERIOD,
            &PROPOSAL_THRESHOLD,
        ),
        Err(Ok(GovernanceError::AlreadyInitialized))
    );
}

// ─── Proposal creation ───────────────────────────────────────────────────────

#[test]
fn create_proposal_sets_the_expected_ledger_window() {
    let env = Env::default();
    env.ledger().set_sequence_number(GENESIS);
    let (admin, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(OPENED);
    let proposal = propose(&env, &governance_id, &admin);

    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.proposer, admin);
    assert_eq!(proposal.snapshot_ledger, OPENED);
    assert_eq!(proposal.start_ledger, OPENED + 1);
    assert_eq!(proposal.end_ledger, OPENED + 1 + VOTING_PERIOD);
    assert_eq!(proposal.queue_ledger, 0);
    assert_eq!(proposal.status, ProposalStatus::Active);
    assert_eq!(
        (proposal.for_votes, proposal.against_votes, proposal.abstain_votes),
        (0, 0, 0)
    );
    assert_eq!(governance.get_proposal_count(), 1);
}

#[test]
fn proposal_ids_increment_from_one() {
    let env = Env::default();
    env.ledger().set_sequence_number(GENESIS);
    let (admin, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(OPENED);
    for expected in 1..=3u64 {
        assert_eq!(propose(&env, &governance_id, &admin).id, expected);
    }
    assert_eq!(governance.get_proposal_count(), 3);
}

#[test]
fn get_proposal_rejects_an_unknown_id() {
    let env = Env::default();
    let (_, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);

    assert_eq!(
        GovernanceContractClient::new(&env, &governance_id).try_get_proposal(&999),
        Err(Ok(GovernanceError::ProposalNotFound))
    );
}

// ─── Voting ──────────────────────────────────────────────────────────────────

/// Deploys with `supply`, funds each holder before the snapshot, and opens a
/// proposal at `OPENED`. Returns (admin, governance id, proposal id).
fn open_with_holders(
    env: &Env,
    supply: i128,
    quorum_bps: u32,
    holders: &[(Address, i128)],
) -> (Address, Address, u64) {
    env.ledger().set_sequence_number(GENESIS);
    let (admin, token_id, governance_id) = deploy(env, supply, quorum_bps);
    let token = QuorumTokenClient::new(env, &token_id);

    // Funded at GENESIS, before the snapshot, so the grants carry weight.
    for (holder, amount) in holders {
        token.transfer(&admin, holder, amount);
    }

    env.ledger().set_sequence_number(OPENED);
    let id = propose(env, &governance_id, &admin).id;
    (admin, governance_id, id)
}

#[test]
fn votes_accumulate_into_the_matching_tally() {
    let env = Env::default();
    let (against_voter, abstain_voter) = (Address::generate(&env), Address::generate(&env));
    let (admin, governance_id, proposal_id) = open_with_holders(
        &env,
        1_000_000,
        QUORUM_BPS,
        &[(against_voter.clone(), 200_000), (abstain_voter.clone(), 300_000)],
    );
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&admin, &proposal_id, &VOTE_FOR); // 500_000 left after grants
    governance.vote(&against_voter, &proposal_id, &VOTE_AGAINST);
    governance.vote(&abstain_voter, &proposal_id, &VOTE_ABSTAIN);

    let proposal = governance.get_proposal(&proposal_id);
    assert_eq!(proposal.for_votes, 500_000);
    assert_eq!(proposal.against_votes, 200_000);
    assert_eq!(proposal.abstain_votes, 300_000);
}

#[test]
fn a_voter_cannot_vote_twice() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&admin, &proposal_id, &VOTE_FOR);
    assert_eq!(
        governance.try_vote(&admin, &proposal_id, &VOTE_AGAINST),
        Err(Ok(GovernanceError::AlreadyVoted))
    );

    // The rejected second vote left the tallies untouched.
    let proposal = governance.get_proposal(&proposal_id);
    assert_eq!(proposal.for_votes, 1_000_000);
    assert_eq!(proposal.against_votes, 0);
}

#[test]
fn votes_after_the_deadline_are_rejected() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    let end_ledger = governance.get_proposal(&proposal_id).end_ledger;

    // The final ledger of the window still accepts votes.
    env.ledger().set_sequence_number(end_ledger);
    governance.vote(&admin, &proposal_id, &VOTE_FOR);

    env.ledger().set_sequence_number(end_ledger + 1);
    let latecomer = Address::generate(&env);
    assert_eq!(
        governance.try_vote(&latecomer, &proposal_id, &VOTE_FOR),
        Err(Ok(GovernanceError::VotingPeriodEnded))
    );
}

#[test]
fn an_out_of_range_vote_choice_is_rejected() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    assert_eq!(
        governance.try_vote(&admin, &proposal_id, &3),
        Err(Ok(GovernanceError::InvalidVoteChoice))
    );
    assert!(!governance.has_voted(&proposal_id, &admin));
}

#[test]
fn voting_on_an_unknown_proposal_is_rejected() {
    let env = Env::default();
    let (admin, governance_id, _) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);

    assert_eq!(
        GovernanceContractClient::new(&env, &governance_id).try_vote(&admin, &999, &VOTE_FOR),
        Err(Ok(GovernanceError::ProposalNotFound))
    );
}

#[test]
fn get_vote_returns_the_recorded_choice() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token_id, governance_id) = deploy(&env, 900_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    let token = QuorumTokenClient::new(&env, &token_id);

    // Three holders, funded before the snapshot so each carries weight.
    let against = Address::generate(&env);
    let abstain = Address::generate(&env);
    token.transfer(&admin, &against, &300_000);
    token.transfer(&admin, &abstain, &300_000);

    env.ledger().set_sequence_number(20);
    let proposal_id = governance.create_proposal(
        &admin,
        &String::from_str(&env, "Three-way split"),
        &String::from_str(&env, "One voter per choice."),
    );

    governance.vote(&admin, &proposal_id, &VOTE_FOR);
    governance.vote(&against, &proposal_id, &VOTE_AGAINST);
    governance.vote(&abstain, &proposal_id, &VOTE_ABSTAIN);

    assert_eq!(governance.get_vote(&proposal_id, &admin), Some(VOTE_FOR));
    assert_eq!(governance.get_vote(&proposal_id, &against), Some(VOTE_AGAINST));
    assert_eq!(governance.get_vote(&proposal_id, &abstain), Some(VOTE_ABSTAIN));
}

#[test]
fn get_vote_is_none_for_an_address_that_has_not_voted() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(20);
    let proposal_id = governance.create_proposal(
        &admin,
        &String::from_str(&env, "Nobody has voted yet"),
        &String::from_str(&env, "Fresh proposal."),
    );

    // Never voted, and a non-holder who could not vote even if they tried.
    assert_eq!(governance.get_vote(&proposal_id, &admin), None);
    assert_eq!(governance.get_vote(&proposal_id, &Address::generate(&env)), None);

    // A rejected vote must not leave a record behind.
    let latecomer = Address::generate(&env);
    assert!(governance.try_vote(&latecomer, &proposal_id, &VOTE_FOR).is_err());
    assert_eq!(governance.get_vote(&proposal_id, &latecomer), None);
}

#[test]
fn get_vote_is_none_for_an_unknown_proposal() {
    let env = Env::default();
    let (admin, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);

    assert_eq!(
        GovernanceContractClient::new(&env, &governance_id).get_vote(&999, &admin),
        None
    );
}

// ─── Finalize ────────────────────────────────────────────────────────────────

/// Moves past `end_ledger` so the proposal can be finalized.
fn close_voting(env: &Env, governance: &GovernanceContractClient, proposal_id: u64) {
    let end_ledger = governance.get_proposal(&proposal_id).end_ledger;
    env.ledger().set_sequence_number(end_ledger + 1);
}

#[test]
fn finalize_queues_a_proposal_that_clears_quorum_and_majority() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&admin, &proposal_id, &VOTE_FOR);
    close_voting(&env, &governance, proposal_id);
    let closed_at = env.ledger().sequence();

    assert_eq!(governance.finalize(&proposal_id), ProposalStatus::Queued);

    let proposal = governance.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Queued);
    assert_eq!(proposal.queue_ledger, closed_at + TIMELOCK_PERIOD);
}

#[test]
fn finalize_fails_a_proposal_that_misses_quorum() {
    let env = Env::default();
    let small_holder = Address::generate(&env);
    // 50% quorum against a 1_000_000 supply needs 500_000; this voter has 1_000.
    let (_, governance_id, proposal_id) = open_with_holders(
        &env,
        1_000_000,
        5_000,
        &[(small_holder.clone(), 1_000)],
    );
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&small_holder, &proposal_id, &VOTE_FOR);
    close_voting(&env, &governance, proposal_id);

    assert_eq!(governance.finalize(&proposal_id), ProposalStatus::Failed);
    assert_eq!(governance.get_proposal(&proposal_id).queue_ledger, 0);
}

#[test]
fn finalize_fails_a_proposal_that_clears_quorum_but_loses_the_vote() {
    let env = Env::default();
    let opposition = Address::generate(&env);
    let (admin, governance_id, proposal_id) = open_with_holders(
        &env,
        1_000_000,
        QUORUM_BPS,
        &[(opposition.clone(), 600_000)],
    );
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&admin, &proposal_id, &VOTE_FOR); // 400_000
    governance.vote(&opposition, &proposal_id, &VOTE_AGAINST); // 600_000
    close_voting(&env, &governance, proposal_id);

    // Turnout clears quorum, but Against wins.
    assert_eq!(governance.finalize(&proposal_id), ProposalStatus::Failed);
}

#[test]
fn a_tie_fails_because_majority_requires_strictly_more_for_votes() {
    let env = Env::default();
    let opposition = Address::generate(&env);
    let (admin, governance_id, proposal_id) = open_with_holders(
        &env,
        1_000_000,
        QUORUM_BPS,
        &[(opposition.clone(), 500_000)],
    );
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&admin, &proposal_id, &VOTE_FOR); // 500_000
    governance.vote(&opposition, &proposal_id, &VOTE_AGAINST); // 500_000
    close_voting(&env, &governance, proposal_id);

    assert_eq!(governance.finalize(&proposal_id), ProposalStatus::Failed);
}

#[test]
fn finalize_before_the_deadline_is_rejected() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    governance.vote(&admin, &proposal_id, &VOTE_FOR);

    // On end_ledger itself voting is still open, so finalize is premature.
    env.ledger()
        .set_sequence_number(governance.get_proposal(&proposal_id).end_ledger);
    assert_eq!(
        governance.try_finalize(&proposal_id),
        Err(Ok(GovernanceError::VotingNotActive))
    );
}

// ─── Execute ─────────────────────────────────────────────────────────────────

/// Votes the proposal through and finalizes it into Queued.
fn queue_proposal(env: &Env, governance: &GovernanceContractClient, admin: &Address, id: u64) {
    governance.vote(admin, &id, &VOTE_FOR);
    close_voting(env, governance, id);
    assert_eq!(governance.finalize(&id), ProposalStatus::Queued);
}

#[test]
fn execute_succeeds_once_the_timelock_has_elapsed() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    queue_proposal(&env, &governance, &admin, proposal_id);

    let queue_ledger = governance.get_proposal(&proposal_id).queue_ledger;
    env.ledger().set_sequence_number(queue_ledger);
    governance.execute(&proposal_id);

    assert_eq!(
        governance.get_proposal(&proposal_id).status,
        ProposalStatus::Executed
    );
}

#[test]
fn execute_during_the_timelock_is_rejected() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    queue_proposal(&env, &governance, &admin, proposal_id);

    let queue_ledger = governance.get_proposal(&proposal_id).queue_ledger;
    env.ledger().set_sequence_number(queue_ledger - 1);

    assert_eq!(
        governance.try_execute(&proposal_id),
        Err(Ok(GovernanceError::TimelockNotExpired))
    );
    assert_eq!(
        governance.get_proposal(&proposal_id).status,
        ProposalStatus::Queued
    );
}

#[test]
fn execute_is_rejected_while_a_proposal_is_still_active() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);
    governance.vote(&admin, &proposal_id, &VOTE_FOR);

    assert_eq!(
        governance.try_execute(&proposal_id),
        Err(Ok(GovernanceError::ProposalNotPassed))
    );
}

#[test]
fn a_failed_proposal_cannot_be_executed() {
    let env = Env::default();
    let small_holder = Address::generate(&env);
    let (_, governance_id, proposal_id) =
        open_with_holders(&env, 1_000_000, 5_000, &[(small_holder.clone(), 1_000)]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.vote(&small_holder, &proposal_id, &VOTE_FOR);
    close_voting(&env, &governance, proposal_id);
    assert_eq!(governance.finalize(&proposal_id), ProposalStatus::Failed);

    env.ledger().set_sequence_number(env.ledger().sequence() + TIMELOCK_PERIOD + 1);
    assert_eq!(
        governance.try_execute(&proposal_id),
        Err(Ok(GovernanceError::ProposalNotPassed))
    );
}

// ─── Cancel ──────────────────────────────────────────────────────────────────

#[test]
fn a_proposer_can_cancel_their_own_proposal() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    // admin is the proposer here (open_with_holders proposes as admin).
    governance.cancel(&admin, &proposal_id);
    assert_eq!(
        governance.get_proposal(&proposal_id).status,
        ProposalStatus::Cancelled
    );
}

#[test]
fn the_admin_can_cancel_a_proposal_they_did_not_open() {
    let env = Env::default();
    env.ledger().set_sequence_number(GENESIS);
    let (admin, _, governance_id) = deploy(&env, 1_000_000, QUORUM_BPS);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(OPENED);
    let outsider = Address::generate(&env);
    let proposal_id = propose(&env, &governance_id, &outsider).id;

    governance.cancel(&admin, &proposal_id);
    assert_eq!(
        governance.get_proposal(&proposal_id).status,
        ProposalStatus::Cancelled
    );
}

#[test]
fn a_third_party_cannot_cancel() {
    let env = Env::default();
    let (_, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    assert_eq!(
        governance.try_cancel(&Address::generate(&env), &proposal_id),
        Err(Ok(GovernanceError::Unauthorized))
    );
    assert_eq!(
        governance.get_proposal(&proposal_id).status,
        ProposalStatus::Active
    );
}

#[test]
fn a_cancelled_proposal_stops_accepting_votes() {
    let env = Env::default();
    let (admin, governance_id, proposal_id) = open_with_holders(&env, 1_000_000, QUORUM_BPS, &[]);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    governance.cancel(&admin, &proposal_id);
    assert_eq!(
        governance.try_vote(&admin, &proposal_id, &VOTE_FOR),
        Err(Ok(GovernanceError::VotingNotActive))
    );
}

#[test]
fn add_weight_saturates_into_an_error_at_the_i128_boundary() {
    assert_eq!(
        GovernanceContract::add_weight(i128::MAX - 1, 1),
        Ok(i128::MAX)
    );
    assert_eq!(
        GovernanceContract::add_weight(i128::MAX, 1),
        Err(GovernanceError::Overflow)
    );
    assert_eq!(
        GovernanceContract::add_weight(1, i128::MAX),
        Err(GovernanceError::Overflow)
    );
    assert_eq!(GovernanceContract::add_weight(0, 0), Ok(0));
}

#[test]
fn tallying_the_entire_supply_at_the_boundary_does_not_trap() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    // quorum_bps must be 0 here: any non-zero bps against a maxed supply
    // overflows the quorum calculation before a vote is ever cast.
    let (admin, _, governance_id) = deploy(&env, i128::MAX, 0);
    let governance = GovernanceContractClient::new(&env, &governance_id);

    env.ledger().set_sequence_number(20);
    let proposal_id = governance.create_proposal(
        &admin,
        &String::from_str(&env, "Whole supply votes"),
        &String::from_str(&env, "Single holder controlling i128::MAX."),
    );
    governance.vote(&admin, &proposal_id, &VOTE_FOR);

    assert_eq!(governance.get_proposal(&proposal_id).for_votes, i128::MAX);
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
