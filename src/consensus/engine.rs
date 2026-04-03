//! Consensus engine — single entry point for the DAG consensus layer.
//!
//! `ConsensusEngine` ties together the DAG, fork-choice rule, and slot
//! scheduler.  It is the only type callers need to interact with.

use crate::consensus::{
    dag::{Dag, DagBlock},
    fork_choice::{ForkChoice, ForkChoiceRule},
    scheduler::SlotScheduler,
    validator::{BlockValidator, ValidityResult},
    ConsensusConfig,
};

/// Errors returned by the consensus engine.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConsensusError {
    #[error("invalid block: {0}")]
    InvalidBlock(String),
    #[error("dag error: {0}")]
    DagError(String),
}

/// The consensus engine.
///
/// # Example
/// ```rust
/// use rust_bc::consensus::{ConsensusEngine, ConsensusConfig, ForkChoiceRule};
///
/// let validators = vec!["alice".to_string()];
/// let engine = ConsensusEngine::new(
///     ConsensusConfig::default(),
///     ForkChoiceRule::HeaviestSubtree,
///     validators,
///     0,   // genesis timestamp
/// );
/// ```
pub struct ConsensusEngine {
    dag: Dag,
    fork_choice: ForkChoice,
    scheduler: SlotScheduler,
}

impl ConsensusEngine {
    /// Create a new `ConsensusEngine`.
    ///
    /// - `config`       — slot duration and parallel-slot limits
    /// - `rule`         — fork-choice strategy
    /// - `validators`   — ordered list of validator identities used for
    ///                    round-robin slot assignment
    /// - `genesis_time` — UNIX timestamp of the first slot's start
    pub fn new(
        config: ConsensusConfig,
        rule: ForkChoiceRule,
        validators: Vec<String>,
        genesis_time: u64,
    ) -> Self {
        let scheduler =
            SlotScheduler::new(config.slot_duration_ms / 1000, validators, genesis_time);
        Self {
            dag: Dag::new(),
            fork_choice: ForkChoice::new(rule),
            scheduler,
        }
    }

    // --- mutations ---

    /// Validate and insert a block into the DAG.
    ///
    /// Runs the full `BlockValidator` pipeline (format → signature → parent →
    /// slot) before inserting.  Returns the block's hash on success.
    pub fn accept_block(&mut self, block: DagBlock) -> Result<[u8; 32], ConsensusError> {
        match BlockValidator::validate(&block, &self.scheduler) {
            ValidityResult::Valid => {}
            ValidityResult::Invalid(reason) => {
                return Err(ConsensusError::InvalidBlock(reason));
            }
        }

        let hash = block.hash;
        self.dag
            .add_block(block)
            .map_err(ConsensusError::DagError)?;

        Ok(hash)
    }

    // --- accessors ---

    /// Return the canonical chain as an ordered list of hashes (genesis → tip).
    pub fn canonical_chain(&self) -> Vec<[u8; 32]> {
        self.fork_choice.canonical_chain(&self.dag)
    }

    /// Return the canonical tip hash, i.e. the last block in the canonical
    /// chain.  Returns `None` when the DAG is empty.
    pub fn canonical_tip(&self) -> Option<[u8; 32]> {
        self.fork_choice.canonical_chain(&self.dag).into_iter().last()
    }

    /// Total number of blocks in the DAG (including stale branches).
    pub fn block_count(&self) -> u64 {
        self.dag.block_count()
    }

    /// Borrow the underlying DAG (read-only).
    pub fn dag(&self) -> &Dag {
        &self.dag
    }

    /// Borrow the fork-choice engine (read-only).
    pub fn fork_choice(&self) -> &ForkChoice {
        &self.fork_choice
    }

    /// Borrow the slot scheduler (read-only).
    pub fn scheduler(&self) -> &SlotScheduler {
        &self.scheduler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // slot_duration_ms=6000 → secs=6; genesis_time=0
    // slot 0 covers [0, 6), proposer = "v1" (round-robin with one validator)
    fn engine() -> ConsensusEngine {
        ConsensusEngine::new(
            ConsensusConfig::default(),
            ForkChoiceRule::HeaviestSubtree,
            vec!["v1".to_string()],
            0,
        )
    }

    fn mk(id: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    /// Build a block that passes full validation:
    /// slot 0, timestamp within [0,6), proposer "v1".
    fn valid_block(hash: u8, parent: u8, height: u64) -> DagBlock {
        DagBlock::new(mk(hash), mk(parent), height, 0, 0, "v1".to_string(), [2u8; 64])
    }

    // --- accept_block: happy path ---

    #[test]
    fn accept_genesis_block() {
        let mut e = engine();
        let result = e.accept_block(valid_block(1, 0, 0));
        assert_eq!(result, Ok(mk(1)));
        assert_eq!(e.block_count(), 1);
    }

    #[test]
    fn accept_chain_of_blocks() {
        let mut e = engine();
        e.accept_block(valid_block(1, 0, 0)).unwrap();
        e.accept_block(valid_block(2, 1, 1)).unwrap();
        assert_eq!(e.block_count(), 2);
    }

    // --- accept_block: rejection cases ---

    #[test]
    fn reject_zero_hash() {
        let mut e = engine();
        let mut b = valid_block(1, 0, 0);
        b.hash = [0u8; 32];
        assert!(matches!(e.accept_block(b), Err(ConsensusError::InvalidBlock(_))));
    }

    #[test]
    fn reject_empty_proposer() {
        let mut e = engine();
        let mut b = valid_block(1, 0, 0);
        b.proposer = String::new();
        assert!(matches!(e.accept_block(b), Err(ConsensusError::InvalidBlock(_))));
    }

    #[test]
    fn reject_zero_signature() {
        let mut e = engine();
        let mut b = valid_block(1, 0, 0);
        b.signature = [0u8; 64];
        assert!(matches!(e.accept_block(b), Err(ConsensusError::InvalidBlock(_))));
    }

    #[test]
    fn reject_wrong_proposer_for_slot() {
        let mut e = engine();
        let mut b = valid_block(1, 0, 0);
        b.proposer = "intruder".to_string();
        assert!(matches!(e.accept_block(b), Err(ConsensusError::InvalidBlock(_))));
    }

    #[test]
    fn reject_duplicate_block() {
        let mut e = engine();
        e.accept_block(valid_block(1, 0, 0)).unwrap();
        assert!(matches!(
            e.accept_block(valid_block(1, 0, 0)),
            Err(ConsensusError::DagError(_))
        ));
    }

    // --- canonical_tip / canonical_chain ---

    #[test]
    fn canonical_tip_empty_dag() {
        let e = engine();
        assert_eq!(e.canonical_tip(), None);
    }

    #[test]
    fn canonical_tip_after_linear_chain() {
        let mut e = engine();
        e.accept_block(valid_block(1, 0, 0)).unwrap();
        e.accept_block(valid_block(2, 1, 1)).unwrap();
        assert_eq!(e.canonical_tip(), Some(mk(2)));
    }

    #[test]
    fn canonical_chain_returns_full_path() {
        let mut e = engine();
        e.accept_block(valid_block(1, 0, 0)).unwrap();
        e.accept_block(valid_block(2, 1, 1)).unwrap();
        assert_eq!(e.canonical_chain(), vec![mk(1), mk(2)]);
    }

    #[test]
    fn canonical_tip_picks_heavier_fork() {
        // genesis=1 → fork: branch A (2→4) heavier, branch B (3) stale
        let mut e = engine();
        e.accept_block(valid_block(1, 0, 0)).unwrap();
        e.accept_block(valid_block(2, 1, 1)).unwrap();
        e.accept_block(valid_block(3, 1, 1)).unwrap();
        e.accept_block(valid_block(4, 2, 2)).unwrap();

        assert_eq!(e.canonical_tip(), Some(mk(4)));
        assert_eq!(e.block_count(), 4);
    }
}
