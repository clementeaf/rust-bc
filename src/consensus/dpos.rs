//! Delegated Proof-of-Stake (DPoS) validator selection.
//!
//! Selects validators proportional to their staked amount using weighted
//! random sampling. Integrates with the BFT round manager for leader
//! election and the staking system for stake tracking.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A validator candidate with their stake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorStake {
    pub address: String,
    pub stake: u64,
    pub active: bool,
}

/// Configuration for DPoS selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DposConfig {
    /// Maximum number of active validators in the committee.
    pub max_validators: usize,
    /// Minimum stake required to be eligible.
    pub min_stake: u64,
}

impl Default for DposConfig {
    fn default() -> Self {
        Self {
            max_validators: 150,
            min_stake: 1000,
        }
    }
}

/// Result of a validator selection round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorCommittee {
    /// Selected validators ordered by stake (descending).
    pub members: Vec<ValidatorStake>,
    /// Total stake in the committee.
    pub total_stake: u64,
    /// Epoch for which this committee is valid.
    pub epoch: u64,
}

impl ValidatorCommittee {
    /// Get the leader for a given round (stake-weighted round-robin).
    ///
    /// Higher-stake validators are assigned proportionally more leader slots.
    /// Deterministic: same committee + round always produces same leader.
    pub fn leader_for_round(&self, round: u64) -> Option<&str> {
        if self.members.is_empty() || self.total_stake == 0 {
            return None;
        }

        // Weighted round-robin: map round to a position in the total stake range,
        // then find which validator owns that position.
        let position = round % self.total_stake;
        let mut cumulative = 0u64;

        for member in &self.members {
            cumulative += member.stake;
            if position < cumulative {
                return Some(&member.address);
            }
        }

        // Fallback (shouldn't happen with correct math).
        self.members.last().map(|m| m.address.as_str())
    }

    /// Get a validator's voting power (= their stake).
    pub fn voting_power(&self, address: &str) -> u64 {
        self.members
            .iter()
            .find(|m| m.address == address)
            .map(|m| m.stake)
            .unwrap_or(0)
    }

    /// Whether an address is in the committee.
    pub fn is_member(&self, address: &str) -> bool {
        self.members.iter().any(|m| m.address == address)
    }

    /// Number of validators in the committee.
    pub fn size(&self) -> usize {
        self.members.len()
    }
}

/// Select a validator committee from the candidate pool.
///
/// 1. Filter: only active candidates with `stake >= min_stake`
/// 2. Sort by stake descending (deterministic tie-breaking by address)
/// 3. Take top `max_validators`
pub fn select_committee(
    candidates: &[ValidatorStake],
    config: &DposConfig,
    epoch: u64,
) -> ValidatorCommittee {
    let mut eligible: Vec<ValidatorStake> = candidates
        .iter()
        .filter(|c| c.active && c.stake >= config.min_stake)
        .cloned()
        .collect();

    // Sort by stake desc, then by address asc (deterministic).
    eligible.sort_by(|a, b| {
        b.stake
            .cmp(&a.stake)
            .then_with(|| a.address.cmp(&b.address))
    });

    // Take top N.
    eligible.truncate(config.max_validators);

    let total_stake: u64 = eligible.iter().map(|v| v.stake).sum();

    ValidatorCommittee {
        members: eligible,
        total_stake,
        epoch,
    }
}

/// Calculate the expected leader frequency for each validator over `rounds` rounds.
///
/// Useful for verifying that stake-proportional selection works correctly.
pub fn expected_leader_distribution(
    committee: &ValidatorCommittee,
    rounds: u64,
) -> HashMap<String, u64> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    for round in 0..rounds {
        if let Some(leader) = committee.leader_for_round(round) {
            *counts.entry(leader.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(addr: &str, stake: u64) -> ValidatorStake {
        ValidatorStake {
            address: addr.into(),
            stake,
            active: true,
        }
    }

    fn inactive(addr: &str, stake: u64) -> ValidatorStake {
        ValidatorStake {
            address: addr.into(),
            stake,
            active: false,
        }
    }

    // --- select_committee ---

    #[test]
    fn select_filters_by_min_stake() {
        let candidates = vec![
            candidate("v1", 5000),
            candidate("v2", 500), // Below min_stake
            candidate("v3", 3000),
        ];
        let config = DposConfig {
            min_stake: 1000,
            max_validators: 10,
        };
        let committee = select_committee(&candidates, &config, 0);
        assert_eq!(committee.size(), 2);
        assert!(committee.is_member("v1"));
        assert!(!committee.is_member("v2"));
        assert!(committee.is_member("v3"));
    }

    #[test]
    fn select_filters_inactive() {
        let candidates = vec![
            candidate("v1", 5000),
            inactive("v2", 10_000), // Inactive despite high stake
        ];
        let config = DposConfig::default();
        let committee = select_committee(&candidates, &config, 0);
        assert_eq!(committee.size(), 1);
        assert!(!committee.is_member("v2"));
    }

    #[test]
    fn select_respects_max_validators() {
        let candidates: Vec<ValidatorStake> = (0..200)
            .map(|i| candidate(&format!("v{i}"), 5000 + i))
            .collect();
        let config = DposConfig {
            max_validators: 10,
            min_stake: 1000,
        };
        let committee = select_committee(&candidates, &config, 0);
        assert_eq!(committee.size(), 10);
    }

    #[test]
    fn select_sorted_by_stake_desc() {
        let candidates = vec![
            candidate("low", 1000),
            candidate("high", 10_000),
            candidate("mid", 5000),
        ];
        let config = DposConfig::default();
        let committee = select_committee(&candidates, &config, 0);
        assert_eq!(committee.members[0].address, "high");
        assert_eq!(committee.members[1].address, "mid");
        assert_eq!(committee.members[2].address, "low");
    }

    #[test]
    fn select_deterministic_tie_breaking() {
        let candidates = vec![
            candidate("b", 5000),
            candidate("a", 5000),
            candidate("c", 5000),
        ];
        let config = DposConfig::default();
        let c1 = select_committee(&candidates, &config, 0);
        let c2 = select_committee(&candidates, &config, 0);
        // Same stake → sorted by address ascending.
        assert_eq!(c1.members[0].address, "a");
        assert_eq!(c1.members, c2.members);
    }

    #[test]
    fn select_empty_candidates() {
        let committee = select_committee(&[], &DposConfig::default(), 0);
        assert_eq!(committee.size(), 0);
        assert_eq!(committee.total_stake, 0);
    }

    #[test]
    fn total_stake_computed() {
        let candidates = vec![candidate("v1", 3000), candidate("v2", 7000)];
        let committee = select_committee(&candidates, &DposConfig::default(), 0);
        assert_eq!(committee.total_stake, 10_000);
    }

    // --- leader_for_round (stake-weighted) ---

    #[test]
    fn leader_proportional_to_stake() {
        // v1 has 75% of stake, v2 has 25%.
        let candidates = vec![candidate("v1", 7500), candidate("v2", 2500)];
        let committee = select_committee(&candidates, &DposConfig::default(), 0);

        let dist = expected_leader_distribution(&committee, 10_000);
        let v1_count = dist.get("v1").copied().unwrap_or(0);
        let v2_count = dist.get("v2").copied().unwrap_or(0);

        // v1 should get ~75% of leader slots.
        assert_eq!(v1_count, 7500);
        assert_eq!(v2_count, 2500);
    }

    #[test]
    fn leader_equal_stakes_equal_distribution() {
        let candidates = vec![candidate("v1", 5000), candidate("v2", 5000)];
        let committee = select_committee(&candidates, &DposConfig::default(), 0);

        let dist = expected_leader_distribution(&committee, 10_000);
        assert_eq!(dist.get("v1").copied().unwrap_or(0), 5000);
        assert_eq!(dist.get("v2").copied().unwrap_or(0), 5000);
    }

    #[test]
    fn leader_single_validator() {
        let candidates = vec![candidate("solo", 10_000)];
        let committee = select_committee(&candidates, &DposConfig::default(), 0);

        for round in 0..100 {
            assert_eq!(committee.leader_for_round(round), Some("solo"));
        }
    }

    #[test]
    fn leader_empty_committee() {
        let committee = select_committee(&[], &DposConfig::default(), 0);
        assert_eq!(committee.leader_for_round(0), None);
    }

    #[test]
    fn leader_deterministic_across_calls() {
        let candidates = vec![candidate("v1", 6000), candidate("v2", 4000)];
        let committee = select_committee(&candidates, &DposConfig::default(), 0);

        for round in 0..100 {
            let a = committee.leader_for_round(round);
            let b = committee.leader_for_round(round);
            assert_eq!(a, b, "non-deterministic at round {round}");
        }
    }

    // --- voting_power ---

    #[test]
    fn voting_power_matches_stake() {
        let candidates = vec![candidate("v1", 3000), candidate("v2", 7000)];
        let committee = select_committee(&candidates, &DposConfig::default(), 0);
        assert_eq!(committee.voting_power("v1"), 3000);
        assert_eq!(committee.voting_power("v2"), 7000);
        assert_eq!(committee.voting_power("unknown"), 0);
    }

    // --- stress: 1000 candidates → top 150 ---

    #[test]
    fn stress_1000_candidates() {
        let candidates: Vec<ValidatorStake> = (0..1000)
            .map(|i| candidate(&format!("v{i:04}"), 1000 + (i as u64) * 10))
            .collect();
        let config = DposConfig {
            max_validators: 150,
            min_stake: 1000,
        };
        let committee = select_committee(&candidates, &config, 0);
        assert_eq!(committee.size(), 150);

        // Top validator should be v0999 (highest stake: 1000 + 999*10 = 10990).
        assert_eq!(committee.members[0].address, "v0999");
        assert_eq!(committee.members[0].stake, 10_990);

        // Leader distribution over 100K rounds should be proportional.
        let dist = expected_leader_distribution(&committee, committee.total_stake);
        for member in &committee.members {
            let count = dist.get(&member.address).copied().unwrap_or(0);
            assert_eq!(
                count, member.stake,
                "leader slots mismatch for {}",
                member.address
            );
        }
    }
}
