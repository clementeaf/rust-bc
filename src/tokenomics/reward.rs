//! Block reward distribution — connects `EconomicsState` to `AccountStore`.
//!
//! After each block, the protocol mints a reward and distributes fees.
//! This module applies those effects to account balances.

use crate::account::{AccountError, AccountStore};
use crate::tokenomics::economics::{process_block, EconomicsState, FeeSplit, BURN_ADDRESS};

/// Result of processing a block's economic effects against account state.
#[derive(Debug, Clone)]
pub struct BlockRewardResult {
    /// Updated economics state for the next block.
    pub economics: EconomicsState,
    /// Block reward minted to proposer.
    pub reward: u64,
    /// Fee split (burn + proposer share).
    pub fee_split: FeeSplit,
}

/// Process a block's economics and apply balance changes to the account store.
///
/// 1. Calls `process_block()` to compute reward, fee split, and new base fee.
/// 2. Credits the proposer with: block_reward + fee proposer share.
/// 3. Sends burned fees to `BURN_ADDRESS`.
///
/// Returns the updated economics state and distribution details.
pub fn apply_block_rewards(
    economics: &EconomicsState,
    store: &dyn AccountStore,
    proposer: &str,
    tx_count: u64,
    total_fees: u64,
) -> Result<BlockRewardResult, AccountError> {
    let (new_state, fee_split, reward) = process_block(economics, tx_count, total_fees);

    // Credit block reward to proposer
    if reward > 0 {
        store.credit(proposer, reward)?;
    }

    // Credit proposer's share of fees
    if fee_split.proposer > 0 {
        store.credit(proposer, fee_split.proposer)?;
    }

    // Burn fees
    if fee_split.burn > 0 {
        store.credit(BURN_ADDRESS, fee_split.burn)?;
    }

    Ok(BlockRewardResult {
        economics: new_state,
        reward,
        fee_split,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::MemoryAccountStore;
    use crate::tokenomics::economics::{
        FEE_BURN_PERCENT, FEE_PROPOSER_PERCENT, INITIAL_BLOCK_REWARD,
    };

    #[test]
    fn genesis_block_reward_credits_proposer() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState::default();

        let result = apply_block_rewards(&economics, &store, "proposer", 0, 0).unwrap();

        assert_eq!(result.reward, INITIAL_BLOCK_REWARD);
        let proposer = store.get_account("proposer").unwrap();
        assert_eq!(proposer.balance, INITIAL_BLOCK_REWARD);
    }

    #[test]
    fn fees_split_between_burn_and_proposer() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState::default();

        let total_fees = 100;
        let result = apply_block_rewards(&economics, &store, "proposer", 5, total_fees).unwrap();

        let expected_burn = (total_fees * FEE_BURN_PERCENT) / 100;
        let expected_proposer = total_fees - expected_burn;

        assert_eq!(result.fee_split.burn, expected_burn);
        assert_eq!(result.fee_split.proposer, expected_proposer);

        let proposer = store.get_account("proposer").unwrap();
        assert_eq!(proposer.balance, INITIAL_BLOCK_REWARD + expected_proposer);

        let burned = store.get_account(BURN_ADDRESS).unwrap();
        assert_eq!(burned.balance, expected_burn);
    }

    #[test]
    fn economics_state_advances() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState::default();

        let r = apply_block_rewards(&economics, &store, "p", 10, 50).unwrap();
        assert_eq!(r.economics.height, 1);
        assert_eq!(r.economics.total_minted, INITIAL_BLOCK_REWARD);

        let r2 = apply_block_rewards(&r.economics, &store, "p", 5, 20).unwrap();
        assert_eq!(r2.economics.height, 2);
        assert_eq!(r2.economics.total_minted, INITIAL_BLOCK_REWARD * 2);
    }

    #[test]
    fn zero_reward_when_supply_capped() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState {
            height: 0,
            total_minted: crate::tokenomics::economics::MAX_SUPPLY,
            total_burned: 0,
            base_fee: 1,
            epoch: 0,
            epoch_fees: 0,
        };

        let r = apply_block_rewards(&economics, &store, "p", 0, 0).unwrap();
        assert_eq!(r.reward, 0);
        let proposer = store.get_account("p").unwrap();
        assert_eq!(proposer.balance, 0);
    }

    #[test]
    fn no_fees_no_burn() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState::default();

        let r = apply_block_rewards(&economics, &store, "p", 0, 0).unwrap();
        assert_eq!(r.fee_split.burn, 0);
        assert_eq!(r.fee_split.proposer, 0);

        let burned = store.get_account(BURN_ADDRESS).unwrap();
        assert_eq!(burned.balance, 0);
    }

    #[test]
    fn multiple_blocks_accumulate_rewards() {
        let store = MemoryAccountStore::new();
        let mut economics = EconomicsState::default();

        for _ in 0..10 {
            let r = apply_block_rewards(&economics, &store, "miner", 0, 0).unwrap();
            economics = r.economics;
        }

        let miner = store.get_account("miner").unwrap();
        assert_eq!(miner.balance, INITIAL_BLOCK_REWARD * 10);
        assert_eq!(economics.height, 10);
        assert_eq!(economics.total_minted, INITIAL_BLOCK_REWARD * 10);
    }

    #[test]
    fn fee_only_block_after_supply_cap() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState {
            height: 999_999,
            total_minted: crate::tokenomics::economics::MAX_SUPPLY,
            total_burned: 0,
            base_fee: 1,
            epoch: 0,
            epoch_fees: 0,
        };

        let r = apply_block_rewards(&economics, &store, "p", 10, 200).unwrap();
        assert_eq!(r.reward, 0);

        let expected_proposer_fee = 200 * FEE_PROPOSER_PERCENT / 100;
        let proposer = store.get_account("p").unwrap();
        // Only fee share, no reward
        assert_eq!(proposer.balance, expected_proposer_fee);
    }
}
