//! Consensus Tier (Tier 2): DAG-based consensus engine
//!
//! Responsibilities:
//! - DAG vertex creation and validation
//! - Slot mining with difficulty adjustment
//! - Fork resolution and canonical path selection
//! - Byzantine fault tolerance
//! - Thread-safe parallel mining

use thiserror::Error;

pub mod dag;
pub mod slots;
pub mod fork_resolution;
pub mod mining;
pub mod errors;
pub mod traits;

pub use dag::DagVertex;
pub use errors::{ConsensusError, ConsensusResult};
pub use slots::SlotScheduler;
pub use traits::ConsensusEngine;

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
