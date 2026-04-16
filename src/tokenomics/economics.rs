//! Token economics — supply cap, issuance curve, fee model, and epoch rewards.
//!
//! The NOTA token has a fixed maximum supply with a decaying issuance curve.
//! Transaction fees are partially burned (deflationary) and partially
//! distributed to validators (security incentive).

use serde::{Deserialize, Serialize};

// ── Constants ───────────────────────────────────────────────────────────────

/// Maximum total supply of NOTA tokens (100 million).
pub const MAX_SUPPLY: u64 = 100_000_000;

/// Initial block reward before any decay.
pub const INITIAL_BLOCK_REWARD: u64 = 50;

/// Number of blocks between each reward halving.
pub const HALVING_INTERVAL: u64 = 210_000;

/// Percentage of transaction fees burned (deflationary pressure).
pub const FEE_BURN_PERCENT: u64 = 80;

/// Percentage of transaction fees distributed to the block proposer.
pub const FEE_PROPOSER_PERCENT: u64 = 20;

/// Sentinel address for burned tokens. Tokens sent here are permanently
/// removed from circulating supply.
pub const BURN_ADDRESS: &str = "BURN_ADDRESS_00000000000000000000";

/// Minimum transaction fee (in smallest NOTA unit) to prevent spam.
pub const MIN_TX_FEE: u64 = 1;

/// Target block utilization (fraction of max txs per block).
/// Used for dynamic base fee adjustment.
pub const TARGET_UTILIZATION: f64 = 0.5;

/// Maximum transactions per block (for utilization calculation).
pub const MAX_TXS_PER_BLOCK: u64 = 500;

/// Base fee adjustment factor per block (EIP-1559 inspired).
/// Fee increases by up to 12.5% per block when above target, decreases when below.
pub const BASE_FEE_ADJUSTMENT_FACTOR: u64 = 8; // 1/8 = 12.5%

/// Minimum base fee floor (prevents zero-fee blocks).
pub const MIN_BASE_FEE: u64 = 1;

/// Blocks per epoch for reward distribution. (~2 days at 15s/block)
pub const BLOCKS_PER_EPOCH: u64 = 11_520;

/// Approximate blocks per year at 15-second block time.
pub const BLOCKS_PER_YEAR: u64 = 2_102_400;

// ── Token Economics ─────────────────────────────────────────────────────────

/// Snapshot of the protocol's economic state at a given block height.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomicsState {
    /// Current block height.
    pub height: u64,
    /// Total tokens minted so far (cumulative block rewards).
    pub total_minted: u64,
    /// Total tokens burned so far (cumulative fee burns).
    pub total_burned: u64,
    /// Current dynamic base fee per transaction.
    pub base_fee: u64,
    /// Current epoch number.
    pub epoch: u64,
    /// Cumulative fees collected in the current epoch.
    pub epoch_fees: u64,
}

impl Default for EconomicsState {
    fn default() -> Self {
        Self {
            height: 0,
            total_minted: 0,
            total_burned: 0,
            base_fee: MIN_BASE_FEE,
            epoch: 0,
            epoch_fees: 0,
        }
    }
}

impl EconomicsState {
    /// Circulating supply = minted - burned.
    pub fn circulating_supply(&self) -> u64 {
        self.total_minted.saturating_sub(self.total_burned)
    }

    /// Whether the maximum supply has been reached (no more block rewards).
    pub fn supply_cap_reached(&self) -> bool {
        self.total_minted >= MAX_SUPPLY
    }

    /// Current epoch (0-indexed).
    pub fn current_epoch(&self) -> u64 {
        self.height / BLOCKS_PER_EPOCH
    }

    /// Annual inflation rate as a percentage, based on current block reward.
    pub fn annual_inflation_percent(&self) -> f64 {
        let reward = block_reward(self.height);
        if self.circulating_supply() == 0 {
            return 0.0;
        }
        let annual_rewards = reward as f64 * BLOCKS_PER_YEAR as f64;
        (annual_rewards / self.circulating_supply() as f64) * 100.0
    }
}

// ── Issuance Curve ──────────────────────────────────────────────────────────

/// Calculate the block reward for a given height.
///
/// Uses Bitcoin-style halving: reward starts at [`INITIAL_BLOCK_REWARD`]
/// and halves every [`HALVING_INTERVAL`] blocks. Returns 0 if the reward
/// would underflow or if `MAX_SUPPLY` has been reached.
pub fn block_reward(height: u64) -> u64 {
    let halvings = height / HALVING_INTERVAL;
    if halvings >= 64 {
        return 0;
    }
    INITIAL_BLOCK_REWARD >> halvings
}

/// Calculate the block reward capped by remaining supply.
///
/// If `total_minted + reward > MAX_SUPPLY`, the reward is reduced to
/// exactly fill the remaining supply. Returns 0 if supply cap is reached.
pub fn capped_block_reward(height: u64, total_minted: u64) -> u64 {
    if total_minted >= MAX_SUPPLY {
        return 0;
    }
    let reward = block_reward(height);
    let remaining = MAX_SUPPLY - total_minted;
    reward.min(remaining)
}

// ── Fee Distribution ────────────────────────────────────────────────────────

/// Split transaction fees into burn and proposer portions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeSplit {
    /// Amount sent to `BURN_ADDRESS` (permanently removed from supply).
    pub burn: u64,
    /// Amount distributed to the block proposer.
    pub proposer: u64,
}

/// Calculate the fee split for a given total fee amount.
pub fn split_fees(total_fees: u64) -> FeeSplit {
    let burn = (total_fees * FEE_BURN_PERCENT) / 100;
    let proposer = total_fees - burn; // Remainder to avoid rounding loss
    FeeSplit { burn, proposer }
}

// ── Dynamic Base Fee ────────────────────────────────────────────────────────

/// Calculate the new base fee for the next block based on utilization.
///
/// Inspired by EIP-1559:
/// - If block is more than 50% full: base fee increases (up to 12.5%)
/// - If block is less than 50% full: base fee decreases (up to 12.5%)
/// - Never drops below `MIN_BASE_FEE`
pub fn next_base_fee(current_base_fee: u64, tx_count: u64) -> u64 {
    let target = (MAX_TXS_PER_BLOCK as f64 * TARGET_UTILIZATION) as u64;

    if tx_count == target {
        return current_base_fee;
    }

    let delta = current_base_fee / BASE_FEE_ADJUSTMENT_FACTOR;

    if tx_count > target {
        // Above target: increase fee
        let excess = tx_count - target;
        let increase = (delta * excess) / target.max(1);
        current_base_fee.saturating_add(increase.max(1))
    } else {
        // Below target: decrease fee
        let deficit = target - tx_count;
        let decrease = (delta * deficit) / target.max(1);
        current_base_fee.saturating_sub(decrease).max(MIN_BASE_FEE)
    }
}

/// Validate that a transaction's fee meets the current base fee.
pub fn validate_tx_fee(tx_fee: u64, base_fee: u64) -> Result<(), FeeError> {
    if tx_fee < base_fee {
        return Err(FeeError::BelowBaseFee {
            offered: tx_fee,
            required: base_fee,
        });
    }
    if tx_fee < MIN_TX_FEE {
        return Err(FeeError::BelowMinimum {
            offered: tx_fee,
            minimum: MIN_TX_FEE,
        });
    }
    Ok(())
}

/// Fee validation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeeError {
    #[error("fee {offered} below base fee {required}")]
    BelowBaseFee { offered: u64, required: u64 },
    #[error("fee {offered} below minimum {minimum}")]
    BelowMinimum { offered: u64, minimum: u64 },
}

// ── State Transition ────────────────────────────────────────────────────────

/// Process a block's economic effects and return the updated state.
///
/// This is the single entry point for all tokenomics state transitions:
/// 1. Calculate (and cap) the block reward
/// 2. Split fees into burn + proposer
/// 3. Adjust the dynamic base fee
/// 4. Advance the epoch if needed
pub fn process_block(
    state: &EconomicsState,
    tx_count: u64,
    total_fees: u64,
) -> (EconomicsState, FeeSplit, u64) {
    let reward = capped_block_reward(state.height, state.total_minted);
    let fee_split = split_fees(total_fees);
    let new_base_fee = next_base_fee(state.base_fee, tx_count);

    let new_height = state.height + 1;
    let new_epoch = new_height / BLOCKS_PER_EPOCH;
    let epoch_fees = if new_epoch != state.epoch {
        0 // Reset on epoch boundary
    } else {
        state.epoch_fees + total_fees
    };

    let new_state = EconomicsState {
        height: new_height,
        total_minted: state.total_minted + reward,
        total_burned: state.total_burned + fee_split.burn,
        base_fee: new_base_fee,
        epoch: new_epoch,
        epoch_fees,
    };

    (new_state, fee_split, reward)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- block_reward ---

    #[test]
    fn reward_at_genesis() {
        assert_eq!(block_reward(0), 50);
    }

    #[test]
    fn reward_before_first_halving() {
        assert_eq!(block_reward(209_999), 50);
    }

    #[test]
    fn reward_at_first_halving() {
        assert_eq!(block_reward(210_000), 25);
    }

    #[test]
    fn reward_at_second_halving() {
        assert_eq!(block_reward(420_000), 12);
    }

    #[test]
    fn reward_at_third_halving() {
        assert_eq!(block_reward(630_000), 6);
    }

    #[test]
    fn reward_eventually_reaches_zero() {
        // After 64 halvings, reward is 0.
        assert_eq!(block_reward(64 * HALVING_INTERVAL), 0);
    }

    // --- capped_block_reward ---

    #[test]
    fn capped_reward_normal() {
        assert_eq!(capped_block_reward(0, 0), 50);
    }

    #[test]
    fn capped_reward_near_cap() {
        // Only 10 tokens remaining.
        assert_eq!(capped_block_reward(0, MAX_SUPPLY - 10), 10);
    }

    #[test]
    fn capped_reward_at_cap() {
        assert_eq!(capped_block_reward(0, MAX_SUPPLY), 0);
    }

    #[test]
    fn capped_reward_over_cap() {
        assert_eq!(capped_block_reward(0, MAX_SUPPLY + 100), 0);
    }

    // --- split_fees ---

    #[test]
    fn fee_split_100() {
        let s = split_fees(100);
        assert_eq!(s.burn, 80);
        assert_eq!(s.proposer, 20);
    }

    #[test]
    fn fee_split_zero() {
        let s = split_fees(0);
        assert_eq!(s.burn, 0);
        assert_eq!(s.proposer, 0);
    }

    #[test]
    fn fee_split_odd_amount() {
        // 101 * 80 / 100 = 80, proposer gets 21 (remainder).
        let s = split_fees(101);
        assert_eq!(s.burn, 80);
        assert_eq!(s.proposer, 21);
        assert_eq!(s.burn + s.proposer, 101); // No loss
    }

    // --- next_base_fee ---

    #[test]
    fn base_fee_stable_at_target() {
        let target = (MAX_TXS_PER_BLOCK as f64 * TARGET_UTILIZATION) as u64;
        assert_eq!(next_base_fee(100, target), 100);
    }

    #[test]
    fn base_fee_increases_above_target() {
        let above = MAX_TXS_PER_BLOCK; // 100% full
        let new_fee = next_base_fee(100, above);
        assert!(new_fee > 100, "fee should increase, got {new_fee}");
    }

    #[test]
    fn base_fee_decreases_below_target() {
        let below = 0; // Empty block
        let new_fee = next_base_fee(100, below);
        assert!(new_fee < 100, "fee should decrease, got {new_fee}");
    }

    #[test]
    fn base_fee_never_below_minimum() {
        let new_fee = next_base_fee(MIN_BASE_FEE, 0);
        assert_eq!(new_fee, MIN_BASE_FEE);
    }

    #[test]
    fn base_fee_gradual_increase() {
        // Slightly above target — small increase.
        let target = (MAX_TXS_PER_BLOCK as f64 * TARGET_UTILIZATION) as u64;
        let slightly_above = target + 10;
        let new_fee = next_base_fee(1000, slightly_above);
        assert!(new_fee > 1000 && new_fee < 1200, "got {new_fee}");
    }

    // --- validate_tx_fee ---

    #[test]
    fn fee_at_base_is_valid() {
        assert!(validate_tx_fee(10, 10).is_ok());
    }

    #[test]
    fn fee_above_base_is_valid() {
        assert!(validate_tx_fee(20, 10).is_ok());
    }

    #[test]
    fn fee_below_base_is_rejected() {
        let err = validate_tx_fee(5, 10).unwrap_err();
        assert!(matches!(err, FeeError::BelowBaseFee { .. }));
    }

    #[test]
    fn fee_zero_is_rejected() {
        let err = validate_tx_fee(0, 1).unwrap_err();
        assert!(matches!(err, FeeError::BelowBaseFee { .. }));
    }

    // --- process_block ---

    #[test]
    fn process_block_from_genesis() {
        let state = EconomicsState::default();
        let (new_state, fee_split, reward) = process_block(&state, 10, 100);

        assert_eq!(reward, 50);
        assert_eq!(new_state.height, 1);
        assert_eq!(new_state.total_minted, 50);
        assert_eq!(new_state.total_burned, 80);
        assert_eq!(fee_split.burn, 80);
        assert_eq!(fee_split.proposer, 20);
    }

    #[test]
    fn process_block_caps_reward_at_supply_limit() {
        let state = EconomicsState {
            total_minted: MAX_SUPPLY - 10,
            ..Default::default()
        };
        let (new_state, _, reward) = process_block(&state, 5, 50);
        assert_eq!(reward, 10); // Capped to remaining
        assert_eq!(new_state.total_minted, MAX_SUPPLY);
    }

    #[test]
    fn process_block_no_reward_after_cap() {
        let state = EconomicsState {
            total_minted: MAX_SUPPLY,
            ..Default::default()
        };
        let (new_state, _, reward) = process_block(&state, 5, 50);
        assert_eq!(reward, 0);
        assert_eq!(new_state.total_minted, MAX_SUPPLY);
    }

    #[test]
    fn process_block_epoch_advances() {
        let state = EconomicsState {
            height: BLOCKS_PER_EPOCH - 1,
            epoch: 0,
            epoch_fees: 5000,
            ..Default::default()
        };
        let (new_state, _, _) = process_block(&state, 5, 100);
        assert_eq!(new_state.epoch, 1);
        assert_eq!(new_state.epoch_fees, 0); // Reset
    }

    // --- EconomicsState ---

    #[test]
    fn circulating_supply() {
        let state = EconomicsState {
            total_minted: 1000,
            total_burned: 300,
            ..Default::default()
        };
        assert_eq!(state.circulating_supply(), 700);
    }

    #[test]
    fn supply_cap_not_reached() {
        let state = EconomicsState::default();
        assert!(!state.supply_cap_reached());
    }

    #[test]
    fn supply_cap_reached() {
        let state = EconomicsState {
            total_minted: MAX_SUPPLY,
            ..Default::default()
        };
        assert!(state.supply_cap_reached());
    }

    #[test]
    fn annual_inflation_at_genesis() {
        let state = EconomicsState {
            total_minted: 1_000_000,
            ..Default::default()
        };
        let inflation = state.annual_inflation_percent();
        // 50 * 2_102_400 / 1_000_000 * 100 = 10512%
        assert!(inflation > 0.0);
    }

    // --- multi-block simulation ---

    #[test]
    fn simulate_1000_blocks() {
        let mut state = EconomicsState::default();
        for _ in 0..1000 {
            let (new_state, _, _) = process_block(&state, 50, 10);
            state = new_state;
        }
        assert_eq!(state.height, 1000);
        assert_eq!(state.total_minted, 50 * 1000); // No halving yet
        assert_eq!(state.total_burned, 8 * 1000); // 80% of 10 per block
        assert_eq!(state.circulating_supply(), 50_000 - 8_000);
    }
}
