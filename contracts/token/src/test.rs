use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};

const INITIAL_SUPPLY: i128 = 1_000_000;

fn deploy(env: &Env) -> (Address, QuorumTokenClient<'_>) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let token_id = env.register(QuorumToken, ());
    let token = QuorumTokenClient::new(env, &token_id);
    token.initialize(
        &admin,
        &String::from_str(env, "Quorum"),
        &String::from_str(env, "QUORUM"),
        &7,
        &INITIAL_SUPPLY,
    );
    (admin, token)
}

#[test]
fn initial_supply_is_checkpointed_to_the_admin() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);

    assert_eq!(token.get_past_balance(&admin, &10), INITIAL_SUPPLY);
}

#[test]
fn balance_before_first_checkpoint_reads_as_zero() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);

    // The admin held nothing before the token existed.
    assert_eq!(token.get_past_balance(&admin, &9), 0);

    // An address that never held tokens has no checkpoints at all.
    let stranger = Address::generate(&env);
    assert_eq!(token.get_past_balance(&stranger, &10), 0);
}

#[test]
fn transfers_checkpoint_both_sides_at_the_current_ledger() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    env.ledger().set_sequence_number(20);
    token.transfer(&admin, &recipient, &400_000);

    // Before the transfer
    assert_eq!(token.get_past_balance(&admin, &19), INITIAL_SUPPLY);
    assert_eq!(token.get_past_balance(&recipient, &19), 0);

    // After
    assert_eq!(token.get_past_balance(&admin, &20), 600_000);
    assert_eq!(token.get_past_balance(&recipient, &20), 400_000);
}

#[test]
fn past_balance_holds_steady_between_checkpoints() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    env.ledger().set_sequence_number(20);
    token.transfer(&admin, &recipient, &100_000);

    // No activity between 20 and 50, so the ledger-20 balance still stands.
    for ledger in [21u32, 35, 50] {
        assert_eq!(token.get_past_balance(&recipient, &ledger), 100_000);
    }
}

#[test]
fn multiple_transfers_in_one_ledger_collapse_to_closing_balance() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    env.ledger().set_sequence_number(20);
    token.transfer(&admin, &recipient, &100_000);
    token.transfer(&admin, &recipient, &50_000);
    token.transfer(&recipient, &admin, &25_000);

    assert_eq!(token.get_past_balance(&recipient, &20), 125_000);
    assert_eq!(token.balance(&recipient), 125_000);
}

#[test]
fn mint_and_burn_are_checkpointed() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);

    env.ledger().set_sequence_number(30);
    token.mint(&admin, &500_000);
    assert_eq!(token.get_past_balance(&admin, &30), 1_500_000);

    env.ledger().set_sequence_number(40);
    token.burn(&admin, &200_000);
    assert_eq!(token.get_past_balance(&admin, &40), 1_300_000);

    // History stays intact behind the latest entry.
    assert_eq!(token.get_past_balance(&admin, &30), 1_500_000);
    assert_eq!(token.get_past_balance(&admin, &10), INITIAL_SUPPLY);
}

#[test]
fn binary_search_resolves_the_correct_entry_across_many_checkpoints() {
    let env = Env::default();
    env.ledger().set_sequence_number(1);
    let (admin, token) = deploy(&env);
    let holder = Address::generate(&env);

    // One transfer of 1_000 per ledger, at ledgers 10, 20 … 200.
    for step in 1..=20u32 {
        env.ledger().set_sequence_number(step * 10);
        token.transfer(&admin, &holder, &1_000);
    }

    // Each checkpoint, and each gap between them, resolves to the running total.
    for step in 1..=20u32 {
        let expected = i128::from(step) * 1_000;
        assert_eq!(token.get_past_balance(&holder, &(step * 10)), expected);
        assert_eq!(token.get_past_balance(&holder, &(step * 10 + 9)), expected);
    }

    assert_eq!(token.get_past_balance(&holder, &9), 0);
    assert_eq!(token.get_past_balance(&holder, &10_000), 20_000);
}
