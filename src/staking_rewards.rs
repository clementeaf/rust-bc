/**
 * Staking Rewards Module
 *
 * APY-based reward system for staking contracts:
 * - Reward calculation based on APY
 * - Distribution to stakers
 * - Claim mechanism
 * - Integration with staking_manager
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Staking reward record for a staker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakerRewards {
    pub address: String,
    pub total_staked: u64,
    pub accumulated_rewards: u64,
    pub claimed_rewards: u64,
    pub last_reward_block: u64,
    pub stake_start_block: u64,
}

impl StakerRewards {
    pub fn new(address: String, total_staked: u64, start_block: u64) -> Self {
        StakerRewards {
            address,
            total_staked,
            accumulated_rewards: 0,
            claimed_rewards: 0,
            last_reward_block: start_block,
            stake_start_block: start_block,
        }
    }

    /// Get pending rewards (not yet claimed)
    pub fn pending_rewards(&self) -> u64 {
        self.accumulated_rewards.saturating_sub(self.claimed_rewards)
    }
}

/// Staking reward contract configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    pub apy_percentage: f64,              // Annual Percentage Yield (e.g., 10.0 for 10% APY)
    pub blocks_per_year: u64,             // Average blocks per year (for APY calculation)
    pub min_staking_amount: u64,          // Minimum amount to earn rewards
    pub reward_interval_blocks: u64,      // Blocks between reward distributions
    pub max_annual_rewards: u64,          // Max rewards available per year (treasury limit)
}

impl Default for RewardConfig {
    fn default() -> Self {
        RewardConfig {
            apy_percentage: 10.0,                      // 10% APY
            blocks_per_year: 2_102_400,                // ~365 days at 15s/block
            min_staking_amount: 100,
            reward_interval_blocks: 13_440,             // ~2 days in blocks
            max_annual_rewards: 10_000_000,             // 10M max annual rewards
        }
    }
}

/// Staking reward contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingRewardContract {
    pub contract_address: String,
    pub owner: String,
    pub token_address: String,           // Governance token address
    pub config: RewardConfig,
    pub staker_rewards: HashMap<String, StakerRewards>,
    pub total_rewards_distributed: u64,
    pub treasury_balance: u64,           // Available rewards
    pub last_distribution_block: u64,
}

impl StakingRewardContract {
    pub fn new(
        contract_address: String,
        owner: String,
        token_address: String,
        config: RewardConfig,
        initial_treasury: u64,
    ) -> Self {
        StakingRewardContract {
            contract_address,
            owner,
            token_address,
            config,
            staker_rewards: HashMap::new(),
            total_rewards_distributed: 0,
            treasury_balance: initial_treasury,
            last_distribution_block: 0,
        }
    }

    /// Calculate reward for a staker based on blocks staked and APY
    fn calculate_reward_internal(&self, staked_amount: u64, blocks_elapsed: u64, config: &RewardConfig) -> u64 {
        if staked_amount < config.min_staking_amount || blocks_elapsed == 0 {
            return 0;
        }

        // Formula: reward = staked_amount * (APY/100) * (blocks_elapsed / blocks_per_year)
        let apy_factor = config.apy_percentage / 100.0;
        let time_factor = blocks_elapsed as f64 / config.blocks_per_year as f64;
        let reward = staked_amount as f64 * apy_factor * time_factor;

        reward.floor() as u64
    }

    /// Public calculate reward using current config
    pub fn calculate_reward(&self, staked_amount: u64, blocks_elapsed: u64) -> u64 {
        self.calculate_reward_internal(staked_amount, blocks_elapsed, &self.config)
    }

    /// Register a staker in the reward system
    pub fn register_staker(
        &mut self,
        address: String,
        staked_amount: u64,
        current_block: u64,
    ) -> Result<(), String> {
        if staked_amount < self.config.min_staking_amount {
            return Err(format!(
                "Staking amount {} below minimum {}",
                staked_amount, self.config.min_staking_amount
            ));
        }

        if self.staker_rewards.contains_key(&address) {
            return Err("Staker already registered".to_string());
        }

        let staker = StakerRewards::new(address, staked_amount, current_block);
        self.staker_rewards.insert(staker.address.clone(), staker);

        Ok(())
    }

    /// Update staker's staked amount (when additional staking occurs)
    pub fn update_stake(
        &mut self,
        address: &str,
        new_total_amount: u64,
        current_block: u64,
    ) -> Result<(), String> {
        let config = self.config.clone();
        if new_total_amount < config.min_staking_amount {
            return Err(format!(
                "New staking amount {} below minimum {}",
                new_total_amount, config.min_staking_amount
            ));
        }

        // Get staker info without holding mutable borrow
        let (old_staked, blocks_since) = {
            let staker = self
                .staker_rewards
                .get(address)
                .ok_or("Staker not found")?;
            (staker.total_staked, current_block.saturating_sub(staker.last_reward_block))
        };

        // Calculate reward outside of borrow scope
        let pending_reward = self.calculate_reward_internal(old_staked, blocks_since, &config);

        // Now get mutable borrow
        let staker = self
            .staker_rewards
            .get_mut(address)
            .ok_or("Staker not found")?;

        staker.accumulated_rewards += pending_reward;
        staker.total_staked = new_total_amount;
        staker.last_reward_block = current_block;

        Ok(())
    }

    /// Distribute rewards to all stakers
    pub fn distribute_rewards(
        &mut self,
        current_block: u64,
    ) -> Result<u64, String> {
        if current_block < self.last_distribution_block + self.config.reward_interval_blocks {
            return Err("Not enough blocks elapsed for next distribution".to_string());
        }

        let mut total_distributed = 0u64;
        let config = self.config.clone();
        let mut updates: Vec<(String, u64, u64)> = Vec::new();

        for (addr, staker) in self.staker_rewards.iter() {
            if staker.total_staked < config.min_staking_amount {
                continue;
            }

            let blocks_since_reward = current_block.saturating_sub(staker.last_reward_block);
            let reward = self.calculate_reward_internal(staker.total_staked, blocks_since_reward, &config);

            if reward > 0 && self.treasury_balance >= reward {
                updates.push((addr.clone(), reward, current_block));
                total_distributed += reward;
                self.treasury_balance -= reward;
            }
        }

        for (addr, reward, block) in updates {
            if let Some(staker) = self.staker_rewards.get_mut(&addr) {
                staker.accumulated_rewards += reward;
                staker.last_reward_block = block;
            }
        }

        self.total_rewards_distributed += total_distributed;
        self.last_distribution_block = current_block;

        Ok(total_distributed)
    }

    /// Claim accumulated rewards
    pub fn claim_rewards(&mut self, address: &str) -> Result<u64, String> {
        let staker = self
            .staker_rewards
            .get_mut(address)
            .ok_or("Staker not found")?;

        let pending = staker.pending_rewards();
        if pending == 0 {
            return Err("No pending rewards to claim".to_string());
        }

        if self.treasury_balance < pending {
            return Err("Insufficient treasury balance".to_string());
        }

        staker.claimed_rewards += pending;
        self.treasury_balance -= pending;

        Ok(pending)
    }

    /// Get staker information
    pub fn get_staker(&self, address: &str) -> Option<StakerRewards> {
        self.staker_rewards.get(address).cloned()
    }

    /// Get total staked across all stakers
    pub fn total_staked(&self) -> u64 {
        self.staker_rewards
            .values()
            .map(|s| s.total_staked)
            .sum()
    }

    /// Get number of active stakers
    pub fn active_stakers_count(&self) -> u64 {
        self.staker_rewards
            .values()
            .filter(|s| s.total_staked >= self.config.min_staking_amount)
            .count() as u64
    }

    /// Update configuration (owner only)
    pub fn update_config(&mut self, caller: &str, new_config: RewardConfig) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can update config".to_string());
        }

        if new_config.apy_percentage < 0.0 {
            return Err("APY percentage cannot be negative".to_string());
        }

        if new_config.apy_percentage > 100.0 {
            return Err("APY percentage cannot exceed 100%".to_string());
        }

        self.config = new_config;
        Ok(())
    }

    /// Deposit tokens to treasury (owner only)
    pub fn deposit_treasury(&mut self, caller: &str, amount: u64) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can deposit to treasury".to_string());
        }

        self.treasury_balance += amount;
        Ok(())
    }

    /// Withdraw from treasury (owner only)
    pub fn withdraw_treasury(&mut self, caller: &str, amount: u64) -> Result<u64, String> {
        if caller != self.owner {
            return Err("Only owner can withdraw from treasury".to_string());
        }

        let withdrawal = amount.min(self.treasury_balance);
        self.treasury_balance -= withdrawal;

        Ok(withdrawal)
    }

    /// Remove staker from reward system (stakes go to 0)
    pub fn unstake_staker(
        &mut self,
        address: &str,
        current_block: u64,
    ) -> Result<u64, String> {
        let config = self.config.clone();
        
        // Get staker info without holding mutable borrow
        let (old_staked, blocks_since) = {
            let staker = self
                .staker_rewards
                .get(address)
                .ok_or("Staker not found")?;
            (staker.total_staked, current_block.saturating_sub(staker.last_reward_block))
        };

        // Calculate reward outside of borrow scope
        let pending_reward = self.calculate_reward_internal(old_staked, blocks_since, &config);

        // Now get mutable borrow
        let staker = self
            .staker_rewards
            .get_mut(address)
            .ok_or("Staker not found")?;

        staker.accumulated_rewards += pending_reward;
        let prev_staked = staker.total_staked;
        staker.total_staked = 0;

        Ok(prev_staked)
    }

    /// Get estimated annual rewards for a staker
    pub fn estimate_annual_rewards(&self, staked_amount: u64) -> u64 {
        self.calculate_reward(staked_amount, self.config.blocks_per_year)
    }

    /// Get reward statistics
    pub fn get_statistics(&self) -> RewardStatistics {
        let total_stakers = self.staker_rewards.len() as u64;
        let active_stakers = self.active_stakers_count();
        let total_staked = self.total_staked();
        let average_apy = self.config.apy_percentage;

        RewardStatistics {
            total_stakers,
            active_stakers,
            total_staked,
            treasury_balance: self.treasury_balance,
            total_rewards_distributed: self.total_rewards_distributed,
            average_apy,
            estimated_annual_rewards: self.calculate_reward(total_staked, self.config.blocks_per_year),
        }
    }
}

/// Reward statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardStatistics {
    pub total_stakers: u64,
    pub active_stakers: u64,
    pub total_staked: u64,
    pub treasury_balance: u64,
    pub total_rewards_distributed: u64,
    pub average_apy: f64,
    pub estimated_annual_rewards: u64,
}
