//! Consensus Tier (Tier 2): DAG-based consensus engine
//!
//! Responsibilities:
//! - DAG vertex creation and validation
//! - Slot mining with difficulty adjustment
//! - Fork resolution and canonical path selection
//! - Byzantine fault tolerance
//! - Thread-safe parallel mining

pub mod backend;
pub mod bft;
pub mod dag;
pub mod dpos;
pub mod engine;
pub mod equivocation;
pub mod fork_choice;
pub mod scheduler;
pub mod slashing;
pub mod validator;

/// Consensus configuration
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    #[allow(dead_code)]
    pub slot_duration_ms: u64,
    #[allow(dead_code)]
    pub max_parallel_slots: u32,
    #[allow(dead_code)]
    pub byzantine_fault_tolerance: f64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            slot_duration_ms: 6000,
            max_parallel_slots: 4,
            byzantine_fault_tolerance: 0.33,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = ConsensusConfig::default();
        assert_eq!(cfg.max_parallel_slots, 4);
    }
}
