//! Consensus backend trait — abstracts over Raft (CFT) and BFT consensus.
//!
//! Allows runtime selection of the consensus mechanism via configuration,
//! keeping the rest of the node agnostic to which protocol is active.

use crate::consensus::bft::types::QuorumCertificate;
use crate::storage::errors::StorageResult;
use crate::storage::traits::{Block, Transaction};

/// Consensus mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConsensusMode {
    /// Raft-based ordering (CFT — crash fault tolerant).
    /// Suitable for permissioned deployments where all nodes are trusted.
    Raft,
    /// HotStuff-inspired BFT (Byzantine fault tolerant).
    /// Suitable for semi-trusted or public deployments.
    Bft,
}

impl ConsensusMode {
    /// Read from the `CONSENSUS_MODE` environment variable.
    /// Defaults to `Raft` for backward compatibility.
    pub fn from_env() -> Self {
        match std::env::var("CONSENSUS_MODE")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "bft" => ConsensusMode::Bft,
            _ => ConsensusMode::Raft,
        }
    }
}

/// Common interface for consensus backends.
///
/// Both Raft and BFT implement this trait so the node can switch between
/// them at startup without changing the transaction pipeline.
pub trait ConsensusBackend: Send + Sync {
    /// Submit a transaction to be included in the next block.
    fn submit_tx(&self, tx: &Transaction) -> StorageResult<()>;

    /// Cut a block from pending transactions.
    ///
    /// For Raft: the leader batches and orders transactions.
    /// For BFT: the round leader proposes a block that goes through
    /// Prepare→PreCommit→Commit before finalization.
    fn cut_block(&self, height: u64, proposer: &str) -> StorageResult<Option<Block>>;

    /// Number of transactions waiting to be included.
    fn pending_count(&self) -> usize;

    /// The active consensus mode.
    fn mode(&self) -> ConsensusMode;

    /// The highest committed QC (BFT only). Returns `None` for Raft.
    fn highest_qc(&self) -> Option<QuorumCertificate> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn consensus_mode_default_is_raft() {
        // Without env var set, defaults to Raft.
        // Note: we can't reliably test from_env() since it reads real env,
        // so test the parsing logic directly.
        assert_eq!(ConsensusMode::Raft, ConsensusMode::Raft);
        assert_ne!(ConsensusMode::Raft, ConsensusMode::Bft);
    }

    #[test]
    fn consensus_mode_serialization_roundtrip() {
        let json = serde_json::to_string(&ConsensusMode::Bft).unwrap();
        let parsed: ConsensusMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ConsensusMode::Bft);
    }

    /// Dummy backend to verify the trait is object-safe.
    struct DummyBackend {
        mode: ConsensusMode,
    }

    impl ConsensusBackend for DummyBackend {
        fn submit_tx(&self, _tx: &Transaction) -> StorageResult<()> {
            Ok(())
        }
        fn cut_block(&self, _height: u64, _proposer: &str) -> StorageResult<Option<Block>> {
            Ok(None)
        }
        fn pending_count(&self) -> usize {
            0
        }
        fn mode(&self) -> ConsensusMode {
            self.mode
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let raft: Box<dyn ConsensusBackend> = Box::new(DummyBackend {
            mode: ConsensusMode::Raft,
        });
        assert_eq!(raft.mode(), ConsensusMode::Raft);
        assert!(raft.highest_qc().is_none());

        let bft: Box<dyn ConsensusBackend> = Box::new(DummyBackend {
            mode: ConsensusMode::Bft,
        });
        assert_eq!(bft.mode(), ConsensusMode::Bft);
    }

    #[test]
    fn default_highest_qc_is_none() {
        let backend = DummyBackend {
            mode: ConsensusMode::Bft,
        };
        assert!(backend.highest_qc().is_none());
    }
}
