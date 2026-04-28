//! Equivocation detection for block proposals.
//!
//! Detects when a proposer signs two different valid blocks for the same
//! consensus position (height, slot). This is a Byzantine fault — not
//! cryptographic forgery — and must be handled by penalizing the proposer.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::identity::signing::SigningAlgorithm;

/// Uniquely identifies a consensus position for equivocation detection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsensusPosition {
    pub height: u64,
    pub slot: u64,
    pub proposer: String,
}

/// Cryptographic proof that a proposer equivocated: two different valid
/// signatures over different block hashes for the same consensus position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivocationProof {
    pub position: ConsensusPosition,
    pub block_hash_a: [u8; 32],
    pub block_hash_b: [u8; 32],
    pub signature_a: Vec<u8>,
    pub signature_b: Vec<u8>,
    pub algorithm: SigningAlgorithm,
}

impl EquivocationProof {
    /// Validate that the proof contains two distinct block hashes.
    pub fn is_valid(&self) -> bool {
        self.block_hash_a != self.block_hash_b
            && !self.signature_a.is_empty()
            && !self.signature_b.is_empty()
    }
}

/// Tracks proposals per consensus position and detects equivocation.
#[derive(Default)]
pub struct EquivocationDetector {
    /// Maps (proposer, height, slot) → (block_hash, signature).
    seen: HashMap<ConsensusPosition, ([u8; 32], Vec<u8>)>,
    /// Collected equivocation proofs.
    proofs: Vec<EquivocationProof>,
    /// Set of proof hashes for deduplication.
    proof_hashes: HashSet<([u8; 32], [u8; 32])>,
    /// Penalized proposers.
    penalized: HashSet<String>,
}

impl EquivocationDetector {
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
            proofs: Vec::new(),
            proof_hashes: HashSet::new(),
            penalized: HashSet::new(),
        }
    }

    /// Check a proposal for equivocation.
    ///
    /// Returns `Some(proof)` if this is the second different block from the
    /// same proposer at the same position. Returns `None` if this is the
    /// first proposal or a duplicate of an already-seen block.
    pub fn check_proposal(
        &mut self,
        height: u64,
        slot: u64,
        proposer: &str,
        block_hash: [u8; 32],
        signature: &[u8],
        algorithm: SigningAlgorithm,
    ) -> Option<EquivocationProof> {
        let position = ConsensusPosition {
            height,
            slot,
            proposer: proposer.to_string(),
        };

        if let Some((existing_hash, existing_sig)) = self.seen.get(&position) {
            if *existing_hash == block_hash {
                // Same block — duplicate delivery, not equivocation.
                return None;
            }

            // Different block for same position = EQUIVOCATION
            let proof = EquivocationProof {
                position: position.clone(),
                block_hash_a: *existing_hash,
                block_hash_b: block_hash,
                signature_a: existing_sig.clone(),
                signature_b: signature.to_vec(),
                algorithm,
            };

            // Deduplicate proofs
            let proof_key = if proof.block_hash_a < proof.block_hash_b {
                (proof.block_hash_a, proof.block_hash_b)
            } else {
                (proof.block_hash_b, proof.block_hash_a)
            };

            if self.proof_hashes.insert(proof_key) {
                self.proofs.push(proof.clone());
                self.penalized.insert(proposer.to_string());
            }

            Some(proof)
        } else {
            // First proposal at this position.
            self.seen.insert(position, (block_hash, signature.to_vec()));
            None
        }
    }

    /// Check if a proposer is penalized (quarantined).
    pub fn is_penalized(&self, proposer: &str) -> bool {
        self.penalized.contains(proposer)
    }

    /// Return the number of equivocation proofs for a given proposer.
    pub fn proof_count_for(&self, proposer: &str) -> usize {
        self.proofs
            .iter()
            .filter(|p| p.position.proposer == proposer)
            .count()
    }

    /// Return all proofs.
    pub fn proofs(&self) -> &[EquivocationProof] {
        &self.proofs
    }

    /// Accept an externally received equivocation proof (e.g. via gossip).
    /// Returns `true` if the proof was new, `false` if it was a duplicate.
    pub fn receive_proof(&mut self, proof: &EquivocationProof) -> bool {
        if !proof.is_valid() {
            return false;
        }

        let proof_key = if proof.block_hash_a < proof.block_hash_b {
            (proof.block_hash_a, proof.block_hash_b)
        } else {
            (proof.block_hash_b, proof.block_hash_a)
        };

        if self.proof_hashes.insert(proof_key) {
            self.proofs.push(proof.clone());
            self.penalized.insert(proof.position.proposer.clone());
            true
        } else {
            false
        }
    }

    /// Total number of penalized proposers.
    pub fn penalized_count(&self) -> usize {
        self.penalized.len()
    }

    // ── Persistence ─────────────────────────────────────────────────

    /// Serialize the current state to JSON bytes for persistence.
    pub fn to_bytes(&self) -> Vec<u8> {
        let state = EquivocationPersistState {
            proofs: self.proofs.clone(),
            penalized: self.penalized.iter().cloned().collect(),
        };
        serde_json::to_vec(&state).unwrap_or_default()
    }

    /// Restore state from persisted JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let state: EquivocationPersistState = serde_json::from_slice(data).ok()?;
        let mut detector = Self::new();
        for proof in &state.proofs {
            detector.receive_proof(proof);
        }
        for p in state.penalized {
            detector.penalized.insert(p);
        }
        Some(detector)
    }
}

/// Serializable snapshot of equivocation detector state.
#[derive(Serialize, Deserialize)]
struct EquivocationPersistState {
    proofs: Vec<EquivocationProof>,
    penalized: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_proposal_returns_none() {
        let mut det = EquivocationDetector::new();
        let result = det.check_proposal(
            0,
            0,
            "alice",
            [1u8; 32],
            &[42u8; 64],
            SigningAlgorithm::Ed25519,
        );
        assert!(result.is_none());
    }

    #[test]
    fn same_block_twice_returns_none() {
        let mut det = EquivocationDetector::new();
        det.check_proposal(
            0,
            0,
            "alice",
            [1u8; 32],
            &[42u8; 64],
            SigningAlgorithm::Ed25519,
        );
        let result = det.check_proposal(
            0,
            0,
            "alice",
            [1u8; 32],
            &[42u8; 64],
            SigningAlgorithm::Ed25519,
        );
        assert!(result.is_none(), "duplicate is not equivocation");
    }

    #[test]
    fn different_block_same_position_is_equivocation() {
        let mut det = EquivocationDetector::new();
        det.check_proposal(
            0,
            0,
            "alice",
            [1u8; 32],
            &[42u8; 64],
            SigningAlgorithm::MlDsa65,
        );
        let result = det.check_proposal(
            0,
            0,
            "alice",
            [2u8; 32],
            &[43u8; 64],
            SigningAlgorithm::MlDsa65,
        );
        assert!(result.is_some(), "conflicting block must be equivocation");
        let proof = result.unwrap();
        assert!(proof.is_valid());
        assert_eq!(proof.position.proposer, "alice");
    }

    #[test]
    fn different_proposers_same_height_not_equivocation() {
        let mut det = EquivocationDetector::new();
        det.check_proposal(
            0,
            0,
            "alice",
            [1u8; 32],
            &[42u8; 64],
            SigningAlgorithm::MlDsa65,
        );
        let result = det.check_proposal(
            0,
            0,
            "bob",
            [2u8; 32],
            &[43u8; 64],
            SigningAlgorithm::MlDsa65,
        );
        assert!(result.is_none(), "different proposers is NOT equivocation");
    }

    #[test]
    fn penalized_after_equivocation() {
        let mut det = EquivocationDetector::new();
        det.check_proposal(
            0,
            0,
            "alice",
            [1u8; 32],
            &[42u8; 64],
            SigningAlgorithm::MlDsa65,
        );
        det.check_proposal(
            0,
            0,
            "alice",
            [2u8; 32],
            &[43u8; 64],
            SigningAlgorithm::MlDsa65,
        );
        assert!(det.is_penalized("alice"));
        assert!(!det.is_penalized("bob"));
    }

    #[test]
    fn receive_proof_deduplicates() {
        let mut det = EquivocationDetector::new();
        let proof = EquivocationProof {
            position: ConsensusPosition {
                height: 0,
                slot: 0,
                proposer: "alice".to_string(),
            },
            block_hash_a: [1u8; 32],
            block_hash_b: [2u8; 32],
            signature_a: vec![42u8; 64],
            signature_b: vec![43u8; 64],
            algorithm: SigningAlgorithm::MlDsa65,
        };
        assert!(det.receive_proof(&proof), "first receive should be new");
        assert!(!det.receive_proof(&proof), "second receive should be dedup");
        assert_eq!(det.proofs().len(), 1);
    }
}
