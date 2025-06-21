#![cfg(test)]

//! **Comprehensive integration tests** for the Kale‑Prediction contract.
//! These spin up an in‑memory SEP‑41 token, mint funds, run full market
//! cycles, and assert balances / error paths.

extern crate std;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::println;

use soroban_sdk::{
    testutils::Ledger,
    testutils::{Address as _, EnvTestConfig},
    token::{self, StellarAssetClient},
    Address, Env,
};

use crate::{KalePrediction, KalePredictionClient, Side, GRACE_LEDGERS};

// ---------------------------------------------------------------------
// Test‑bed bootstrap
// ---------------------------------------------------------------------

/// Builds a fresh environment with:
/// * an on‑the‑fly SEP‑41 token contract (mint authority held by `token_admin`)
/// * a deployed and initialised Kale‑Prediction contract using that token.
fn setup() -> (
    Env,
    StellarAssetClient<'static>, // mint‑only helper
    token::Client<'static>,      // generic token client for balance checks
    KalePredictionClient<'static>,
    Address, // admin
) {
    let mut env = Env::default();
    env.mock_all_auths();
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });

    // ── 1. Create KALE test token ────────────────────────────────────
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();

    let mint_client = StellarAssetClient::new(&env, &token_addr);
    let token_client = token::Client::new(&env, &token_addr);

    // ── 2. Deploy Kale‑Prediction ────────────────────────────────────
    let admin = Address::generate(&env);
    // pass constructor arguments directly when registering (best‑practice)
    let contract_id = env.register(KalePrediction, (&admin, &token_addr));
    let kp_client = KalePredictionClient::new(&env, &contract_id);

    (env, mint_client, token_client, kp_client, admin)
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

/// Happy‑path: bets, resolution, and correct payouts.
#[test]
fn happy_path_claims() {
    let (env, mint, tok, kp, admin) = setup();

    // current ledger
    let cur = env.ledger().sequence();
    let deadline = cur + 5;
    let finality = cur + 10;

    let round_id = kp.start_round(&admin, &100u32, &deadline, &finality);

    // bettors
    let alice = Address::generate(&env); // winner
    let bob = Address::generate(&env); // loser

    mint.mint(&alice, &100);
    mint.mint(&bob, &300);

    kp.bet(&alice, &round_id, &Side::Higher, &100);
    kp.bet(&bob, &round_id, &Side::Lower, &300);

    env.ledger().set_sequence_number(finality + 1);

    // actual count higher than predicted ⇒ Higher wins
    kp.resolve_round(&admin, &round_id, &150u32);

    let bal_a_before = tok.balance(&alice);
    let bal_b_before = tok.balance(&bob);

    kp.claim(&alice, &round_id);
    kp.claim(&bob, &round_id);

    let bal_a_after = tok.balance(&alice);
    let bal_b_after = tok.balance(&bob);

    assert_eq!(bal_a_after - bal_a_before, 400); // 100 + 300
    assert_eq!(bal_b_after - bal_b_before, 0);

    println!("✅ happy_path_claims passed");
}

/// Refund after grace period if admin never resolves.
#[test]
fn refund_after_grace() {
    let (env, mint, tok, kp, admin) = setup();
    let cur = env.ledger().sequence();

    let deadline = cur + 3;
    let finality = cur + 6;
    let round_id = kp.start_round(&admin, &42u32, &deadline, &finality);

    let carol = Address::generate(&env);
    mint.mint(&carol, &150);

    kp.bet(&carol, &round_id, &Side::Lower, &150);

    // move to just before grace expiry – refund should panic
    env.ledger()
        .set_sequence_number(finality + GRACE_LEDGERS - 1);
    assert!(catch_unwind(AssertUnwindSafe(|| { kp.refund(&carol, &round_id) })).is_err());

    // past grace
    env.ledger()
        .set_sequence_number(finality + GRACE_LEDGERS + 1);
    let bal_before = tok.balance(&carol);
    kp.refund(&carol, &round_id);
    let bal_after = tok.balance(&carol);
    assert_eq!(bal_after - bal_before, 150);

    println!("✅ refund_after_grace passed");
}

/// Two winners split the loser pot proportionally to their stake.
#[test]
fn proportional_split_two_winners() {
    let (env, mint, tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let deadline = cur + 4;
    let finality = cur + 8;
    let round_id = kp.start_round(&admin, &120u32, &deadline, &finality);

    // players
    let alice = Address::generate(&env); // Lower winner, stake 100 (25% of winner pool)
    let bob = Address::generate(&env); // Lower winner, stake 300 (75% of winner pool)
    let charlie = Address::generate(&env); // Higher loser, stake 400

    mint.mint(&alice, &100);
    mint.mint(&bob, &300);
    mint.mint(&charlie, &400);

    // place bets
    kp.bet(&alice, &round_id, &Side::Lower, &100);
    kp.bet(&bob, &round_id, &Side::Lower, &300);
    kp.bet(&charlie, &round_id, &Side::Higher, &400);

    // move past finality and resolve with actual LOWER than predicted
    env.ledger().set_sequence_number(finality + 1);
    kp.resolve_round(&admin, &round_id, &100u32); // actual < predicted ⇒ Lower wins

    // balances before claims are 0 because stakes are locked
    assert_eq!(tok.balance(&alice), 0);
    assert_eq!(tok.balance(&bob), 0);
    assert_eq!(tok.balance(&charlie), 0);

    // winners claim
    kp.claim(&alice, &round_id);
    kp.claim(&bob, &round_id);

    // loser claim (should do nothing but allowed)
    kp.claim(&charlie, &round_id);

    // Expected payouts:
    // Winner pool = 100 + 300 = 400
    // Total pool   = 800
    // Alice payout = 100 * 800 / 400 = 200
    // Bob   payout = 300 * 800 / 400 = 600

    assert_eq!(tok.balance(&alice), 200);
    assert_eq!(tok.balance(&bob), 600);
    assert_eq!(tok.balance(&charlie), 0);

    println!("✅ proportional_split_two_winners passed");
}

// ---------------------------------------------------------------------
// Error‑coverage tests (one per Error::* variant)
// ---------------------------------------------------------------------

/// Non‑admin attempts to start and resolve ➜ `Unauthorized` (#1).
#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn unauthorized_admin_calls() {
    let (env, _mint, _tok, kp, _admin) = setup();
    let eve = Address::generate(&env);
    let cur = env.ledger().sequence();
    let deadline = cur + 2;
    let finality = cur + 4;
    // eve tries to start
    kp.start_round(&eve, &1u32, &deadline, &finality);
}

/// Claiming an unknown round ➜ `RoundNotFound` (#3).
#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn round_not_found_claim() {
    let (env, _mint, _tok, kp, _admin) = setup();
    let frank = Address::generate(&env); // Higher loser, stake 400
    kp.claim(&frank, &999u32);
}

/// Betting after the deadline must fail ➜ `BettingClosed` (#4).
#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn bet_after_deadline_panics() {
    let (env, mint, _tok, kp, admin) = setup();

    let cur = env.ledger().sequence();
    let deadline = cur + 1;
    let finality = cur + 10;
    let round_id = kp.start_round(&admin, &10u32, &deadline, &finality);

    let dave = Address::generate(&env);
    mint.mint(&dave, &10);

    env.ledger().set_sequence_number(deadline + 1); // after deadline
    kp.bet(&dave, &round_id, &Side::Higher, &10); // should panic
}

/// Admin resolves twice ➜ `AlreadyResolved` (#5).
#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn resolve_twice_panics() {
    let (env, _mint, _tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let round = kp.start_round(&admin, &1u32, &(cur + 1), &(cur + 2));
    env.ledger().set_sequence_number(cur + 3);
    kp.resolve_round(&admin, &round, &2u32);
    kp.resolve_round(&admin, &round, &3u32); // second time
}

/// Resolving before finality ➜ `TooEarly` (#6).
#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn resolve_too_early_panics() {
    let (env, _mint, _tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let round = kp.start_round(&admin, &1u32, &(cur + 5), &(cur + 10));
    env.ledger().set_sequence_number(cur + 6); // before finality
    kp.resolve_round(&admin, &round, &0u32);
}

/// Claim before resolution ➜ `NotResolved` (#7).
#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn claim_not_resolved_panics() {
    let (env, mint, _tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let round = kp.start_round(&admin, &1u32, &(cur + 1), &(cur + 3));
    let alice = Address::generate(&env);
    mint.mint(&alice, &1);
    kp.bet(&alice, &round, &Side::Higher, &1);
    env.ledger().set_sequence_number(cur + 2);
    kp.claim(&alice, &round);
}

/// Double claim ➜ `AlreadyClaimed` (#8).
#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn double_claim_panics() {
    let (env, mint, _tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let round = kp.start_round(&admin, &1u32, &(cur + 1), &(cur + 2));
    let alice = Address::generate(&env);
    mint.mint(&alice, &1);
    kp.bet(&alice, &round, &Side::Higher, &1);
    env.ledger().set_sequence_number(cur + 3);
    kp.resolve_round(&admin, &round, &2u32);
    kp.claim(&alice, &round);
    kp.claim(&alice, &round); // second claim
}

/// Refund attempt before grace period ➜ `RefundNotAvailable` (#9).
#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn refund_before_grace_panics() {
    let (env, mint, _tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let deadline = cur + 2;
    let finality = cur + 4;
    let round = kp.start_round(&admin, &1u32, &deadline, &finality);
    let frank = Address::generate(&env);
    mint.mint(&frank, &10);
    kp.bet(&frank, &round, &Side::Higher, &10);

    // move to just after finality but before grace period ends
    env.ledger().set_sequence_number(finality + 1);
    kp.refund(&frank, &round); // should panic with #9
}

/// Bet amount zero ➜ `ZeroAmount` (#10).
#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn zero_amount_panics() {
    let (env, _mint, _tok, kp, admin) = setup();
    let cur = env.ledger().sequence();
    let round = kp.start_round(&admin, &1u32, &(cur + 1), &(cur + 2));
    let alice = Address::generate(&env);
    kp.bet(&alice, &round, &Side::Higher, &0);
}
