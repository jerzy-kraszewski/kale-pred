#![no_std]
//! Kale‑Prediction — over/under prediction‑market for **Kale‑contract
//! invocation counts**.
//!
//! * One active round at a time (fits hackathon scope).
//! * Bets are placed in a **SEP‑41 token** chosen at deployment (e.g. KALE).
//! * Losers lose their stake; winners split the total pot proportionally.
//! * If the admin never resolves, participants can refund after a grace
//!   period.
//!
//! Built against **soroban‑sdk 22.0.x**.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
};

// ──────────────────────────────────────────────────────────────────────────
// Storage keys
// ──────────────────────────────────────────────────────────────────────────

#[contracttype]
enum DataKey {
    Admin,
    Token,               // KALE token contract address
    NextRoundId,         // u32 counter
    Round(u32),          // Round data
    Stake(u32, Address), // bettor stakes
}

// ──────────────────────────────────────────────────────────────────────────
// Config
// ──────────────────────────────────────────────────────────────────────────

/// Ledgers after `finality_ledger` before refunds become possible.
const GRACE_LEDGERS: u32 = 100;

// ──────────────────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Side {
    Lower = 0,
    Higher = 1,
}

#[contracttype]
#[derive(Clone)]
pub struct Round {
    // parameters
    predicted_count: u32,
    deadline_ledger: u32,
    finality_ledger: u32,
    // liquidity pools (token minor‑units)
    high_pool: i128,
    low_pool: i128,
    // resolution data
    resolved: bool,
    winning_side: Side, // meaningful only when `resolved == true`
    actual_count: u32,  // idem
}

#[contracttype]
#[derive(Clone, Copy)]
pub struct Stake {
    amount: i128,
    side: Side,
}

// ──────────────────────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1,
    AlreadyInitialised = 2,
    RoundNotFound = 3,
    BettingClosed = 4,
    AlreadyResolved = 5,
    TooEarly = 6,
    NotResolved = 7,
    AlreadyClaimed = 8,
    RefundNotAvailable = 9,
    ZeroAmount = 10,
}

// ──────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────

fn token_client(e: &Env) -> token::Client {
    let addr: Address = e
        .storage()
        .instance()
        .get(&DataKey::Token)
        .expect("token not set");
    token::Client::new(e, &addr)
}

fn get_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("not initialised")
}

// ──────────────────────────────────────────────────────────────────────────
// Contract implementation
// ──────────────────────────────────────────────────────────────────────────

#[contract]
pub struct KalePrediction;

#[contractimpl]
impl KalePrediction {
    // ---------------------------------------------------
    // Admin / init
    // ---------------------------------------------------

    /// Initialise contract with `admin` and **token** used for wagering.
    pub fn __constructor(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(env, Error::AlreadyInitialised);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::NextRoundId, &0u32);
    }

    /// Start a new prediction round.
    pub fn start_round(
        env: Env,
        admin: Address,
        predicted_count: u32,
        deadline_ledger: u32,
        finality_ledger: u32,
    ) -> u32 {
        // auth
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }
        admin.require_auth();

        if deadline_ledger >= finality_ledger {
            panic_with_error!(env, Error::TooEarly);
        }

        // id generation
        let mut next_id: u32 = env.storage().instance().get(&DataKey::NextRoundId).unwrap();
        let round_id = next_id;
        next_id += 1;
        env.storage()
            .instance()
            .set(&DataKey::NextRoundId, &next_id);

        let round = Round {
            predicted_count,
            deadline_ledger,
            finality_ledger,
            high_pool: 0,
            low_pool: 0,
            resolved: false,
            winning_side: Side::Lower, // placeholder
            actual_count: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Round(round_id), &round);

        round_id
    }

    // ---------------------------------------------------
    // Betting
    // ---------------------------------------------------

    pub fn bet(env: Env, player: Address, round_id: u32, side: Side, amount: i128) {
        if amount <= 0 {
            panic_with_error!(env, Error::ZeroAmount);
        }
        player.require_auth();

        // load round
        let mut round: Round = env
            .storage()
            .persistent()
            .get(&DataKey::Round(round_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::RoundNotFound));

        if env.ledger().sequence() > round.deadline_ledger {
            panic_with_error!(env, Error::BettingClosed);
        }

        // transfer stake → contract
        token_client(&env).transfer(&player, &env.current_contract_address(), &amount);

        // update pools
        match side {
            Side::Higher => round.high_pool += amount,
            Side::Lower => round.low_pool += amount,
        }
        env.storage()
            .persistent()
            .set(&DataKey::Round(round_id), &round);

        // upsert stake
        let stake_key = DataKey::Stake(round_id, player.clone());
        let updated_amount = env
            .storage()
            .persistent()
            .get::<DataKey, Stake>(&stake_key)
            .map(|s| s.amount + amount)
            .unwrap_or(amount);
        env.storage().persistent().set(
            &stake_key,
            &Stake {
                amount: updated_amount,
                side,
            },
        );
    }

    // ---------------------------------------------------
    // Resolution
    // ---------------------------------------------------

    pub fn resolve_round(env: Env, admin: Address, round_id: u32, actual_count: u32) {
        // auth
        let stored_admin = get_admin(&env);
        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }
        admin.require_auth();

        let mut round: Round = env
            .storage()
            .persistent()
            .get(&DataKey::Round(round_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::RoundNotFound));

        if env.ledger().sequence() < round.finality_ledger {
            panic_with_error!(env, Error::TooEarly);
        }
        if round.resolved {
            panic_with_error!(env, Error::AlreadyResolved);
        }

        round.winning_side = if actual_count > round.predicted_count {
            Side::Higher
        } else {
            Side::Lower
        };
        round.actual_count = actual_count;
        round.resolved = true;

        env.storage()
            .persistent()
            .set(&DataKey::Round(round_id), &round);
    }

    // ---------------------------------------------------
    // Claim & refund
    // ---------------------------------------------------

    pub fn claim(env: Env, player: Address, round_id: u32) {
        player.require_auth();

        let round: Round = env
            .storage()
            .persistent()
            .get(&DataKey::Round(round_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::RoundNotFound));

        if !round.resolved {
            panic_with_error!(env, Error::NotResolved);
        }

        let stake_key = DataKey::Stake(round_id, player.clone());
        let stake: Stake = env
            .storage()
            .persistent()
            .get(&stake_key)
            .unwrap_or_else(|| panic_with_error!(env, Error::AlreadyClaimed));

        // remove stake first to block re‑entrancy / double claim
        env.storage().persistent().remove(&stake_key);

        if stake.side != round.winning_side {
            return; // loser gets nothing
        }

        let side_pool = match round.winning_side {
            Side::Higher => round.high_pool,
            Side::Lower => round.low_pool,
        };
        let total_pool = round.high_pool + round.low_pool;

        let payout = stake.amount * total_pool / side_pool;
        token_client(&env).transfer(&env.current_contract_address(), &player, &payout);
    }

    /// Refund original stake if admin never resolved within grace period.
    pub fn refund(env: Env, player: Address, round_id: u32) {
        player.require_auth();

        let round: Round = env
            .storage()
            .persistent()
            .get(&DataKey::Round(round_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::RoundNotFound));

        if round.resolved {
            panic_with_error!(env, Error::AlreadyResolved);
        }

        if env.ledger().sequence() <= round.finality_ledger + GRACE_LEDGERS {
            panic_with_error!(env, Error::RefundNotAvailable);
        }

        let stake_key = DataKey::Stake(round_id, player.clone());
        let stake: Stake = env
            .storage()
            .persistent()
            .get(&stake_key)
            .unwrap_or_else(|| panic_with_error!(env, Error::AlreadyClaimed));

        // remove stake first
        env.storage().persistent().remove(&stake_key);

        // transfer original stake back
        token_client(&env).transfer(&env.current_contract_address(), &player, &stake.amount);
    }

    /// Address that was set as admin in the constructor.
    pub fn get_admin(env: Env) -> Address {
        get_admin(&env)
    }

    /// Full `Round` data, or panics with `RoundNotFound` (#3).
    pub fn get_round(env: Env, round_id: u32) -> Round {
        env.storage()
            .persistent()
            .get(&DataKey::Round(round_id))
            .unwrap_or_else(|| panic_with_error!(env, Error::RoundNotFound))
    }

    /// Caller’s stake for a round, or `None` if they never bet.
    pub fn get_stake(env: Env, player: Address, round_id: u32) -> Option<Stake> {
        env.storage()
            .persistent()
            .get(&DataKey::Stake(round_id, player))
    }
}

mod test;
