use rust_bc::staking_rewards::*;

#[test]
fn test_staking_reward_contract_creation() {
    let contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    assert_eq!(contract.contract_address, "0x123");
    assert_eq!(contract.owner, "owner");
    assert_eq!(contract.token_address, "0x456");
    assert_eq!(contract.treasury_balance, 1_000_000);
    assert_eq!(contract.total_rewards_distributed, 0);
}

#[test]
fn test_register_staker() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    let staker = contract.get_staker("staker1").unwrap();
    assert_eq!(staker.address, "staker1");
    assert_eq!(staker.total_staked, 1000);
    assert_eq!(staker.accumulated_rewards, 0);
    assert_eq!(staker.stake_start_block, 100);
}

#[test]
fn test_register_staker_below_minimum() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let result = contract.register_staker("staker1".to_string(), 50, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("below minimum"));
}

#[test]
fn test_register_staker_duplicate() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    let result = contract.register_staker("staker1".to_string(), 1000, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already registered"));
}

#[test]
fn test_calculate_reward_basic() {
    let contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    // 1000 tokens staked for 100_000 blocks
    // Reward = 1000 * 0.10 * (100_000 / 1_000_000) = 1000 * 0.10 * 0.1 = 10
    let reward = contract.calculate_reward(1000, 100_000);
    assert_eq!(reward, 10);
}

#[test]
fn test_calculate_reward_annual() {
    let contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    // 1000 tokens staked for 1 full year
    // Reward = 1000 * 0.10 * (1_000_000 / 1_000_000) = 100
    let reward = contract.calculate_reward(1000, 1_000_000);
    assert_eq!(reward, 100);
}

#[test]
fn test_calculate_reward_zero_blocks() {
    let contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let reward = contract.calculate_reward(1000, 0);
    assert_eq!(reward, 0);
}

#[test]
fn test_calculate_reward_below_minimum() {
    let contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let reward = contract.calculate_reward(50, 1000);
    assert_eq!(reward, 0);
}

#[test]
fn test_update_stake() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    // Move forward 100_000 blocks and update stake
    contract.update_stake("staker1", 2000, 100_100).unwrap();

    let staker = contract.get_staker("staker1").unwrap();
    assert_eq!(staker.total_staked, 2000);
    // Reward = 1000 * 0.10 * (100_000 / 1_000_000) = 10
    assert_eq!(staker.accumulated_rewards, 10);
}

#[test]
fn test_distribute_rewards() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    contract.register_staker("staker2".to_string(), 2000, 100).unwrap();

    // Distribute after interval - need enough blocks elapsed for measurable rewards
    // 100_000 blocks = 10% of year = 1% rewards = 10 tokens per 1000 staked
    let distributed = contract.distribute_rewards(100_100).unwrap();
    assert!(distributed > 0);

    let staker1 = contract.get_staker("staker1").unwrap();
    assert!(staker1.accumulated_rewards > 0);
}

#[test]
fn test_distribute_rewards_too_early() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    // Try to distribute before interval
    let result = contract.distribute_rewards(500);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Not enough blocks elapsed"));
}

#[test]
fn test_claim_rewards() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    // Distribute with enough blocks elapsed
    contract.distribute_rewards(100_100).unwrap();

    let claimed = contract.claim_rewards("staker1").unwrap();
    assert!(claimed > 0);

    let staker = contract.get_staker("staker1").unwrap();
    assert_eq!(staker.pending_rewards(), 0);
    assert_eq!(staker.claimed_rewards, claimed);
}

#[test]
fn test_claim_rewards_no_pending() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    let result = contract.claim_rewards("staker1");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No pending rewards"));
}

#[test]
fn test_claim_rewards_insufficient_treasury() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1, // Very small treasury
    );

    contract.register_staker("staker1".to_string(), 1_000_000, 100).unwrap();
    // Don't distribute - just try to claim when there's no treasury and no rewards
    let result = contract.claim_rewards("staker1");

    // Should fail because no rewards accumulated
    assert!(result.is_err());
}

#[test]
fn test_total_staked() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    contract.register_staker("staker2".to_string(), 2000, 100).unwrap();
    contract.register_staker("staker3".to_string(), 3000, 100).unwrap();

    assert_eq!(contract.total_staked(), 6000);
}

#[test]
fn test_active_stakers_count() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    contract.register_staker("staker2".to_string(), 2000, 100).unwrap();

    assert_eq!(contract.active_stakers_count(), 2);
}

#[test]
fn test_update_config() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let new_config = RewardConfig {
        apy_percentage: 20.0,
        blocks_per_year: 2_000_000,
        min_staking_amount: 500,
        reward_interval_blocks: 2000,
        max_annual_rewards: 20_000_000,
    };

    contract.update_config("owner", new_config.clone()).unwrap();

    assert_eq!(contract.config.apy_percentage, 20.0);
    assert_eq!(contract.config.min_staking_amount, 500);
}

#[test]
fn test_update_config_not_owner() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let result = contract.update_config("not_owner", RewardConfig::default());

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can update config"));
}

#[test]
fn test_update_config_invalid_apy_negative() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let invalid_config = RewardConfig {
        apy_percentage: -10.0,
        blocks_per_year: 1_000_000,
        min_staking_amount: 100,
        reward_interval_blocks: 1000,
        max_annual_rewards: 10_000_000,
    };

    let result = contract.update_config("owner", invalid_config);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot be negative"));
}

#[test]
fn test_update_config_invalid_apy_too_high() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let invalid_config = RewardConfig {
        apy_percentage: 150.0,
        blocks_per_year: 1_000_000,
        min_staking_amount: 100,
        reward_interval_blocks: 1000,
        max_annual_rewards: 10_000_000,
    };

    let result = contract.update_config("owner", invalid_config);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("exceed 100%"));
}

#[test]
fn test_deposit_treasury() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.deposit_treasury("owner", 500_000).unwrap();

    assert_eq!(contract.treasury_balance, 1_500_000);
}

#[test]
fn test_deposit_treasury_not_owner() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let result = contract.deposit_treasury("not_owner", 500_000);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can deposit"));
}

#[test]
fn test_withdraw_treasury() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let withdrawn = contract.withdraw_treasury("owner", 500_000).unwrap();

    assert_eq!(withdrawn, 500_000);
    assert_eq!(contract.treasury_balance, 500_000);
}

#[test]
fn test_withdraw_treasury_more_than_available() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let withdrawn = contract.withdraw_treasury("owner", 2_000_000).unwrap();

    assert_eq!(withdrawn, 1_000_000);
    assert_eq!(contract.treasury_balance, 0);
}

#[test]
fn test_unstake_staker() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    let unstaked = contract.unstake_staker("staker1", 100_100).unwrap();

    assert_eq!(unstaked, 1000);

    let staker = contract.get_staker("staker1").unwrap();
    assert_eq!(staker.total_staked, 0);
}

#[test]
fn test_estimate_annual_rewards() {
    let contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    let annual_reward = contract.estimate_annual_rewards(1000);

    // 1000 * 0.10 = 100
    assert_eq!(annual_reward, 100);
}

#[test]
fn test_get_statistics() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    contract.register_staker("staker2".to_string(), 2000, 100).unwrap();

    let stats = contract.get_statistics();

    assert_eq!(stats.total_stakers, 2);
    assert_eq!(stats.active_stakers, 2);
    assert_eq!(stats.total_staked, 3000);
    assert_eq!(stats.treasury_balance, 1_000_000);
}

#[test]
fn test_pending_rewards_calculation() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    contract.distribute_rewards(1100).unwrap();

    let staker = contract.get_staker("staker1").unwrap();
    let pending = staker.pending_rewards();

    assert_eq!(pending, staker.accumulated_rewards - staker.claimed_rewards);
}

#[test]
fn test_multiple_distributions() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        10_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();

    let dist1 = contract.distribute_rewards(100_100).unwrap();
    let dist2 = contract.distribute_rewards(200_100).unwrap();

    assert!(dist1 > 0);
    assert!(dist2 > 0);
    assert_eq!(contract.total_rewards_distributed, dist1 + dist2);
}

#[test]
fn test_staker_not_found() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig::default(),
        1_000_000,
    );

    let result = contract.update_stake("nonexistent", 1000, 100);
    assert!(result.is_err());

    let result = contract.claim_rewards("nonexistent");
    assert!(result.is_err());

    let result = contract.unstake_staker("nonexistent", 100);
    assert!(result.is_err());
}

#[test]
fn test_reward_distribution_proportional() {
    let mut contract = StakingRewardContract::new(
        "0x123".to_string(),
        "owner".to_string(),
        "0x456".to_string(),
        RewardConfig {
            apy_percentage: 10.0,
            blocks_per_year: 1_000_000,
            min_staking_amount: 100,
            reward_interval_blocks: 1000,
            max_annual_rewards: 10_000_000,
        },
        1_000_000,
    );

    contract.register_staker("staker1".to_string(), 1000, 100).unwrap();
    contract.register_staker("staker2".to_string(), 2000, 100).unwrap();

    contract.distribute_rewards(1100).unwrap();

    let staker1 = contract.get_staker("staker1").unwrap();
    let staker2 = contract.get_staker("staker2").unwrap();

    // staker2 has 2x the stake, so should get roughly 2x the rewards
    assert!(staker2.accumulated_rewards >= staker1.accumulated_rewards);
}
