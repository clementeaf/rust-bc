//! Consensus Tier (Tier 2): DAG-based consensus engine
//!
//! Responsibilities:
//! - DAG vertex creation and validation
//! - Slot mining with difficulty adjustment
//! - Fork resolution and canonical path selection
//! - Byzantine fault tolerance
//! - Thread-safe parallel mining

pub mod dag;
pub mod fork_choice;
pub mod scheduler;
pub mod validator;

pub use dag::{Dag, DagBlock, DagEdge, DagVertex};
pub use fork_choice::{ForkChoice, ForkChoiceRule};
pub use scheduler::SlotScheduler;
pub use validator::{BlockValidator, ValidityResult};

/// Consensus configuration
#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    pub slot_duration_ms: u64,
    pub max_parallel_slots: u32,
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
