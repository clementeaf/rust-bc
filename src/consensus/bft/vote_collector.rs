//! Vote accumulator for BFT rounds.
//!
//! Collects individual [`VoteMessage`]s for a specific `(phase, round, block_hash)`
//! and signals when quorum (`2f + 1`) is reached, producing a [`QuorumCertificate`].

use std::collections::HashMap;

use super::quorum::{QuorumValidator, SignatureVerifier};
use super::types::{BftPhase, QcError, QuorumCertificate, VoteMessage};

/// Result of adding a vote to the collector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoteResult {
    /// Vote accepted, quorum not yet reached. `count` = valid votes so far.
    Pending { count: usize },
    /// Vote accepted and quorum reached — returns the formed QC.
    QuorumReached { qc: QuorumCertificate },
    /// Vote was rejected (reason in the error).
    Rejected { reason: QcError },
}

/// Accumulates votes for a single `(phase, round, block_hash)` tuple.
///
/// Thread-safe: intended to be used behind external synchronization
/// (e.g. `Mutex<BftRound>` owns the collector).
pub struct VoteCollector<V: SignatureVerifier> {
    phase: BftPhase,
    round: u64,
    block_hash: [u8; 32],
    /// Accepted votes keyed by voter_id to prevent duplicates.
    votes: HashMap<String, VoteMessage>,
    /// Quorum validator for signature and membership checks.
    quorum_validator: QuorumValidator<V>,
    /// Cached: whether quorum was already reached.
    quorum_reached: bool,
}

impl<V: SignatureVerifier> VoteCollector<V> {
    /// Create a new collector for a specific `(phase, round, block_hash)`.
    pub fn new(
        phase: BftPhase,
        round: u64,
        block_hash: [u8; 32],
        quorum_validator: QuorumValidator<V>,
    ) -> Self {
        Self {
            phase,
            round,
            block_hash,
            votes: HashMap::new(),
            quorum_validator,
            quorum_reached: false,
        }
    }

    /// Add a vote. Returns the result of the operation.
    ///
    /// Checks:
    /// 1. Vote matches this collector's `(phase, round, block_hash)`
    /// 2. Voter is not a duplicate
    /// 3. Voter is a known validator with a valid signature
    /// 4. Whether quorum is now reached
    pub fn add_vote(&mut self, vote: VoteMessage) -> VoteResult {
        // Already finalized — ignore further votes.
        if self.quorum_reached {
            return VoteResult::Rejected {
                reason: QcError::DuplicateVoter("quorum already reached".into()),
            };
        }

        // Phase mismatch.
        if vote.phase != self.phase {
            return VoteResult::Rejected {
                reason: QcError::MismatchedPhase {
                    expected: self.phase,
                    got: vote.phase,
                },
            };
        }

        // Round mismatch.
        if vote.round != self.round {
            return VoteResult::Rejected {
                reason: QcError::MismatchedRound {
                    expected: self.round,
                    got: vote.round,
                },
            };
        }

        // Block hash mismatch.
        if vote.block_hash != self.block_hash {
            return VoteResult::Rejected {
                reason: QcError::MismatchedBlockHash {
                    expected: self.block_hash,
                    got: vote.block_hash,
                },
            };
        }

        // Duplicate voter.
        if self.votes.contains_key(&vote.voter_id) {
            return VoteResult::Rejected {
                reason: QcError::DuplicateVoter(vote.voter_id.clone()),
            };
        }

        // Validate membership + signature.
        if let Err(e) = self.quorum_validator.validate_vote(&vote) {
            return VoteResult::Rejected { reason: e };
        }

        // Accept the vote.
        let voter_id = vote.voter_id.clone();
        self.votes.insert(voter_id, vote);

        // Check quorum.
        let count = self.votes.len();
        let threshold = self.quorum_validator.quorum_threshold();

        if count >= threshold {
            self.quorum_reached = true;
            let votes: Vec<VoteMessage> = self.votes.values().cloned().collect();
            match QuorumCertificate::new(self.phase, self.block_hash, self.round, votes) {
                Ok(qc) => VoteResult::QuorumReached { qc },
                Err(e) => VoteResult::Rejected { reason: e },
            }
        } else {
            VoteResult::Pending { count }
        }
    }

    /// Current number of accepted votes.
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Whether quorum has been reached.
    pub fn is_complete(&self) -> bool {
        self.quorum_reached
    }

    /// The quorum threshold for this collector.
    pub fn threshold(&self) -> usize {
        self.quorum_validator.quorum_threshold()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::bft::quorum::AcceptAllVerifier;

    fn validators(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("v{i}")).collect()
    }

    fn block_hash(id: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    fn make_vote(phase: BftPhase, hash_id: u8, round: u64, voter: &str) -> VoteMessage {
        VoteMessage {
            block_hash: block_hash(hash_id),
            round,
            phase,
            voter_id: voter.to_string(),
            signature: vec![1u8; 64],
        }
    }

    fn collector_4(phase: BftPhase, hash_id: u8, round: u64) -> VoteCollector<AcceptAllVerifier> {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        VoteCollector::new(phase, round, block_hash(hash_id), qv)
    }

    // --- basic accumulation ---

    #[test]
    fn first_vote_returns_pending() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v0"));
        assert!(matches!(result, VoteResult::Pending { count: 1 }));
        assert_eq!(c.vote_count(), 1);
    }

    #[test]
    fn second_vote_returns_pending() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v0"));
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v1"));
        assert!(matches!(result, VoteResult::Pending { count: 2 }));
    }

    #[test]
    fn third_vote_reaches_quorum() {
        // n=4, threshold=3
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v0"));
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v1"));
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v2"));
        match result {
            VoteResult::QuorumReached { qc } => {
                assert_eq!(qc.phase, BftPhase::Prepare);
                assert_eq!(qc.round, 0);
                assert_eq!(qc.block_hash, block_hash(1));
                assert_eq!(qc.voter_count(), 3);
            }
            other => panic!("expected QuorumReached, got {other:?}"),
        }
        assert!(c.is_complete());
    }

    // --- rejection cases ---

    #[test]
    fn rejects_duplicate_voter() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v0"));
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v0"));
        assert!(matches!(result, VoteResult::Rejected { reason: QcError::DuplicateVoter(_) }));
        assert_eq!(c.vote_count(), 1);
    }

    #[test]
    fn rejects_wrong_phase() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        let result = c.add_vote(make_vote(BftPhase::Commit, 1, 0, "v0"));
        assert!(matches!(
            result,
            VoteResult::Rejected { reason: QcError::MismatchedPhase { .. } }
        ));
    }

    #[test]
    fn rejects_wrong_round() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 99, "v0"));
        assert!(matches!(
            result,
            VoteResult::Rejected { reason: QcError::MismatchedRound { .. } }
        ));
    }

    #[test]
    fn rejects_wrong_block_hash() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        let result = c.add_vote(make_vote(BftPhase::Prepare, 99, 0, "v0"));
        assert!(matches!(
            result,
            VoteResult::Rejected { reason: QcError::MismatchedBlockHash { .. } }
        ));
    }

    #[test]
    fn rejects_unknown_voter() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "intruder"));
        assert!(matches!(
            result,
            VoteResult::Rejected { reason: QcError::UnknownVoter(_) }
        ));
    }

    // --- post-quorum behavior ---

    #[test]
    fn rejects_votes_after_quorum() {
        let mut c = collector_4(BftPhase::Prepare, 1, 0);
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v0"));
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v1"));
        c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v2"));
        assert!(c.is_complete());

        // Fourth vote should be rejected.
        let result = c.add_vote(make_vote(BftPhase::Prepare, 1, 0, "v3"));
        assert!(matches!(result, VoteResult::Rejected { .. }));
    }

    // --- threshold accessor ---

    #[test]
    fn threshold_matches_quorum_validator() {
        let c = collector_4(BftPhase::Prepare, 1, 0);
        assert_eq!(c.threshold(), 3); // n=4, f=1, 2f+1=3
    }

    // --- different phases produce different collectors ---

    #[test]
    fn precommit_collector_works() {
        let mut c = collector_4(BftPhase::PreCommit, 1, 0);
        c.add_vote(make_vote(BftPhase::PreCommit, 1, 0, "v0"));
        c.add_vote(make_vote(BftPhase::PreCommit, 1, 0, "v1"));
        let result = c.add_vote(make_vote(BftPhase::PreCommit, 1, 0, "v2"));
        match result {
            VoteResult::QuorumReached { qc } => {
                assert_eq!(qc.phase, BftPhase::PreCommit);
            }
            other => panic!("expected QuorumReached, got {other:?}"),
        }
    }

    #[test]
    fn commit_collector_works() {
        let mut c = collector_4(BftPhase::Commit, 1, 5);
        c.add_vote(make_vote(BftPhase::Commit, 1, 5, "v0"));
        c.add_vote(make_vote(BftPhase::Commit, 1, 5, "v1"));
        let result = c.add_vote(make_vote(BftPhase::Commit, 1, 5, "v2"));
        match result {
            VoteResult::QuorumReached { qc } => {
                assert_eq!(qc.phase, BftPhase::Commit);
                assert_eq!(qc.round, 5);
            }
            other => panic!("expected QuorumReached, got {other:?}"),
        }
    }
}
