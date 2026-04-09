#![no_std]

//! # SoroStream — Real-time Token Streaming & Vesting Protocol
//!
//! This contract implements a **token streaming** primitive on Stellar Soroban.
//! A *stream* continuously makes tokens claimable by a recipient over a fixed
//! time window (`start_time` → `end_time`).  The sender deposits the total
//! amount upfront; the recipient can call `claim_stream` at any ledger to
//! withdraw whatever proportion of the stream has elapsed.
//!
//! ## Architecture Overview
//!
//! ```
//! ┌──────────────────────────────┐
//! │  SoroStream Contract         │
//! │                              │
//! │  initialize()                │  ← one-time admin setup
//! │  create_stream()             │  ← sender locks tokens
//! │  claim_stream()              │  ← recipient pulls vested tokens
//! │  cancel_stream()  [TODO]     │  ← sender cancels; split refund
//! │  get_stream()                │  ← read-only query
//! └──────────────────────────────┘
//! ```
//!
//! ## Contributor Guide
//!
//! Functions marked `// TODO(contributor):` are intentional gaps left for
//! open-source contributors to implement.  Each TODO block has a complexity
//! label:
//!   - `[trivial]`  — documentation, constant tweaks, small helpers
//!   - `[medium]`   — maths, new entrypoints, frontend integration
//!   - `[high]`     — security-critical paths, edge-case hardening

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env,
};

// ─────────────────────────────────────────────────────────────────────────────
// Error Codes
// ─────────────────────────────────────────────────────────────────────────────

/// Typed error codes returned by contract methods.
///
/// Using an enum (rather than panic strings) lets callers distinguish failure
/// modes without matching on string literals.
///
/// # Contributor Note [trivial]
/// Add new variants here as you implement new failure paths.  Keep them short
/// so they fit inside Soroban's XDR `Error` type.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SoroError {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    StreamNotFound     = 3,
    Unauthorized       = 4,
    InvalidAmount      = 5,
    InvalidTimeRange   = 6,
    NothingToClaim     = 7,
    StreamEnded        = 8,
}

// ─────────────────────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a single vesting / streaming position.
///
/// All timestamps use Soroban's `current_ledger().timestamp()` (Unix seconds).
/// All amounts are in the *smallest unit* of the chosen token (e.g. stroops for
/// XLM, or 7-decimal units for USDC on Stellar).
///
/// # Contributor Note [medium]
/// Consider adding a `cliff_time: u64` field so tokens are locked until a cliff
/// has passed (useful for employee vesting schedules).  That's a good first
/// medium-complexity issue.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stream {
    /// Unique identifier for this stream (auto-incremented counter).
    pub id: u64,
    /// The address that funded the stream (token sender).
    pub sender: Address,
    /// The address that will receive the streamed tokens.
    pub recipient: Address,
    /// The SAC (Stellar Asset Contract) address of the streamed token.
    pub token: Address,
    /// Total tokens locked at stream creation (in token's smallest unit).
    pub total_amount: i128,
    /// Tokens already claimed by the recipient.
    pub claimed_amount: i128,
    /// Unix timestamp at which streaming begins.
    pub start_time: u64,
    /// Unix timestamp at which the stream is fully vested.
    pub end_time: u64,
    /// Whether the stream has been cancelled.
    pub is_cancelled: bool,
}

/// Storage key enum.  Every piece of persistent state is keyed by one of these
/// variants so there are no accidental key collisions.
#[contracttype]
pub enum DataKey {
    /// Whether the contract has been initialized.
    Init,
    /// The protocol-level admin address (can pause/upgrade in future).
    Admin,
    /// Auto-incrementing counter used as stream IDs.
    NextStreamId,
    /// The `Stream` struct for a given stream ID.
    Stream(u64),
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract
// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct SoroStream;

#[contractimpl]
impl SoroStream {
    // ─────────────────────────────────────────────────────────────────────────
    // Admin / Initialization
    // ─────────────────────────────────────────────────────────────────────────

    /// Initialize the SoroStream protocol.
    ///
    /// Must be called exactly once after deployment.  Sets the protocol `admin`
    /// address that will govern future upgrades or emergency pauses.
    ///
    /// # Panics
    /// - If the contract has already been initialized.
    ///
    /// # Contributor Note [trivial]
    /// The `admin` address is stored but never checked in v0.1 — a future PR
    /// should add `pause()` / `unpause()` admin functions that gate on this.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().persistent().has(&DataKey::Init) {
            panic!("already initialized");
        }

        admin.require_auth();

        env.storage().persistent().set(&DataKey::Init, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::NextStreamId, &0_u64);

        env.events()
            .publish((symbol_short!("protocol"), symbol_short!("init")), admin);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Stream Lifecycle
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new token stream from `sender` to `recipient`.
    ///
    /// The sender must pre-approve this contract to spend `amount` tokens via
    /// the SAC's `approve()` call before invoking `create_stream`.
    ///
    /// # Parameters
    /// - `sender`      — Address funding the stream (must sign the transaction).
    /// - `recipient`   — Address that can claim tokens as they vest.
    /// - `token`       — SAC address of the token to stream.
    /// - `amount`      — Total tokens to lock (must be > 0).
    /// - `start_time`  — Unix timestamp when streaming begins (≥ current time).
    /// - `end_time`    — Unix timestamp when streaming ends (> start_time).
    ///
    /// # Returns
    /// The unique `stream_id` assigned to the new stream.
    ///
    /// # Panics
    /// - `amount <= 0`
    /// - `end_time <= start_time`
    /// - `start_time < current ledger timestamp`
    ///
    /// # Contributor Note [medium]
    /// Implement a **protocol fee** here: deduct a small percentage of `amount`
    /// and transfer it to the `Admin` treasury before locking the rest.
    /// See issue template for the "protocol fee" medium issue.
    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        start_time: u64,
        end_time: u64,
    ) -> u64 {
        // ── Gate: contract must be initialized ────────────────────────────
        if !env.storage().persistent().has(&DataKey::Init) {
            panic!("contract not initialized");
        }

        // ── Require the sender's signature for this transaction ───────────
        sender.require_auth();

        // ── Input validation ──────────────────────────────────────────────
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let now = env.ledger().timestamp();

        if start_time < now {
            panic!("start_time must be in the future");
        }

        if end_time <= start_time {
            panic!("end_time must be after start_time");
        }

        // ── Pull tokens from sender into the contract ─────────────────────
        // Soroban's token interface: sender must have approved this contract
        // for at least `amount` prior to this call.
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        // ── Assign stream ID and persist ──────────────────────────────────
        let stream_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextStreamId)
            .unwrap_or(0);

        let stream = Stream {
            id: stream_id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            total_amount: amount,
            claimed_amount: 0,
            start_time,
            end_time,
            is_cancelled: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        env.storage()
            .persistent()
            .set(&DataKey::NextStreamId, &(stream_id + 1));

        env.events().publish(
            (symbol_short!("stream"), symbol_short!("created")),
            (stream_id, sender, recipient, token, amount, start_time, end_time),
        );

        stream_id
    }

    /// Claim all currently vested tokens from a stream.
    ///
    /// The claimable amount is computed as:
    ///
    /// ```text
    /// elapsed      = min(now, end_time) - start_time
    /// duration     = end_time - start_time
    /// vested       = total_amount * elapsed / duration
    /// claimable    = vested - claimed_amount
    /// ```
    ///
    /// # Parameters
    /// - `stream_id`   — ID returned by `create_stream`.
    /// - `recipient`   — Must match the stream's `recipient`; must sign.
    ///
    /// # Returns
    /// The amount of tokens transferred to the recipient in this call.
    ///
    /// # Panics
    /// - Stream not found.
    /// - Caller is not the stream's recipient.
    /// - Nothing to claim yet (stream hasn't started or all tokens already claimed).
    /// - Stream has been cancelled.
    ///
    /// # Contributor Note [high]
    /// The linear vesting formula above is intentionally simple.  A high-value
    /// contribution would add support for **non-linear curves** (e.g. exponential
    /// unlock, step-cliff schedules).  This requires a new `curve_type` field on
    /// `Stream` and a matching calculation function.  Security review required
    /// for the maths — see the high-complexity issue template.
    pub fn claim_stream(env: Env, stream_id: u64, recipient: Address) -> i128 {
        recipient.require_auth();

        // ── Load stream ───────────────────────────────────────────────────
        let mut stream: Stream = env
            .storage()
            .persistent()
            .get(&DataKey::Stream(stream_id))
            .expect("stream not found");

        // ── Authorization & state guards ──────────────────────────────────
        if stream.recipient != recipient {
            panic!("caller is not the stream recipient");
        }

        if stream.is_cancelled {
            panic!("stream has been cancelled");
        }

        // ── Vesting calculation ───────────────────────────────────────────
        let now = env.ledger().timestamp();

        // TODO(contributor [medium]): Replace the linear formula below with a
        // pluggable curve system.  The current implementation is intentionally
        // naïve to keep the first version auditable.
        let claimable = Self::_compute_claimable(&stream, now);

        if claimable == 0 {
            panic!("nothing to claim");
        }

        // ── Checks-Effects-Interactions pattern ───────────────────────────
        // Update state BEFORE transferring to prevent re-entrancy.
        stream.claimed_amount += claimable;
        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        // ── Token transfer ────────────────────────────────────────────────
        let token_client = token::Client::new(&env, &stream.token);
        token_client.transfer(&env.current_contract_address(), &recipient, &claimable);

        env.events().publish(
            (symbol_short!("stream"), symbol_short!("claimed")),
            (stream_id, recipient, claimable),
        );

        claimable
    }

    /// Cancel a stream early.  Sends the recipient their vested portion and
    /// returns the unvested remainder to the sender.
    ///
    /// # Contributor Note [high]
    /// This function skeleton is provided; the split-refund logic is **not yet
    /// implemented**.  This is the primary high-complexity open issue.
    /// Requirements:
    /// 1. Only the `sender` may cancel (require_auth).
    /// 2. Compute `claimable` (vested but not yet claimed) for recipient.
    /// 3. Compute `refund = total_amount - claimed_amount - claimable` for sender.
    /// 4. Transfer both amounts atomically (CEI pattern).
    /// 5. Mark `stream.is_cancelled = true` and write back to storage.
    /// 6. Emit a `(stream, cancelled)` event.
    pub fn cancel_stream(env: Env, stream_id: u64, sender: Address) {
        sender.require_auth();

        let mut stream: Stream = env
            .storage()
            .persistent()
            .get(&DataKey::Stream(stream_id))
            .expect("stream not found");

        if stream.sender != sender {
            panic!("caller is not the stream sender");
        }

        if stream.is_cancelled {
            panic!("stream has been cancelled");
        }

        let now = env.ledger().timestamp();
        let recipient_claimable = Self::_compute_claimable(&stream, now);
        let sender_refund = stream.total_amount - stream.claimed_amount - recipient_claimable;

        // CEI pattern: state update BEFORE transfer
        stream.claimed_amount += recipient_claimable;
        stream.is_cancelled = true;
        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        let token_client = token::Client::new(&env, &stream.token);
        
        if recipient_claimable > 0 {
            token_client.transfer(&env.current_contract_address(), &stream.recipient, &recipient_claimable);
        }
        
        if sender_refund > 0 {
            token_client.transfer(&env.current_contract_address(), &stream.sender, &sender_refund);
        }

        env.events().publish(
            (symbol_short!("stream"), symbol_short!("cancelled")),
            (stream_id, sender_refund, recipient_claimable),
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Read-Only Queries
    // ─────────────────────────────────────────────────────────────────────────

    /// Return the full `Stream` struct for a given `stream_id`.
    pub fn get_stream(env: Env, stream_id: u64) -> Stream {
        env.storage()
            .persistent()
            .get(&DataKey::Stream(stream_id))
            .expect("stream not found")
    }

    /// Return the currently claimable token amount without writing state.
    ///
    /// Useful for frontend display: call this via `simulateTransaction` to
    /// show live vesting progress without spending gas.
    pub fn claimable_amount(env: Env, stream_id: u64) -> i128 {
        let stream: Stream = env
            .storage()
            .persistent()
            .get(&DataKey::Stream(stream_id))
            .expect("stream not found");

        let now = env.ledger().timestamp();
        Self::_compute_claimable(&stream, now)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Compute the claimable (vested but unclaimed) amount for a stream at
    /// ledger timestamp `now`.
    ///
    /// Formula: linear interpolation between `start_time` and `end_time`.
    ///
    /// # Contributor Note [medium]
    /// Replace or extend this function to support non-linear vesting curves.
    fn _compute_claimable(stream: &Stream, now: u64) -> i128 {
        if now <= stream.start_time {
            return 0;
        }

        let effective_now = now.min(stream.end_time);
        let elapsed = (effective_now - stream.start_time) as i128;
        let duration = (stream.end_time - stream.start_time) as i128;

        // Integer arithmetic — multiply first to preserve precision.
        let vested = stream.total_amount * elapsed / duration;
        let claimable = vested - stream.claimed_amount;

        claimable.max(0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{token, Env};

    // ── Test Harness ──────────────────────────────────────────────────────────

    struct Setup {
        env: Env,
        client: SoroStreamClient<'static>,
        token: token::StellarAssetClient<'static>,
        admin: Address,
        sender: Address,
        recipient: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        // ── Deploy a mock SAC token ───────────────────────────────────────
        let admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(admin.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_id.address());

        // ── Deploy SoroStream contract ────────────────────────────────────
        let contract_id = env.register_contract(None, SoroStream);
        let client = SoroStreamClient::new(&env, &contract_id);
        client.initialize(&admin);

        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);

        // Mint tokens to sender so they can fund streams
        token_client.mint(&sender, &1_000_000_000_i128);

        Setup {
            env,
            client,
            token: token_client,
            admin,
            sender,
            recipient,
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_initialize_only_once() {
        let Setup { env, client, admin, .. } = setup();
        let extra_admin = Address::generate(&env);
        let result = client.try_initialize(&extra_admin);
        assert!(result.is_err(), "second initialize should fail");
        // Silence unused variable warning
        let _ = admin;
    }

    #[test]
    fn test_create_stream_happy_path() {
        let Setup {
            env,
            client,
            token,
            sender,
            recipient,
            ..
        } = setup();

        // Advance ledger so start_time is "in the future"
        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 2_000_u64;
        let end = 12_000_u64; // 10 000 second stream

        let stream_id = client.create_stream(
            &sender,
            &recipient,
            &token.address,
            &100_000_000_i128,
            &start,
            &end,
        );

        assert_eq!(stream_id, 0);

        let stream = client.get_stream(&stream_id);
        assert_eq!(stream.total_amount, 100_000_000);
        assert_eq!(stream.claimed_amount, 0);
        assert!(!stream.is_cancelled);
    }

    #[test]
    fn test_claim_stream_half_elapsed() {
        let Setup {
            env,
            client,
            token,
            sender,
            recipient,
            ..
        } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 1_000_u64;
        let end = 11_000_u64; // 10 000 second stream

        let token_client = token::Client::new(&env, &token.address);

        let stream_id = client.create_stream(
            &sender,
            &recipient,
            &token.address,
            &100_000_000_i128,
            &start,
            &end,
        );

        // ── Wind forward to halfway ───────────────────────────────────────
        env.ledger().with_mut(|l| l.timestamp = 6_000); // 5 000s elapsed = 50 %

        let claimed = client.claim_stream(&stream_id, &recipient);

        assert_eq!(claimed, 50_000_000, "50 % of 100M should be claimable");
        assert_eq!(token_client.balance(&recipient), 50_000_000);
    }

    #[test]
    fn test_claim_stream_fully_elapsed() {
        let Setup {
            env,
            client,
            token,
            sender,
            recipient,
            ..
        } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 1_000_u64;
        let end = 11_000_u64;

        let token_client = token::Client::new(&env, &token.address);

        let stream_id = client.create_stream(
            &sender,
            &recipient,
            &token.address,
            &100_000_000_i128,
            &start,
            &end,
        );

        // Wind past end_time
        env.ledger().with_mut(|l| l.timestamp = 20_000);

        let claimed = client.claim_stream(&stream_id, &recipient);
        assert_eq!(claimed, 100_000_000, "full amount vested after end_time");
        assert_eq!(token_client.balance(&recipient), 100_000_000);
    }

    #[test]
    #[should_panic(expected = "nothing to claim")]
    fn test_claim_before_start_panics() {
        let Setup {
            env,
            client,
            token,
            sender,
            recipient,
            ..
        } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 5_000_u64;
        let end = 15_000_u64;

        let stream_id = client.create_stream(
            &sender,
            &recipient,
            &token.address,
            &100_000_000_i128,
            &start,
            &end,
        );

        // Claim before stream has started
        env.ledger().with_mut(|l| l.timestamp = 2_000);
        client.claim_stream(&stream_id, &recipient);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_create_stream_zero_amount_panics() {
        let Setup {
            env,
            client,
            token,
            sender,
            recipient,
            ..
        } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        client.create_stream(&sender, &recipient, &token.address, &0_i128, &2_000_u64, &5_000_u64);
    }

    #[test]
    fn test_cancel_at_50pct() {
        let Setup { env, client, token, sender, recipient, .. } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 1_000_u64;
        let end = 11_000_u64;

        let token_client = token::Client::new(&env, &token.address);

        let stream_id = client.create_stream(&sender, &recipient, &token.address, &100_000_000_i128, &start, &end);

        // Wind forward to halfway
        env.ledger().with_mut(|l| l.timestamp = 6_000);

        client.cancel_stream(&stream_id, &sender);

        assert_eq!(token_client.balance(&recipient), 50_000_000, "50% to recipient");
        assert_eq!(token_client.balance(&sender), 950_000_000, "50% refunded to sender (900M + 50M)");

        let stream = client.get_stream(&stream_id);
        assert!(stream.is_cancelled);
        assert_eq!(stream.claimed_amount, 50_000_000);
    }

    #[test]
    fn test_cancel_before_start() {
        let Setup { env, client, token, sender, recipient, .. } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 5_000_u64;
        let end = 15_000_u64;

        let token_client = token::Client::new(&env, &token.address);
        let stream_id = client.create_stream(&sender, &recipient, &token.address, &100_000_000_i128, &start, &end);

        env.ledger().with_mut(|l| l.timestamp = 2_000);
        client.cancel_stream(&stream_id, &sender);

        assert_eq!(token_client.balance(&recipient), 0);
        assert_eq!(token_client.balance(&sender), 1_000_000_000, "100% refunded");
    }

    #[test]
    fn test_cancel_after_end() {
        let Setup { env, client, token, sender, recipient, .. } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 1_000_u64;
        let end = 11_000_u64;

        let token_client = token::Client::new(&env, &token.address);
        let stream_id = client.create_stream(&sender, &recipient, &token.address, &100_000_000_i128, &start, &end);

        env.ledger().with_mut(|l| l.timestamp = 20_000);
        client.cancel_stream(&stream_id, &sender);

        assert_eq!(token_client.balance(&recipient), 100_000_000, "100% claimable");
        assert_eq!(token_client.balance(&sender), 900_000_000, "0 refund");
    }

    #[test]
    #[should_panic(expected = "stream has been cancelled")]
    fn test_claim_after_cancel_panics() {
        let Setup { env, client, token, sender, recipient, .. } = setup();

        env.ledger().with_mut(|l| l.timestamp = 1_000);
        let start = 1_000_u64;
        let end = 11_000_u64;

        let stream_id = client.create_stream(&sender, &recipient, &token.address, &100_000_000_i128, &start, &end);

        env.ledger().with_mut(|l| l.timestamp = 6_000);
        client.cancel_stream(&stream_id, &sender);

        client.claim_stream(&stream_id, &recipient);
    }
}
