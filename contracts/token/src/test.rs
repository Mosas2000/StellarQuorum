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

// ─── Initialization & metadata ───────────────────────────────────────────────

#[test]
fn initialize_sets_metadata_supply_and_admin_balance() {
    let env = Env::default();
    let (admin, token) = deploy(&env);

    assert_eq!(token.name(), String::from_str(&env, "Quorum"));
    assert_eq!(token.symbol(), String::from_str(&env, "QUORUM"));
    assert_eq!(token.decimals(), 7);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
    assert_eq!(token.balance(&admin), INITIAL_SUPPLY);
}

#[test]
fn initialize_cannot_run_twice() {
    let env = Env::default();
    let (admin, token) = deploy(&env);

    assert_eq!(
        token.try_initialize(
            &admin,
            &String::from_str(&env, "Impostor"),
            &String::from_str(&env, "FAKE"),
            &2,
            &999,
        ),
        Err(Ok(TokenError::AlreadyInitialized))
    );
    // Original metadata survives the rejected call.
    assert_eq!(token.symbol(), String::from_str(&env, "QUORUM"));
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn balance_of_an_unknown_address_is_zero() {
    let env = Env::default();
    let (_, token) = deploy(&env);

    assert_eq!(token.balance(&Address::generate(&env)), 0);
}

// ─── Transfer ────────────────────────────────────────────────────────────────

#[test]
fn transfer_moves_balance_between_accounts() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    token.transfer(&admin, &recipient, &250_000);

    assert_eq!(token.balance(&admin), INITIAL_SUPPLY - 250_000);
    assert_eq!(token.balance(&recipient), 250_000);
    // Moving tokens must not change how many exist.
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn transfer_of_the_entire_balance_is_allowed() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    token.transfer(&admin, &recipient, &INITIAL_SUPPLY);

    assert_eq!(token.balance(&admin), 0);
    assert_eq!(token.balance(&recipient), INITIAL_SUPPLY);
}

#[test]
fn transfer_beyond_balance_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    assert_eq!(
        token.try_transfer(&admin, &recipient, &(INITIAL_SUPPLY + 1)),
        Err(Ok(TokenError::InsufficientBalance))
    );
    assert_eq!(token.balance(&admin), INITIAL_SUPPLY);
    assert_eq!(token.balance(&recipient), 0);
}

#[test]
fn transfer_of_a_non_positive_amount_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    assert_eq!(
        token.try_transfer(&admin, &recipient, &0),
        Err(Ok(TokenError::InvalidAmount))
    );
    assert_eq!(
        token.try_transfer(&admin, &recipient, &-100),
        Err(Ok(TokenError::InvalidAmount))
    );
}

// ─── Mint & burn ─────────────────────────────────────────────────────────────

#[test]
fn mint_raises_the_recipient_balance_and_total_supply() {
    let env = Env::default();
    let (_, token) = deploy(&env);
    let recipient = Address::generate(&env);

    token.mint(&recipient, &300_000);

    assert_eq!(token.balance(&recipient), 300_000);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY + 300_000);
}

#[test]
fn mint_of_a_non_positive_amount_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);

    assert_eq!(token.try_mint(&admin, &0), Err(Ok(TokenError::InvalidAmount)));
    assert_eq!(
        token.try_mint(&admin, &-1),
        Err(Ok(TokenError::InvalidAmount))
    );
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn burn_lowers_the_holder_balance_and_total_supply() {
    let env = Env::default();
    let (admin, token) = deploy(&env);

    token.burn(&admin, &400_000);

    assert_eq!(token.balance(&admin), INITIAL_SUPPLY - 400_000);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY - 400_000);
}

#[test]
fn burn_beyond_balance_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let holder = Address::generate(&env);
    token.transfer(&admin, &holder, &1_000);

    assert_eq!(
        token.try_burn(&holder, &1_001),
        Err(Ok(TokenError::InsufficientBalance))
    );
    assert_eq!(token.balance(&holder), 1_000);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

// ─── Authorization ───────────────────────────────────────────────────────────
//
// `deploy` calls mock_all_auths(), which makes every require_auth() succeed.
// These tests clear the mock with set_auths(&[]) so the guards actually run.

#[test]
fn transfer_without_authorization_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    env.set_auths(&[]);

    assert!(token.try_transfer(&admin, &recipient, &1_000).is_err());
    assert_eq!(token.balance(&admin), INITIAL_SUPPLY);
    assert_eq!(token.balance(&recipient), 0);
}

#[test]
fn mint_without_admin_authorization_is_rejected() {
    let env = Env::default();
    let (_, token) = deploy(&env);
    let attacker = Address::generate(&env);

    env.set_auths(&[]);

    // mint() takes no caller argument — it is gated purely by require_auth()
    // on the stored admin, so an unauthorized call cannot satisfy it.
    assert!(token.try_mint(&attacker, &1_000_000).is_err());
    assert_eq!(token.balance(&attacker), 0);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn burn_without_authorization_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);

    env.set_auths(&[]);

    assert!(token.try_burn(&admin, &1_000).is_err());
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn transfer_admin_without_authorization_is_rejected() {
    let env = Env::default();
    let (_, token) = deploy(&env);
    let attacker = Address::generate(&env);

    env.set_auths(&[]);

    assert!(token.try_transfer_admin(&attacker).is_err());
}

// ─── Allowance ───────────────────────────────────────────────────────────────

#[test]
fn approve_records_an_allowance_per_owner_spender_pair() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let other_spender = Address::generate(&env);

    token.approve(&admin, &spender, &50_000);

    assert_eq!(token.allowance(&admin, &spender), 50_000);
    // Allowances are per pair, not per owner.
    assert_eq!(token.allowance(&admin, &other_spender), 0);
    assert_eq!(token.allowance(&spender, &admin), 0);
}

#[test]
fn approve_overwrites_a_previous_allowance() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);

    token.approve(&admin, &spender, &50_000);
    token.approve(&admin, &spender, &10_000);

    assert_eq!(token.allowance(&admin, &spender), 10_000);
}

// ─── Admin ───────────────────────────────────────────────────────────────────

#[test]
fn transfer_admin_hands_minting_rights_to_the_new_admin() {
    let env = Env::default();
    let (_, token) = deploy(&env);
    let new_admin = Address::generate(&env);

    token.transfer_admin(&new_admin);

    // The rights moved: the new admin can mint.
    token.mint(&new_admin, &1_000);
    assert_eq!(token.balance(&new_admin), 1_000);
}

// ─── Checkpoints ─────────────────────────────────────────────────────────────

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
