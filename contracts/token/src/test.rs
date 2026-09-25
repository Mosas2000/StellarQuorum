use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{IntoVal, TryFromVal, Val};

const INITIAL_SUPPLY: i128 = 1_000_000;
/// Expiry far enough out that approvals stay live for tests that are not about
/// expiry.
const FAR_FUTURE: u32 = 1_000_000;

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

// ─── transfer_from ───────────────────────────────────────────────────────────

#[test]
fn transfer_from_spends_an_approved_allowance() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);
    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    token.transfer_from(&spender, &admin, &recipient, &20_000);

    assert_eq!(token.balance(&admin), INITIAL_SUPPLY - 20_000);
    assert_eq!(token.balance(&recipient), 20_000);
    // Allowance is reduced by exactly the amount spent.
    assert_eq!(token.allowance(&admin, &spender), 30_000);
    assert_eq!(token.total_supply(), INITIAL_SUPPLY);
}

#[test]
fn transfer_from_can_spend_the_allowance_exactly() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);
    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    token.transfer_from(&spender, &admin, &recipient, &50_000);

    assert_eq!(token.allowance(&admin, &spender), 0);
    assert_eq!(token.balance(&recipient), 50_000);

    // Drained, so a further spend of even 1 is refused.
    assert_eq!(
        token.try_transfer_from(&spender, &admin, &recipient, &1),
        Err(Ok(TokenError::InsufficientAllowance))
    );
}

#[test]
fn transfer_from_beyond_the_allowance_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);
    token.approve(&admin, &spender, &10_000, &FAR_FUTURE);

    assert_eq!(
        token.try_transfer_from(&spender, &admin, &recipient, &10_001),
        Err(Ok(TokenError::InsufficientAllowance))
    );
    assert_eq!(token.allowance(&admin, &spender), 10_000);
    assert_eq!(token.balance(&recipient), 0);
}

#[test]
fn transfer_from_without_any_allowance_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let stranger = Address::generate(&env);

    assert_eq!(
        token.try_transfer_from(&stranger, &admin, &stranger, &1),
        Err(Ok(TokenError::InsufficientAllowance))
    );
    assert_eq!(token.balance(&admin), INITIAL_SUPPLY);
}

#[test]
fn transfer_from_beyond_the_owner_balance_leaves_the_allowance_intact() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);

    // Approved for more than the owner actually holds.
    token.transfer(&admin, &owner, &5_000);
    token.approve(&owner, &spender, &50_000, &FAR_FUTURE);

    assert_eq!(
        token.try_transfer_from(&spender, &owner, &recipient, &6_000),
        Err(Ok(TokenError::InsufficientBalance))
    );
    // A failed spend must not burn allowance.
    assert_eq!(token.allowance(&owner, &spender), 50_000);
    assert_eq!(token.balance(&owner), 5_000);
}

#[test]
fn transfer_from_of_a_non_positive_amount_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    token.approve(&admin, &spender, &10_000, &FAR_FUTURE);

    assert_eq!(
        token.try_transfer_from(&spender, &admin, &spender, &0),
        Err(Ok(TokenError::InvalidAmount))
    );
    assert_eq!(
        token.try_transfer_from(&spender, &admin, &spender, &-5),
        Err(Ok(TokenError::InvalidAmount))
    );
}

#[test]
fn transfer_from_emits_a_transfer_event_naming_the_owner_not_the_spender() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);
    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    token.transfer_from(&spender, &admin, &recipient, &20_000);

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "transfer"), admin.clone(), recipient.clone()).into_val(&env)
    );
    assert_eq!(
        Transfer::try_from_val(&env, &data).unwrap(),
        Transfer { from: admin, to: recipient, amount: 20_000 }
    );
}

#[test]
fn transfer_from_checkpoints_balances_for_snapshot_voting() {
    let env = Env::default();
    env.ledger().set_sequence_number(10);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);
    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    env.ledger().set_sequence_number(20);
    token.transfer_from(&spender, &admin, &recipient, &20_000);

    // An allowance spend must move voting power like any other transfer.
    assert_eq!(token.get_past_balance(&recipient, &19), 0);
    assert_eq!(token.get_past_balance(&recipient, &20), 20_000);
}

#[test]
fn transfer_from_without_spender_authorization_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);
    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    env.set_auths(&[]);

    assert!(token
        .try_transfer_from(&spender, &admin, &recipient, &1_000)
        .is_err());
    assert_eq!(token.balance(&recipient), 0);
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

// ─── Events ──────────────────────────────────────────────────────────────────
//
// The test env exposes only the most recent invocation's events, so each of
// these reads immediately after the emitting call.

fn last_event(env: &Env) -> (soroban_sdk::Vec<Val>, Val) {
    let (_, topics, data) = env
        .events()
        .all()
        .last()
        .expect("expected at least one event");
    (topics, data)
}

#[test]
fn initialize_emits_the_genesis_mint() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_id = env.register(QuorumToken, ());
    QuorumTokenClient::new(&env, &token_id).initialize(
        &admin,
        &String::from_str(&env, "Quorum"),
        &String::from_str(&env, "QUORUM"),
        &7,
        &INITIAL_SUPPLY,
    );

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "mint"), admin.clone()).into_val(&env)
    );
    assert_eq!(
        Mint::try_from_val(&env, &data).unwrap(),
        Mint {
            to: admin,
            amount: INITIAL_SUPPLY,
            total_supply: INITIAL_SUPPLY,
        }
    );
}

#[test]
fn transfer_emits_a_transfer_event() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    token.transfer(&admin, &recipient, &250_000);

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "transfer"), admin.clone(), recipient.clone()).into_val(&env)
    );
    assert_eq!(
        Transfer::try_from_val(&env, &data).unwrap(),
        Transfer { from: admin, to: recipient, amount: 250_000 }
    );
}

#[test]
fn mint_emits_the_supply_after_the_mint() {
    let env = Env::default();
    let (_, token) = deploy(&env);
    let recipient = Address::generate(&env);

    token.mint(&recipient, &300_000);

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "mint"), recipient.clone()).into_val(&env)
    );
    assert_eq!(
        Mint::try_from_val(&env, &data).unwrap(),
        Mint {
            to: recipient,
            amount: 300_000,
            total_supply: INITIAL_SUPPLY + 300_000,
        }
    );
}

#[test]
fn burn_emits_the_supply_after_the_burn() {
    let env = Env::default();
    let (admin, token) = deploy(&env);

    token.burn(&admin, &400_000);

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "burn"), admin.clone()).into_val(&env)
    );
    assert_eq!(
        Burn::try_from_val(&env, &data).unwrap(),
        Burn {
            from: admin,
            amount: 400_000,
            total_supply: INITIAL_SUPPLY - 400_000,
        }
    );
}

#[test]
fn approve_emits_an_approve_event() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);

    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "approve"), admin.clone(), spender.clone()).into_val(&env)
    );
    assert_eq!(
        Approve::try_from_val(&env, &data).unwrap(),
        Approve { owner: admin, spender, amount: 50_000, expiration_ledger: FAR_FUTURE }
    );
}

#[test]
fn transfer_admin_emits_both_sides_of_the_handover() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let new_admin = Address::generate(&env);

    token.transfer_admin(&new_admin);

    let (topics, data) = last_event(&env);
    assert_eq!(
        topics,
        (Symbol::new(&env, "admin_transferred"), admin.clone()).into_val(&env)
    );
    assert_eq!(
        AdminTransferred::try_from_val(&env, &data).unwrap(),
        AdminTransferred { previous_admin: admin, new_admin }
    );
}

#[test]
fn a_rejected_transfer_emits_nothing() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let recipient = Address::generate(&env);

    // A successful transfer leaves exactly its own event behind...
    token.transfer(&admin, &recipient, &1_000);
    assert_eq!(env.events().all().len(), 1);

    // ...while a rejected one leaves the log empty, since a failed invocation
    // rolls back its events along with its state.
    assert!(token
        .try_transfer(&admin, &recipient, &(INITIAL_SUPPLY + 1))
        .is_err());
    assert!(env.events().all().is_empty());
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

    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

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

    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);
    token.approve(&admin, &spender, &10_000, &FAR_FUTURE);

    assert_eq!(token.allowance(&admin, &spender), 10_000);
}

// ─── Allowance expiry ────────────────────────────────────────────────────────

#[test]
fn an_allowance_reads_as_zero_once_its_expiry_has_passed() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);

    token.approve(&admin, &spender, &50_000, &200);

    // Live right up to and including the expiry ledger.
    env.ledger().set_sequence_number(200);
    assert_eq!(token.allowance(&admin, &spender), 50_000);

    // Lapsed the ledger after.
    env.ledger().set_sequence_number(201);
    assert_eq!(token.allowance(&admin, &spender), 0);
}

#[test]
fn transfer_from_with_an_expired_allowance_is_rejected() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);

    token.approve(&admin, &spender, &50_000, &200);

    env.ledger().set_sequence_number(201);
    assert_eq!(
        token.try_transfer_from(&spender, &admin, &recipient, &1_000),
        Err(Ok(TokenError::InsufficientAllowance))
    );
    assert_eq!(token.balance(&recipient), 0);
    assert_eq!(token.balance(&admin), INITIAL_SUPPLY);
}

#[test]
fn an_allowance_is_spendable_right_up_to_its_expiry() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);

    token.approve(&admin, &spender, &50_000, &200);

    env.ledger().set_sequence_number(200);
    token.transfer_from(&spender, &admin, &recipient, &50_000);

    assert_eq!(token.balance(&recipient), 50_000);
}

#[test]
fn spending_part_of_an_allowance_does_not_extend_the_remainder() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);

    token.approve(&admin, &spender, &50_000, &200);

    env.ledger().set_sequence_number(150);
    token.transfer_from(&spender, &admin, &recipient, &20_000);
    assert_eq!(token.allowance(&admin, &spender), 30_000);

    // The remaining 30_000 still dies at the original expiry, not 50 ledgers
    // after the partial spend.
    env.ledger().set_sequence_number(201);
    assert_eq!(token.allowance(&admin, &spender), 0);
}

#[test]
fn an_approval_that_expires_in_the_past_is_rejected() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);

    assert_eq!(
        token.try_approve(&admin, &spender, &50_000, &99),
        Err(Ok(TokenError::InvalidExpiration))
    );
    // A zero expiry is in the past for any real ledger, so it is refused too.
    assert_eq!(
        token.try_approve(&admin, &spender, &50_000, &0),
        Err(Ok(TokenError::InvalidExpiration))
    );
    assert_eq!(token.allowance(&admin, &spender), 0);
}

#[test]
fn an_approval_expiring_on_the_current_ledger_is_accepted() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);

    // Valid for the remainder of this ledger only.
    token.approve(&admin, &spender, &50_000, &100);
    assert_eq!(token.allowance(&admin, &spender), 50_000);

    env.ledger().set_sequence_number(101);
    assert_eq!(token.allowance(&admin, &spender), 0);
}

#[test]
fn a_zero_amount_revokes_an_allowance_regardless_of_expiry() {
    let env = Env::default();
    env.ledger().set_sequence_number(100);
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);
    token.approve(&admin, &spender, &50_000, &FAR_FUTURE);

    // Revocation carries no live amount, so SEP-41's expiry rule does not
    // apply — an owner must always be able to cancel an approval.
    token.approve(&admin, &spender, &0, &0);

    assert_eq!(token.allowance(&admin, &spender), 0);
    assert_eq!(
        token.try_transfer_from(&spender, &admin, &spender, &1),
        Err(Ok(TokenError::InsufficientAllowance))
    );
}

#[test]
fn a_negative_approval_is_rejected() {
    let env = Env::default();
    let (admin, token) = deploy(&env);
    let spender = Address::generate(&env);

    assert_eq!(
        token.try_approve(&admin, &spender, &-1, &FAR_FUTURE),
        Err(Ok(TokenError::InvalidAmount))
    );
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

// ─── Property test: binary search vs. naive linear scan (#161) ───────────────

/// Minimal, dependency-free xorshift32 PRNG. Deterministic (fixed seed) so
/// CI failures are reproducible, but still exercises a wide spread of
/// checkpoint-history shapes without adding a `proptest`/`rand` dependency.
struct Xorshift32(u32);

impl Xorshift32 {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Random value in `0..bound`. `bound` must be > 0.
    fn next_below(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }
}

/// Naive O(n) scan mirroring the contract's contract: the balance in effect
/// at `ledger` is the most recent checkpoint at or before it, or 0 if none
/// qualifies (including an empty history).
fn naive_past_balance(history: &std::vec::Vec<(u32, i128)>, ledger: u32) -> i128 {
    history
        .iter()
        .rev()
        .find(|(l, _)| *l <= ledger)
        .map(|(_, balance)| *balance)
        .unwrap_or(0)
}

#[test]
fn get_past_balance_matches_naive_scan_across_random_histories() {
    let mut rng = Xorshift32(0x9E3779B9);

    for _case in 0..50 {
        let env = Env::default();
        env.ledger().set_sequence_number(1);
        let (_admin, token) = deploy(&env);
        let holder = Address::generate(&env);

        // 0..=25 checkpoints per history, so empty and single-entry histories
        // are both represented across the 50 generated cases.
        let len = rng.next_below(26);
        let mut history: std::vec::Vec<(u32, i128)> = std::vec::Vec::new();
        let mut ledger = 1u32;
        let mut balance: i128 = 0;

        for _ in 0..len {
            ledger += 1 + rng.next_below(20);
            env.ledger().set_sequence_number(ledger);
            let delta = 1 + i128::from(rng.next_below(10_000));
            token.mint(&holder, &delta);
            balance += delta;
            history.push((ledger, balance));
        }

        // Query well before, exactly on, one below, one above every
        // checkpoint, plus random ledgers across the whole range — binary
        // search off-by-ones hide exactly at those boundaries.
        let mut queries: std::vec::Vec<u32> = std::vec![0, 1, ledger, ledger + 1000];
        for &(l, _) in history.iter() {
            queries.push(l);
            queries.push(l.saturating_sub(1));
            queries.push(l + 1);
        }
        for _ in 0..10 {
            queries.push(rng.next_below(ledger + 50));
        }

        for query in queries {
            let expected = naive_past_balance(&history, query);
            assert_eq!(
                token.get_past_balance(&holder, &query),
                expected,
                "history={:?} query={}",
                history,
                query,
            );
        }
    }
}

#[test]
fn get_past_balance_with_empty_history_is_always_zero() {
    let env = Env::default();
    env.ledger().set_sequence_number(1);
    let (_admin, token) = deploy(&env);
    let stranger = Address::generate(&env);

    for query in [0u32, 1, 10, 1_000_000] {
        assert_eq!(token.get_past_balance(&stranger, &query), 0);
    }
}

#[test]
fn get_past_balance_with_single_checkpoint() {
    let env = Env::default();
    env.ledger().set_sequence_number(1);
    let (admin, token) = deploy(&env);
    let holder = Address::generate(&env);

    env.ledger().set_sequence_number(50);
    token.transfer(&admin, &holder, &777);

    assert_eq!(token.get_past_balance(&holder, &49), 0);
    assert_eq!(token.get_past_balance(&holder, &50), 777);
    assert_eq!(token.get_past_balance(&holder, &51), 777);
    assert_eq!(token.get_past_balance(&holder, &1_000), 777);
}
