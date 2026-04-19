//! Proof of Contribution: earn curvature by sustaining the field.
//!
//! Not proof of work (useless puzzles). Not proof of stake (capital lockup).
//! Proof of REAL contribution: storage, connectivity, healing, uptime.
//!
//! Measurable. Proportional. From a finite pool. Not inflationary.

/// Metrics of a node's real contribution to the field.
#[derive(Clone, Debug, Default)]
pub struct ContributionMetrics {
    /// Number of cells this node maintains in its projection.
    pub cells_maintained: u64,
    /// Number of boundary exchanges completed (connectivity).
    pub boundary_exchanges: u64,
    /// Number of times this node helped another recover (healing).
    pub recoveries_assisted: u64,
    /// Total seconds of uptime (reliability).
    pub uptime_seconds: u64,
    /// Number of events seeded through this node (activity).
    pub events_processed: u64,
}

impl ContributionMetrics {
    /// Composite contribution score. Weighted sum of all metrics.
    /// Each metric contributes differently — healing is worth more
    /// than just being online, because it's harder and more valuable.
    pub fn score(&self) -> f64 {
        let storage   = self.cells_maintained as f64 * 0.1;
        let network   = self.boundary_exchanges as f64 * 1.0;
        let healing   = self.recoveries_assisted as f64 * 5.0;
        let uptime    = (self.uptime_seconds as f64 / 3600.0) * 0.5; // per hour
        let activity  = self.events_processed as f64 * 2.0;

        storage + network + healing + uptime + activity
    }
}

/// Growth pool: finite curvature that gets distributed to contributors.
/// Not a faucet. Not a handout. Proportional to measured contribution.
pub struct GrowthPool {
    /// Remaining curvature in the pool.
    remaining: f64,
    /// Total ever distributed.
    distributed: f64,
    /// Rate: curvature per contribution point per epoch.
    /// Decreases over time as the pool depletes (natural halving).
    rate: f64,
    /// Minimum contribution score to qualify for distribution.
    min_score: f64,
}

impl GrowthPool {
    pub fn new(total: f64) -> Self {
        Self {
            remaining: total,
            distributed: 0.0,
            rate: 1.0,
            min_score: 10.0,
        }
    }

    pub fn remaining(&self) -> f64 {
        self.remaining
    }

    pub fn distributed(&self) -> f64 {
        self.distributed
    }

    /// Calculate reward for a contribution. Does NOT distribute yet.
    pub fn calculate_reward(&self, metrics: &ContributionMetrics) -> f64 {
        let score = metrics.score();
        if score < self.min_score { return 0.0; }
        if self.remaining <= 0.0 { return 0.0; }

        let raw_reward = score * self.rate;
        // Can't distribute more than what remains
        raw_reward.min(self.remaining)
    }

    /// Distribute reward to a contributor. Returns actual amount distributed.
    /// Reduces the pool. When pool hits zero, no more distribution ever.
    pub fn distribute(&mut self, metrics: &ContributionMetrics) -> f64 {
        let reward = self.calculate_reward(metrics);
        if reward <= 0.0 { return 0.0; }

        self.remaining -= reward;
        self.distributed += reward;

        // Natural rate decay: as pool depletes, rate decreases.
        // Not a halving schedule — a continuous decay proportional to depletion.
        // When 50% distributed → rate = 50% of original.
        // When 90% distributed → rate = 10% of original.
        let depletion = self.distributed / (self.distributed + self.remaining);
        self.rate = 1.0 - depletion;

        reward
    }

    /// Distribute proportionally to multiple contributors in one epoch.
    /// Each gets a share proportional to their score.
    pub fn distribute_epoch(&mut self, contributors: &[ContributionMetrics]) -> Vec<f64> {
        let scores: Vec<f64> = contributors.iter()
            .map(|m| {
                let s = m.score();
                if s >= self.min_score { s } else { 0.0 }
            })
            .collect();

        let total_score: f64 = scores.iter().sum();
        if total_score <= 0.0 || self.remaining <= 0.0 {
            return vec![0.0; contributors.len()];
        }

        // Budget for this epoch: proportional to remaining pool
        // Early epochs distribute more, later epochs less (natural decay)
        let epoch_budget = (self.remaining * 0.01).max(0.001); // 1% of remaining per epoch

        let mut rewards = Vec::with_capacity(contributors.len());
        let mut total_distributed = 0.0;

        for score in &scores {
            if *score <= 0.0 {
                rewards.push(0.0);
                continue;
            }
            let share = score / total_score;
            let reward = (epoch_budget * share).min(self.remaining - total_distributed);
            rewards.push(reward);
            total_distributed += reward;
        }

        self.remaining -= total_distributed;
        self.distributed += total_distributed;

        rewards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_contribution_earns_nothing() {
        let pool = GrowthPool::new(1_000_000.0);
        let metrics = ContributionMetrics::default();
        assert_eq!(pool.calculate_reward(&metrics), 0.0);
    }

    #[test]
    fn real_contribution_earns_curvature() {
        let mut pool = GrowthPool::new(1_000_000.0);
        let metrics = ContributionMetrics {
            cells_maintained: 500,
            boundary_exchanges: 100,
            recoveries_assisted: 5,
            uptime_seconds: 3600,
            events_processed: 20,
        };

        let score = metrics.score();
        assert!(score > 10.0, "Real contribution should have score > minimum");

        let reward = pool.distribute(&metrics);
        assert!(reward > 0.0, "Should earn curvature");
        assert!(pool.remaining() < 1_000_000.0, "Pool should decrease");
    }

    #[test]
    fn pool_depletes_over_time() {
        let mut pool = GrowthPool::new(1000.0);
        let metrics = ContributionMetrics {
            cells_maintained: 100,
            boundary_exchanges: 50,
            recoveries_assisted: 10,
            uptime_seconds: 7200,
            events_processed: 30,
        };

        let mut total_earned = 0.0;
        let mut rounds = 0;

        // Keep contributing until pool is nearly empty
        while pool.remaining() > 1.0 && rounds < 1000 {
            let reward = pool.distribute(&metrics);
            total_earned += reward;
            rounds += 1;
        }

        // Pool should be nearly depleted
        assert!(pool.remaining() < 2.0, "Pool should deplete");
        // Total earned should be close to original pool
        assert!((total_earned + pool.remaining() - 1000.0).abs() < 0.01,
            "Conservation: earned ({}) + remaining ({}) should = 1000",
            total_earned, pool.remaining());
    }

    #[test]
    fn rate_decays_as_pool_depletes() {
        let mut pool = GrowthPool::new(1000.0);
        let metrics = ContributionMetrics {
            cells_maintained: 1000,
            boundary_exchanges: 100,
            recoveries_assisted: 10,
            uptime_seconds: 3600,
            events_processed: 50,
        };

        let first_reward = pool.distribute(&metrics);
        // Distribute half the pool
        while pool.remaining() > 500.0 {
            pool.distribute(&metrics);
        }
        let mid_reward = pool.distribute(&metrics);

        // Later rewards should be smaller (rate decay)
        assert!(mid_reward < first_reward,
            "Rewards should decrease as pool depletes: first={:.4}, mid={:.4}",
            first_reward, mid_reward);
    }

    #[test]
    fn epoch_distribution_proportional() {
        let mut pool = GrowthPool::new(100_000.0);

        let heavy = ContributionMetrics {
            cells_maintained: 1000,
            boundary_exchanges: 200,
            recoveries_assisted: 20,
            uptime_seconds: 86400,
            events_processed: 100,
        };
        let light = ContributionMetrics {
            cells_maintained: 100,
            boundary_exchanges: 20,
            recoveries_assisted: 0,
            uptime_seconds: 3600,
            events_processed: 5,
        };
        let zero = ContributionMetrics::default();

        let rewards = pool.distribute_epoch(&[heavy.clone(), light.clone(), zero]);

        // Heavy contributor gets more than light
        assert!(rewards[0] > rewards[1],
            "Heavy contributor ({:.4}) should earn more than light ({:.4})",
            rewards[0], rewards[1]);
        // Zero contributor gets nothing
        assert_eq!(rewards[2], 0.0);
        // Total distributed from pool
        assert!(pool.remaining() < 100_000.0);
    }

    #[test]
    fn empty_pool_distributes_nothing() {
        let mut pool = GrowthPool::new(0.0);
        let metrics = ContributionMetrics {
            cells_maintained: 10000,
            boundary_exchanges: 1000,
            recoveries_assisted: 100,
            uptime_seconds: 86400 * 365,
            events_processed: 10000,
        };

        let reward = pool.distribute(&metrics);
        assert_eq!(reward, 0.0, "Empty pool should give nothing, no matter the contribution");
    }

    #[test]
    fn pool_conservation_with_multiple_participants() {
        let mut pool = GrowthPool::new(50_000.0);

        let participants: Vec<ContributionMetrics> = (0..10).map(|i| {
            ContributionMetrics {
                cells_maintained: 100 * (i + 1),
                boundary_exchanges: 50 * (i + 1),
                recoveries_assisted: i * 2,
                uptime_seconds: 3600 * (i + 1),
                events_processed: 10 * (i + 1),
            }
        }).collect();

        let mut total_distributed = 0.0;

        // 100 epochs
        for _ in 0..100 {
            let rewards = pool.distribute_epoch(&participants);
            total_distributed += rewards.iter().sum::<f64>();
        }

        // Conservation: distributed + remaining = original
        assert!(
            (total_distributed + pool.remaining() - 50_000.0).abs() < 0.01,
            "Conservation: {:.2} + {:.2} should = 50000",
            total_distributed, pool.remaining()
        );
    }
}
