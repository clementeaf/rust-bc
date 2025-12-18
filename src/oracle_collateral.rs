use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a bonding request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondingStatus {
    Pending,
    Active,
    Released,
}

/// Status of a dispute challenge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeStatus {
    Pending,
    Voting,
    Resolved,
    Rejected,
}

/// Challenge resolution outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeOutcome {
    ChallengeAccepted,
    ChallengeRejected,
    Pending,
}

/// Request to lock collateral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingRequest {
    pub oracle_id: String,
    pub amount: u64,
    pub timestamp: u64,
    pub status: BondingStatus,
}

/// Challenge to oracle's report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeChallenge {
    pub id: String,
    pub oracle_id: String,
    pub challenger_id: String,
    pub report_hash: String,
    pub evidence: String,
    pub timestamp: u64,
    pub status: ChallengeStatus,
    pub votes_for: u64,
    pub votes_against: u64,
    pub voting_end_time: u64,
}

/// Collateral pool for a single oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralPool {
    pub oracle_id: String,
    pub locked_balance: u64,
    pub available_balance: u64,
    pub in_dispute_balance: u64,
    pub min_collateral: u64,
    pub dispute_lock_period_ms: u64,
    pub slashing_factor: u8, // Percentage (0-100)
    pub total_slashed: u64,
}

impl CollateralPool {
    pub fn new(oracle_id: String, min_collateral: u64) -> Self {
        CollateralPool {
            oracle_id,
            locked_balance: 0,
            available_balance: 0,
            in_dispute_balance: 0,
            min_collateral,
            dispute_lock_period_ms: 604800000, // 7 days
            slashing_factor: 10,               // 10%
            total_slashed: 0,
        }
    }

    /// Deposit collateral to the pool
    pub fn deposit(&mut self, amount: u64) {
        self.available_balance += amount;
    }

    /// Request to lock collateral
    pub fn lock_collateral(&mut self, amount: u64) -> Result<(), String> {
        if self.available_balance < amount {
            return Err("Insufficient available balance".to_string());
        }
        self.available_balance -= amount;
        self.locked_balance += amount;
        Ok(())
    }

    /// Move collateral to dispute state
    pub fn dispute_collateral(&mut self, amount: u64) -> Result<(), String> {
        if self.locked_balance < amount {
            return Err("Insufficient locked balance".to_string());
        }
        self.locked_balance -= amount;
        self.in_dispute_balance += amount;
        Ok(())
    }

    /// Release collateral after dispute period
    pub fn release_collateral(&mut self, amount: u64) -> Result<(), String> {
        if self.in_dispute_balance < amount {
            return Err("Insufficient disputed balance".to_string());
        }
        self.in_dispute_balance -= amount;
        self.available_balance += amount;
        Ok(())
    }

    /// Slash collateral due to bad oracle behavior
    pub fn slash_collateral(&mut self, amount: u64) -> Result<u64, String> {
        if self.in_dispute_balance < amount {
            return Err("Insufficient balance to slash".to_string());
        }
        self.in_dispute_balance -= amount;
        let slash_amount = (amount * self.slashing_factor as u64) / 100;
        self.total_slashed += slash_amount;
        Ok(slash_amount)
    }

    /// Restore collateral if challenge fails
    pub fn restore_collateral(&mut self, amount: u64) -> Result<(), String> {
        if self.in_dispute_balance < amount {
            return Err("Insufficient balance to restore".to_string());
        }
        self.in_dispute_balance -= amount;
        self.available_balance += amount;
        Ok(())
    }

    /// Check if oracle can withdraw
    pub fn can_withdraw(&self, amount: u64) -> bool {
        self.available_balance >= amount && self.in_dispute_balance == 0
    }

    /// Get total collateral
    pub fn total_collateral(&self) -> u64 {
        self.locked_balance + self.available_balance + self.in_dispute_balance
    }
}

/// Main bonding and collateral registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingRegistry {
    pub pools: HashMap<String, CollateralPool>,
    pub pending_bonds: Vec<BondingRequest>,
    pub challenges: Vec<DisputeChallenge>,
    pub slashed_total: u64,
    pub slasher_rewards: HashMap<String, u64>,
    pub min_collateral: u64,
    pub voting_period_ms: u64,
    pub challenge_counter: u64,
}

impl BondingRegistry {
    /// Maximum future timestamp drift in milliseconds (5 minutes)
    const MAX_FUTURE_DRIFT_MS: u64 = 300_000;
    /// Maximum past timestamp drift in milliseconds (1 hour)  
    const MAX_PAST_DRIFT_MS: u64 = 3_600_000;

    pub fn new(min_collateral: u64) -> Self {
        BondingRegistry {
            pools: HashMap::new(),
            pending_bonds: Vec::new(),
            challenges: Vec::new(),
            slashed_total: 0,
            slasher_rewards: HashMap::new(),
            min_collateral,
            voting_period_ms: 259200000, // 3 days
            challenge_counter: 0,
        }
    }

    /// Create collateral pool for oracle
    pub fn create_pool(&mut self, oracle_id: String) -> Result<(), String> {
        if self.pools.contains_key(&oracle_id) {
            return Err("Pool already exists".to_string());
        }
        self.pools
            .insert(oracle_id.clone(), CollateralPool::new(oracle_id, self.min_collateral));
        Ok(())
    }

    /// Deposit collateral to oracle pool
    pub fn deposit_collateral(&mut self, oracle_id: &str, amount: u64) -> Result<(), String> {
        if !self.pools.contains_key(oracle_id) {
            self.create_pool(oracle_id.to_string())?;
        }

        if let Some(pool) = self.pools.get_mut(oracle_id) {
            pool.deposit(amount);
            Ok(())
        } else {
            Err("Pool not found".to_string())
        }
    }

    /// Request to lock collateral for bonding
    pub fn lock_collateral(&mut self, oracle_id: &str, amount: u64, timestamp: u64) -> Result<(), String> {
        if amount < self.min_collateral {
            return Err(format!("Amount {} below minimum {}", amount, self.min_collateral));
        }

        if let Some(pool) = self.pools.get_mut(oracle_id) {
            pool.lock_collateral(amount)?;
            self.pending_bonds.push(BondingRequest {
                oracle_id: oracle_id.to_string(),
                amount,
                timestamp,
                status: BondingStatus::Pending,
            });
            Ok(())
        } else {
            Err("Pool not found".to_string())
        }
    }

    /// Activate a pending bond
    pub fn activate_bond(&mut self, oracle_id: &str) -> Result<(), String> {
        if let Some(bond) = self.pending_bonds.iter_mut().find(|b| b.oracle_id == oracle_id) {
            bond.status = BondingStatus::Active;
            Ok(())
        } else {
            Err("Bond not found".to_string())
        }
    }

    /// Get collateral status
    pub fn get_collateral_status(&self, oracle_id: &str) -> Result<(u64, u64, u64), String> {
        self.pools
            .get(oracle_id)
            .map(|pool| (pool.locked_balance, pool.available_balance, pool.in_dispute_balance))
            .ok_or_else(|| "Pool not found".to_string())
    }

    /// Validate timestamp is within acceptable range
    fn validate_challenge_timestamp(current_time: u64, challenge_time: u64) -> Result<(), String> {
        if challenge_time > current_time {
            let drift = challenge_time - current_time;
            if drift > Self::MAX_FUTURE_DRIFT_MS {
                return Err("Challenge timestamp too far in future".to_string());
            }
        }
        if current_time > challenge_time {
            let drift = current_time - challenge_time;
            if drift > Self::MAX_PAST_DRIFT_MS {
                return Err("Challenge timestamp too old".to_string());
            }
        }
        Ok(())
    }

    /// Create a challenge to oracle's report
    pub fn challenge_oracle(
        &mut self,
        oracle_id: &str,
        challenger_id: &str,
        report_hash: String,
        evidence: String,
        current_time: u64,
    ) -> Result<String, String> {
        // Validate challenge timestamp
        Self::validate_challenge_timestamp(current_time, current_time)?;

        // Check oracle has active bond
        if !self.has_active_bond(oracle_id) {
            return Err("Oracle not bonded".to_string());
        }

        // Check oracle doesn't already have an active challenge
        if self.challenges.iter().any(|c| c.oracle_id == oracle_id && c.status == ChallengeStatus::Voting) {
            return Err("Oracle already has an active challenge".to_string());
        }

        // Move collateral to dispute state
        if let Some(pool) = self.pools.get_mut(oracle_id) {
            let locked = pool.locked_balance;
            pool.dispute_collateral(locked)?;
        }

        let challenge_id = format!("challenge_{}", self.challenge_counter);
        self.challenge_counter += 1;

        let challenge = DisputeChallenge {
            id: challenge_id.clone(),
            oracle_id: oracle_id.to_string(),
            challenger_id: challenger_id.to_string(),
            report_hash,
            evidence,
            timestamp: current_time,
            status: ChallengeStatus::Voting,
            votes_for: 0,
            votes_against: 0,
            voting_end_time: current_time + self.voting_period_ms,
        };

        self.challenges.push(challenge);
        Ok(challenge_id)
    }

    /// Calculate reputation-based vote weight
    /// Weight is (1 + reputation / 100) to ensure even users with 0 reputation get 1 vote
    fn calculate_vote_weight(&self, voter_id: &str) -> u64 {
        // Look up voter reputation from oracle pool if they are an oracle
        if let Some(pool) = self.pools.get(voter_id) {
            let rep = pool.total_collateral().max(1) / 100; // Normalize collateral as proxy for reputation
            (1 + rep).min(10) // Cap at 10x weight to prevent dominance
        } else {
            // Non-oracle voters get base weight of 1
            1
        }
    }

    /// Vote on a challenge with reputation-weighted voting
    pub fn vote_on_challenge(&mut self, challenge_id: &str, voter_id: &str, vote_yes: bool) -> Result<(), String> {
        // First check status and calculate weight without holding mutable reference
        let challenge_status = {
            self
                .challenges
                .iter()
                .find(|c| c.id == challenge_id)
                .ok_or("Challenge not found")?
                .status
        };

        if challenge_status != ChallengeStatus::Voting {
            return Err("Challenge is not in voting period".to_string());
        }

        // Calculate reputation-based weight (this doesn't hold references to challenges)
        let weight = self.calculate_vote_weight(voter_id);

        // Now update the challenge
        let challenge = self
            .challenges
            .iter_mut()
            .find(|c| c.id == challenge_id)
            .ok_or("Challenge not found")?;

        if vote_yes {
            challenge.votes_for += weight;
        } else {
            challenge.votes_against += weight;
        }

        Ok(())
    }

    /// Resolve a challenge (check if voting period ended)
    pub fn resolve_challenge(&mut self, challenge_id: &str, _current_time: u64) -> Result<ChallengeOutcome, String> {
        // Extract data from challenge without holding mutable reference
        let (oracle_id, challenger_id, votes_for, votes_against) = {
            let challenge = self
                .challenges
                .iter()
                .find(|c| c.id == challenge_id)
                .ok_or("Challenge not found")?;

            if challenge.status != ChallengeStatus::Voting {
                return Err("Challenge is not in voting period".to_string());
            }

            if _current_time < challenge.voting_end_time {
                return Err("Voting period not ended".to_string());
            }

            (
                challenge.oracle_id.clone(),
                challenge.challenger_id.clone(),
                challenge.votes_for,
                challenge.votes_against,
            )
        };

        let outcome = if votes_for > votes_against {
            ChallengeOutcome::ChallengeAccepted
        } else {
            ChallengeOutcome::ChallengeRejected
        };

        // Update challenge status
        if let Some(challenge) = self.challenges.iter_mut().find(|c| c.id == challenge_id) {
            challenge.status = ChallengeStatus::Resolved;
        }

        // Execute slashing if challenge accepted
        if outcome == ChallengeOutcome::ChallengeAccepted {
            self.execute_slashing(&oracle_id, &challenger_id)?
        } else {
            // Restore collateral if challenge rejected
            if let Some(pool) = self.pools.get_mut(&oracle_id) {
                pool.restore_collateral(pool.in_dispute_balance)?
            }
        }

        Ok(outcome)
    }

    /// Execute slashing and reward slasher
    fn execute_slashing(&mut self, oracle_id: &str, challenger_id: &str) -> Result<(), String> {
        if let Some(pool) = self.pools.get_mut(oracle_id) {
            let slash_amount = pool.slash_collateral(pool.in_dispute_balance)?;

            // Reward the slasher (5% of slashed amount)
            let reward = (slash_amount * 5) / 100;
            *self.slasher_rewards.entry(challenger_id.to_string()).or_insert(0) += reward;

            self.slashed_total += slash_amount;
        }

        Ok(())
    }

    /// Check if oracle has active bond
    pub fn has_active_bond(&self, oracle_id: &str) -> bool {
        self.pending_bonds
            .iter()
            .any(|b| b.oracle_id == oracle_id && b.status == BondingStatus::Active)
    }

    /// Release collateral after dispute period
    pub fn release_collateral(&mut self, oracle_id: &str, _current_time: u64) -> Result<(), String> {
        // Check if there are active disputes
        let has_active_disputes = self
            .challenges
            .iter()
            .any(|c| c.oracle_id == oracle_id && c.status != ChallengeStatus::Resolved);

        if has_active_disputes {
            return Err("Oracle has active disputes".to_string());
        }

        if let Some(pool) = self.pools.get_mut(oracle_id) {
            let amount_to_release = pool.in_dispute_balance;
            pool.release_collateral(amount_to_release)?;
            Ok(())
        } else {
            Err("Pool not found".to_string())
        }
    }

    /// Withdraw available collateral
    pub fn withdraw_collateral(&mut self, oracle_id: &str, amount: u64) -> Result<u64, String> {
        if let Some(pool) = self.pools.get_mut(oracle_id) {
            if !pool.can_withdraw(amount) {
                return Err("Cannot withdraw now (disputes pending or insufficient balance)".to_string());
            }
            pool.available_balance -= amount;
            Ok(amount)
        } else {
            Err("Pool not found".to_string())
        }
    }

    /// Claim slasher rewards
    pub fn claim_slasher_reward(&mut self, challenger_id: &str) -> Result<u64, String> {
        self.slasher_rewards
            .remove(challenger_id)
            .ok_or_else(|| "No rewards available".to_string())
    }

    /// Get slasher rewards
    pub fn get_slasher_rewards(&self, challenger_id: &str) -> u64 {
        *self.slasher_rewards.get(challenger_id).unwrap_or(&0)
    }

    /// Get challenge by ID
    pub fn get_challenge(&self, challenge_id: &str) -> Option<&DisputeChallenge> {
        self.challenges.iter().find(|c| c.id == challenge_id)
    }

    /// Get all challenges for an oracle
    pub fn get_oracle_challenges(&self, oracle_id: &str) -> Vec<&DisputeChallenge> {
        self.challenges.iter().filter(|c| c.oracle_id == oracle_id).collect()
    }

    /// Get pending challenges count
    pub fn pending_challenges_count(&self) -> usize {
        self.challenges.iter().filter(|c| c.status == ChallengeStatus::Voting).count()
    }

    /// Update voting period
    pub fn set_voting_period(&mut self, period_ms: u64) {
        self.voting_period_ms = period_ms;
    }

    /// Update minimum collateral
    pub fn set_min_collateral(&mut self, min: u64) {
        self.min_collateral = min;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collateral_pool_creation() {
        let pool = CollateralPool::new("oracle1".to_string(), 1000);
        assert_eq!(pool.locked_balance, 0);
        assert_eq!(pool.available_balance, 0);
        assert_eq!(pool.min_collateral, 1000);
    }

    #[test]
    fn test_deposit_collateral() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(5000);
        assert_eq!(pool.available_balance, 5000);
    }

    #[test]
    fn test_lock_collateral_success() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(5000);

        let result = pool.lock_collateral(2000);
        assert!(result.is_ok());
        assert_eq!(pool.locked_balance, 2000);
        assert_eq!(pool.available_balance, 3000);
    }

    #[test]
    fn test_lock_collateral_insufficient() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(1000);

        let result = pool.lock_collateral(2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_collateral() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(5000);
        pool.lock_collateral(2000).unwrap();

        let result = pool.dispute_collateral(2000);
        assert!(result.is_ok());
        assert_eq!(pool.locked_balance, 0);
        assert_eq!(pool.in_dispute_balance, 2000);
    }

    #[test]
    fn test_slash_collateral() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(5000);
        pool.lock_collateral(2000).unwrap();
        pool.dispute_collateral(2000).unwrap();

        let slashed = pool.slash_collateral(2000).unwrap();
        assert_eq!(slashed, 200); // 10% of 2000
        assert_eq!(pool.total_slashed, 200);
        assert_eq!(pool.in_dispute_balance, 0);
    }

    #[test]
    fn test_can_withdraw() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(5000);
        assert!(pool.can_withdraw(1000));

        pool.lock_collateral(2000).unwrap();
        assert!(pool.can_withdraw(1000));

        pool.dispute_collateral(2000).unwrap();
        assert!(!pool.can_withdraw(1000)); // Can't withdraw with disputed balance
    }

    #[test]
    fn test_total_collateral() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(5000);
        pool.lock_collateral(2000).unwrap();

        assert_eq!(pool.total_collateral(), 5000);
    }

    #[test]
    fn test_bonding_registry_creation() {
        let registry = BondingRegistry::new(1000);
        assert_eq!(registry.min_collateral, 1000);
        assert_eq!(registry.voting_period_ms, 259200000);
        assert_eq!(registry.pools.len(), 0);
    }

    #[test]
    fn test_create_pool() {
        let mut registry = BondingRegistry::new(1000);
        let result = registry.create_pool("oracle1".to_string());
        assert!(result.is_ok());
        assert!(registry.pools.contains_key("oracle1"));
    }

    #[test]
    fn test_registry_deposit_collateral() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        let result = registry.deposit_collateral("oracle1", 5000);

        assert!(result.is_ok());
        let (_, available, _) = registry.get_collateral_status("oracle1").unwrap();
        assert_eq!(available, 5000);
    }

    #[test]
    fn test_lock_collateral_below_minimum() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();

        let result = registry.lock_collateral("oracle1", 500, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_lock_collateral_success() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();

        let result = registry.lock_collateral("oracle1", 1500, 1000);
        assert!(result.is_ok());

        let (locked, available, _) = registry.get_collateral_status("oracle1").unwrap();
        assert_eq!(locked, 1500);
        assert_eq!(available, 3500);
    }

    #[test]
    fn test_activate_bond() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();

        let result = registry.activate_bond("oracle1");
        assert!(result.is_ok());
        assert!(registry.has_active_bond("oracle1"));
    }

    #[test]
    fn test_challenge_oracle() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let result = registry.challenge_oracle(
            "oracle1",
            "challenger1",
            "hash123".to_string(),
            "Oracle reported wrong price".to_string(),
            2000,
        );

        assert!(result.is_ok());
        assert_eq!(registry.challenges.len(), 1);
    }

    #[test]
    fn test_vote_on_challenge() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let challenge_id = registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        registry.vote_on_challenge(&challenge_id, "voter1", true).unwrap();
        registry.vote_on_challenge(&challenge_id, "voter2", false).unwrap();

        let challenge = registry.get_challenge(&challenge_id).unwrap();
        assert!(challenge.votes_for > 0);
        assert!(challenge.votes_against > 0);
    }

    #[test]
    fn test_resolve_challenge_accepted() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let challenge_id = registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        // Vote for challenge
        registry.vote_on_challenge(&challenge_id, "voter1", true).unwrap();
        registry.vote_on_challenge(&challenge_id, "voter2", true).unwrap();

        // Resolve after voting period
        let outcome = registry.resolve_challenge(&challenge_id, 3000000000).unwrap();
        assert_eq!(outcome, ChallengeOutcome::ChallengeAccepted);

        // Check slasher got rewarded
        let reward = registry.get_slasher_rewards("challenger1");
        assert!(reward > 0);
    }

    #[test]
    fn test_resolve_challenge_rejected() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let challenge_id = registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        // Vote against challenge
        registry.vote_on_challenge(&challenge_id, "voter1", false).unwrap();
        registry.vote_on_challenge(&challenge_id, "voter2", false).unwrap();

        // Resolve after voting period
        let outcome = registry.resolve_challenge(&challenge_id, 3000000000).unwrap();
        assert_eq!(outcome, ChallengeOutcome::ChallengeRejected);

        // Collateral should be restored
        let (_, _, disputed) = registry.get_collateral_status("oracle1").unwrap();
        assert_eq!(disputed, 0);
    }

    #[test]
    fn test_withdraw_collateral_success() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();

        let result = registry.withdraw_collateral("oracle1", 2000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2000);

        let (_, available, _) = registry.get_collateral_status("oracle1").unwrap();
        assert_eq!(available, 3000);
    }

    #[test]
    fn test_withdraw_collateral_with_dispute() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();
        registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        let result = registry.withdraw_collateral("oracle1", 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_claim_slasher_reward() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let challenge_id = registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        registry.vote_on_challenge(&challenge_id, "voter1", true).unwrap();
        registry.resolve_challenge(&challenge_id, 3000000000).unwrap();

        let reward = registry.claim_slasher_reward("challenger1").unwrap();
        assert!(reward > 0);

        // Second claim should fail
        let result = registry.claim_slasher_reward("challenger1");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_oracle_challenges() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let c1 = registry
            .challenge_oracle("oracle1", "challenger1", "hash1".to_string(), "evidence".to_string(), 2000)
            .unwrap();
        
        // Resolve first challenge
        registry.vote_on_challenge(&c1, "voter1", false).unwrap();
        registry.resolve_challenge(&c1, 3000000000).unwrap();
        
        // Now create second challenge
        registry
            .challenge_oracle("oracle1", "challenger2", "hash2".to_string(), "evidence".to_string(), 3000)
            .unwrap();

        let challenges = registry.get_oracle_challenges("oracle1");
        assert_eq!(challenges.len(), 2);
    }

    #[test]
    fn test_pending_challenges_count() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        registry
            .challenge_oracle("oracle1", "challenger1", "hash1".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        assert_eq!(registry.pending_challenges_count(), 1);
    }

    #[test]
    fn test_set_voting_period() {
        let mut registry = BondingRegistry::new(1000);
        registry.set_voting_period(500000);
        assert_eq!(registry.voting_period_ms, 500000);
    }

    #[test]
    fn test_set_min_collateral() {
        let mut registry = BondingRegistry::new(1000);
        registry.set_min_collateral(5000);
        assert_eq!(registry.min_collateral, 5000);
    }

    #[test]
    fn test_has_active_bond() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();

        assert!(!registry.has_active_bond("oracle1"));

        registry.activate_bond("oracle1").unwrap();
        assert!(registry.has_active_bond("oracle1"));
    }

    #[test]
    fn test_multiple_oracles() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.create_pool("oracle2".to_string()).unwrap();

        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.deposit_collateral("oracle2", 3000).unwrap();

        let (_, avail1, _) = registry.get_collateral_status("oracle1").unwrap();
        let (_, avail2, _) = registry.get_collateral_status("oracle2").unwrap();

        assert_eq!(avail1, 5000);
        assert_eq!(avail2, 3000);
    }

    #[test]
    fn test_challenge_non_bonded_oracle() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();

        let result = registry.challenge_oracle("oracle1", "challenger1", "hash".to_string(), "evidence".to_string(), 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_voting_weight_by_voter_id() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let challenge_id = registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        // Voter with longer ID gets more weight
        registry.vote_on_challenge(&challenge_id, "v1", true).unwrap();
        registry.vote_on_challenge(&challenge_id, "very_long_voter_name", true).unwrap();

        let challenge = registry.get_challenge(&challenge_id).unwrap();
        assert!(challenge.votes_for >= 2); // At least 2 votes
    }

    #[test]
    fn test_release_collateral_with_disputes() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 1500, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        let result = registry.release_collateral("oracle1", 3000);
        assert!(result.is_err()); // Can't release while disputed
    }

    #[test]
    fn test_slash_calculation() {
        let mut pool = CollateralPool::new("oracle1".to_string(), 1000);
        pool.deposit(10000);
        pool.lock_collateral(5000).unwrap();
        pool.dispute_collateral(5000).unwrap();

        // Slash 10% of 5000 = 500
        let slashed = pool.slash_collateral(5000).unwrap();
        assert_eq!(slashed, 500);
    }

    #[test]
    fn test_concurrent_challenges() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 10000).unwrap();
        registry.lock_collateral("oracle1", 3000, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        // First challenge
        let _c1 = registry
            .challenge_oracle("oracle1", "challenger1", "hash1".to_string(), "evidence1".to_string(), 2000)
            .unwrap();

        // Second challenge should fail because oracle is disputed
        let c2 = registry.challenge_oracle("oracle1", "challenger2", "hash2".to_string(), "evidence2".to_string(), 3000);

        assert!(c2.is_err()); // Can't challenge if already disputed
    }

    #[test]
    fn test_reward_calculation() {
        let mut registry = BondingRegistry::new(1000);
        registry.create_pool("oracle1".to_string()).unwrap();
        registry.deposit_collateral("oracle1", 5000).unwrap();
        registry.lock_collateral("oracle1", 2000, 1000).unwrap();
        registry.activate_bond("oracle1").unwrap();

        let challenge_id = registry
            .challenge_oracle("oracle1", "challenger1", "hash123".to_string(), "evidence".to_string(), 2000)
            .unwrap();

        registry.vote_on_challenge(&challenge_id, "voter1", true).unwrap();
        registry.resolve_challenge(&challenge_id, 3000000000).unwrap();

        let reward = registry.get_slasher_rewards("challenger1");
        // 10% of 2000 = 200 slash, 5% of 200 = 10 reward
        assert_eq!(reward, 10);
    }
}
