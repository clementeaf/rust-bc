//! Formal monetary policy — emission schedule, supply invariants, and
//! validator economics.
//!
//! This module provides projection functions and policy enforcement that
//! complement the per-block `economics.rs` calculations.

use super::economics::{
    block_reward, capped_block_reward, HALVING_INTERVAL, MAX_SUPPLY, MAX_TXS_PER_BLOCK,
    MIN_BASE_FEE,
};

// ── Emission Projection ────────────────────────────────────────────────────

/// Project total emission up to a given height (not counting fees/burns).
pub fn projected_emission(up_to_height: u64) -> u64 {
    let mut total: u64 = 0;
    let mut height: u64 = 0;
    while height < up_to_height {
        let reward = block_reward(height);
        if reward == 0 {
            break;
        }
        let era_end = ((height / HALVING_INTERVAL) + 1) * HALVING_INTERVAL;
        let blocks_in_era = era_end.min(up_to_height) - height;
        let era_emission = reward.saturating_mul(blocks_in_era);
        total = total.saturating_add(era_emission);
        if total >= MAX_SUPPLY {
            return MAX_SUPPLY;
        }
        height = era_end;
    }
    total.min(MAX_SUPPLY)
}

/// Height at which the last NOTA token is minted (supply fully emitted).
pub fn final_emission_height() -> u64 {
    let mut total: u64 = 0;
    let mut height: u64 = 0;
    loop {
        let reward = capped_block_reward(height, total);
        if reward == 0 {
            return height;
        }
        total += reward;
        height += 1;
        if total >= MAX_SUPPLY {
            return height;
        }
        // Optimization: skip ahead within same era
        let era_reward = block_reward(height);
        if era_reward == 0 {
            return height;
        }
        let remaining = MAX_SUPPLY - total;
        let era_end = ((height / HALVING_INTERVAL) + 1) * HALVING_INTERVAL;
        let blocks_left_in_era = era_end - height;
        let full_era_emission = era_reward * blocks_left_in_era;
        if full_era_emission <= remaining {
            total += full_era_emission;
            height = era_end;
        }
        // else continue block-by-block near cap
    }
}

// ── Validator Economics ─────────────────────────────────────────────────────

/// Minimum stake required to be eligible as block proposer.
pub const MIN_PROPOSER_STAKE: u64 = 1_000;

/// Minimum stake required to vote on governance proposals.
pub const MIN_GOVERNANCE_STAKE: u64 = 100;

/// Check if an address has sufficient stake to propose blocks.
pub fn can_propose(stake: u64) -> bool {
    stake >= MIN_PROPOSER_STAKE
}

/// Annual yield estimate for a validator at a given height and total staked.
///
/// Assumes all block rewards go to the validator set (distributed proportionally).
pub fn estimated_annual_yield_pct(height: u64, total_staked: u64) -> f64 {
    if total_staked == 0 {
        return 0.0;
    }
    let reward = block_reward(height);
    let annual_rewards = reward as f64 * super::economics::BLOCKS_PER_YEAR as f64;
    (annual_rewards / total_staked as f64) * 100.0
}

// ── Spam Protection Policy ─────────────────────────────────────────────────

/// Validate a transaction fee against the current base fee.
///
/// Returns Ok(effective_priority_fee) or Err with reason.
pub fn validate_fee(offered_fee: u64, base_fee: u64) -> Result<u64, String> {
    if offered_fee < MIN_BASE_FEE {
        return Err(format!(
            "fee {offered_fee} below absolute minimum {MIN_BASE_FEE}"
        ));
    }
    if offered_fee < base_fee {
        return Err(format!(
            "fee {offered_fee} below current base fee {base_fee}"
        ));
    }
    Ok(offered_fee - base_fee)
}

/// Maximum gas/compute units per block (for future gas metering).
pub const MAX_BLOCK_GAS: u64 = MAX_TXS_PER_BLOCK * 21_000; // ~10.5M, similar to Ethereum

// ── Supply Invariant ───────────────────────────────────────────────────────

/// Verify the fundamental supply invariant:
/// `circulating + burned + unminted == MAX_SUPPLY`
pub fn verify_supply_invariant(minted: u64, burned: u64) -> Result<(), String> {
    if minted > MAX_SUPPLY {
        return Err(format!("minted {minted} exceeds MAX_SUPPLY {MAX_SUPPLY}"));
    }
    if burned > minted {
        return Err(format!(
            "burned {burned} exceeds minted {minted} — impossible"
        ));
    }
    // circulating = minted - burned
    // unminted = MAX_SUPPLY - minted
    // circulating + burned + unminted == minted - burned + burned + MAX_SUPPLY - minted == MAX_SUPPLY ✓
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenomics::economics::INITIAL_BLOCK_REWARD;

    #[test]
    fn emission_at_genesis_is_zero() {
        assert_eq!(projected_emission(0), 0);
    }

    #[test]
    fn emission_first_era() {
        // First 210K blocks at 50 NOTA each = 10.5M
        let emission = projected_emission(HALVING_INTERVAL);
        assert_eq!(emission, INITIAL_BLOCK_REWARD * HALVING_INTERVAL);
    }

    #[test]
    fn emission_two_eras() {
        // Era 0: 50 * 210K = 10.5M
        // Era 1: 25 * 210K = 5.25M
        // Total: 15.75M
        let emission = projected_emission(HALVING_INTERVAL * 2);
        assert_eq!(emission, 10_500_000 + 5_250_000);
    }

    #[test]
    fn emission_never_exceeds_max_supply() {
        // Project far into the future
        let emission = projected_emission(u64::MAX);
        assert!(emission <= MAX_SUPPLY);
    }

    #[test]
    fn final_emission_height_is_finite() {
        let h = final_emission_height();
        assert!(h > 0);
        assert!(h < u64::MAX);
        // After this height, no more rewards
        assert_eq!(capped_block_reward(h, MAX_SUPPLY), 0);
    }

    #[test]
    fn total_emission_converges() {
        // Integer halving: 50→25→12→6→3→1→0
        // Total = (50+25+12+6+3+1) * 210_000 = 20_370_000
        let total = projected_emission(HALVING_INTERVAL * 64);
        assert_eq!(total, 20_370_000);
        assert!(total <= MAX_SUPPLY);
        // After all halvings, no more emission
        assert_eq!(block_reward(HALVING_INTERVAL * 7), 0);
    }

    #[test]
    fn halving_schedule() {
        assert_eq!(block_reward(0), 50);
        assert_eq!(block_reward(HALVING_INTERVAL), 25);
        assert_eq!(block_reward(HALVING_INTERVAL * 2), 12);
        assert_eq!(block_reward(HALVING_INTERVAL * 3), 6);
        assert_eq!(block_reward(HALVING_INTERVAL * 4), 3);
        assert_eq!(block_reward(HALVING_INTERVAL * 5), 1);
        // Eventually reaches 0
        assert_eq!(block_reward(HALVING_INTERVAL * 64), 0);
    }

    #[test]
    fn validator_minimum_stake() {
        assert!(!can_propose(0));
        assert!(!can_propose(999));
        assert!(can_propose(1_000));
        assert!(can_propose(100_000));
    }

    #[test]
    fn annual_yield_decreases_with_more_stake() {
        let y1 = estimated_annual_yield_pct(0, 1_000_000);
        let y2 = estimated_annual_yield_pct(0, 10_000_000);
        assert!(y1 > y2);
        assert!(y1 > 0.0);
    }

    #[test]
    fn annual_yield_zero_stake() {
        assert_eq!(estimated_annual_yield_pct(0, 0), 0.0);
    }

    #[test]
    fn fee_validation_below_minimum() {
        assert!(validate_fee(0, 5).is_err());
    }

    #[test]
    fn fee_validation_below_base_fee() {
        assert!(validate_fee(3, 5).is_err());
    }

    #[test]
    fn fee_validation_ok() {
        let priority = validate_fee(10, 5).unwrap();
        assert_eq!(priority, 5); // priority fee = offered - base
    }

    #[test]
    fn supply_invariant_valid() {
        assert!(verify_supply_invariant(50_000_000, 10_000_000).is_ok());
    }

    #[test]
    fn supply_invariant_over_max() {
        assert!(verify_supply_invariant(MAX_SUPPLY + 1, 0).is_err());
    }

    #[test]
    fn supply_invariant_burned_exceeds_minted() {
        assert!(verify_supply_invariant(100, 200).is_err());
    }
}
