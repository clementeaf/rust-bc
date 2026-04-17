//! Light client — verifies state proofs against tracked headers without
//! downloading full blocks or running the chaincode executor.
//!
//! Usage:
//! 1. Sync headers from a full node (or peers)
//! 2. Request a state proof for a key at a specific height
//! 3. Verify the proof against the synced header's state_root

use crate::bridge::verifier;
use crate::consensus::bft::quorum::{QuorumValidator, SignatureVerifier};
use crate::consensus::bft::types::BftPhase;

use super::header::{BlockHeader, HeaderChain, HeaderError};

/// A state proof for a single key-value pair.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateProof {
    /// The key being proven.
    pub key: String,
    /// The value (empty if key does not exist).
    pub value: Vec<u8>,
    /// Whether the key exists in state.
    pub exists: bool,
    /// Block height this proof is against.
    pub height: u64,
    /// Merkle inclusion proof against the block's state_root.
    pub proof: crate::bridge::types::InclusionProof,
}

/// Light client errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LightClientError {
    #[error("header error: {0}")]
    Header(#[from] HeaderError),
    #[error("no header at height {0}")]
    HeightNotFound(u64),
    #[error("state proof verification failed for key '{0}'")]
    ProofFailed(String),
    #[error("BFT verification failed: {0}")]
    BftFailed(String),
    #[error("header has no commit QC at height {0}")]
    MissingQc(u64),
}

/// A light client that tracks headers and verifies state proofs.
pub struct LightClient<V: SignatureVerifier + Clone> {
    chain: HeaderChain,
    /// BFT quorum validator for verifying commit QCs on headers.
    quorum_validator: Option<QuorumValidator<V>>,
}

impl<V: SignatureVerifier + Clone> Default for LightClient<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: SignatureVerifier + Clone> LightClient<V> {
    /// Create a new light client.
    pub fn new() -> Self {
        Self {
            chain: HeaderChain::new(),
            quorum_validator: None,
        }
    }

    /// Create a light client with BFT header verification.
    pub fn with_bft(validators: Vec<String>, verifier: V) -> Self {
        Self {
            chain: HeaderChain::new(),
            quorum_validator: Some(QuorumValidator::new(validators, verifier)),
        }
    }

    /// Sync a header from a full node.
    ///
    /// Validates hash integrity, parent linkage, and (if BFT enabled)
    /// the commit QC on non-genesis headers.
    pub fn sync_header(&mut self, header: BlockHeader) -> Result<(), LightClientError> {
        // BFT verification: non-genesis headers must have a valid CommitQC.
        if header.height > 0 {
            if let Some(ref qv) = self.quorum_validator {
                let qc = header
                    .commit_qc
                    .as_ref()
                    .ok_or(LightClientError::MissingQc(header.height))?;

                if qc.phase != BftPhase::Commit {
                    return Err(LightClientError::BftFailed(format!(
                        "expected Commit QC, got {:?}",
                        qc.phase
                    )));
                }

                if qc.block_hash != header.hash {
                    return Err(LightClientError::BftFailed(
                        "QC block_hash does not match header hash".into(),
                    ));
                }

                qv.validate_qc(qc)
                    .map_err(|e| LightClientError::BftFailed(e.to_string()))?;
            }
        }

        self.chain.append(header)?;
        Ok(())
    }

    /// Verify a state proof against a synced header.
    ///
    /// Checks that the Merkle proof roots to the state_root of the
    /// header at the given height.
    pub fn verify_state_proof(&self, proof: &StateProof) -> Result<bool, LightClientError> {
        let header = self
            .chain
            .get(proof.height)
            .ok_or(LightClientError::HeightNotFound(proof.height))?;

        // The proof's root must match the header's state_root.
        if proof.proof.root != header.state_root {
            return Err(LightClientError::ProofFailed(format!(
                "proof root does not match state_root at height {}",
                proof.height
            )));
        }

        // Verify the Merkle proof.
        let leaf_data = if proof.exists {
            // Leaf = key || value
            let mut data = proof.key.as_bytes().to_vec();
            data.extend_from_slice(&proof.value);
            data
        } else {
            // Non-existence proof: leaf is just the key.
            proof.key.as_bytes().to_vec()
        };

        let valid = verifier::verify_merkle_proof(&leaf_data, &proof.proof);
        if !valid {
            return Err(LightClientError::ProofFailed(proof.key.clone()));
        }

        Ok(proof.exists)
    }

    /// Current synced height.
    pub fn synced_height(&self) -> Option<u64> {
        self.chain.height()
    }

    /// Number of synced headers.
    pub fn header_count(&self) -> usize {
        self.chain.len()
    }

    /// Get a synced header by height.
    pub fn get_header(&self, height: u64) -> Option<&BlockHeader> {
        self.chain.get(height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::types::InclusionProof;
    use crate::bridge::verifier::build_merkle_tree;
    use crate::consensus::bft::types::{BftPhase, QuorumCertificate, VoteMessage};

    #[derive(Clone)]
    struct TestVerifier;
    impl SignatureVerifier for TestVerifier {
        fn verify(&self, _: &str, _: &[u8], sig: &[u8]) -> bool {
            !sig.is_empty()
        }
    }

    fn genesis() -> BlockHeader {
        let mut h = BlockHeader {
            height: 0,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            tx_merkle_root: [1u8; 32],
            state_root: [2u8; 32],
            timestamp: 1000,
            proposer: "v0".into(),
            tx_count: 0,
            commit_qc: None,
        };
        h.hash = h.compute_hash();
        h
    }

    fn make_qc(block_hash: [u8; 32]) -> QuorumCertificate {
        let votes: Vec<VoteMessage> = (0..3)
            .map(|i| VoteMessage {
                block_hash,
                round: 0,
                phase: BftPhase::Commit,
                voter_id: format!("v{i}"),
                signature: vec![1u8; 64],
            })
            .collect();
        QuorumCertificate::new(BftPhase::Commit, block_hash, 0, votes).unwrap()
    }

    fn child_with_qc(parent: &BlockHeader, height: u64, state_root: [u8; 32]) -> BlockHeader {
        let mut h = BlockHeader {
            height,
            hash: [0u8; 32],
            parent_hash: parent.hash,
            tx_merkle_root: [height as u8; 32],
            state_root,
            timestamp: 1000 + height * 15,
            proposer: format!("v{}", height % 4),
            tx_count: 1,
            commit_qc: None,
        };
        h.hash = h.compute_hash();
        h.commit_qc = Some(make_qc(h.hash));
        h
    }

    fn validators() -> Vec<String> {
        (0..4).map(|i| format!("v{i}")).collect()
    }

    // --- basic sync ---

    #[test]
    fn sync_genesis_no_bft() {
        let mut lc: LightClient<TestVerifier> = LightClient::new();
        lc.sync_header(genesis()).unwrap();
        assert_eq!(lc.synced_height(), Some(0));
    }

    #[test]
    fn sync_chain_no_bft() {
        let mut lc: LightClient<TestVerifier> = LightClient::new();
        let g = genesis();
        lc.sync_header(g.clone()).unwrap();

        let c1 = child_with_qc(&g, 1, [10u8; 32]);
        lc.sync_header(c1).unwrap();
        assert_eq!(lc.synced_height(), Some(1));
        assert_eq!(lc.header_count(), 2);
    }

    // --- BFT verification ---

    #[test]
    fn sync_with_bft_accepts_valid_qc() {
        let mut lc = LightClient::with_bft(validators(), TestVerifier);
        lc.sync_header(genesis()).unwrap();

        let c = child_with_qc(&genesis(), 1, [10u8; 32]);
        lc.sync_header(c).unwrap();
        assert_eq!(lc.synced_height(), Some(1));
    }

    #[test]
    fn sync_with_bft_rejects_missing_qc() {
        let mut lc = LightClient::with_bft(validators(), TestVerifier);
        lc.sync_header(genesis()).unwrap();

        let mut c = child_with_qc(&genesis(), 1, [10u8; 32]);
        c.commit_qc = None; // Remove QC.
        let err = lc.sync_header(c).unwrap_err();
        assert!(matches!(err, LightClientError::MissingQc(1)));
    }

    #[test]
    fn sync_with_bft_rejects_wrong_qc_hash() {
        let mut lc = LightClient::with_bft(validators(), TestVerifier);
        lc.sync_header(genesis()).unwrap();

        let mut c = child_with_qc(&genesis(), 1, [10u8; 32]);
        // QC for a different block hash.
        c.commit_qc = Some(make_qc([0xFF; 32]));
        let err = lc.sync_header(c).unwrap_err();
        assert!(matches!(err, LightClientError::BftFailed(_)));
    }

    // --- state proof verification ---

    #[test]
    fn verify_valid_state_proof() {
        let mut lc: LightClient<TestVerifier> = LightClient::new();

        // Build a state with one key-value pair.
        let key = "balance:alice";
        let value = b"1000";
        let leaf_data = [key.as_bytes(), value.as_slice()].concat();

        let (root, proofs) = build_merkle_tree(&[&leaf_data]);
        let state_root = root.unwrap();

        // Create genesis with this state_root.
        let mut g = BlockHeader {
            height: 0,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            state_root,
            timestamp: 1000,
            proposer: "v0".into(),
            tx_count: 0,
            commit_qc: None,
        };
        g.hash = g.compute_hash();
        lc.sync_header(g).unwrap();

        let state_proof = StateProof {
            key: key.to_string(),
            value: value.to_vec(),
            exists: true,
            height: 0,
            proof: proofs[0].clone(),
        };

        let exists = lc.verify_state_proof(&state_proof).unwrap();
        assert!(exists);
    }

    #[test]
    fn verify_invalid_state_proof() {
        let mut lc: LightClient<TestVerifier> = LightClient::new();

        let leaf_data = b"balance:alice1000";
        let (root, proofs) = build_merkle_tree(&[leaf_data.as_slice()]);

        let mut g = BlockHeader {
            height: 0,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            tx_merkle_root: [0u8; 32],
            state_root: root.unwrap(),
            timestamp: 1000,
            proposer: "v0".into(),
            tx_count: 0,
            commit_qc: None,
        };
        g.hash = g.compute_hash();
        lc.sync_header(g).unwrap();

        // Proof for wrong value.
        let state_proof = StateProof {
            key: "balance:alice".to_string(),
            value: b"9999".to_vec(), // Wrong value.
            exists: true,
            height: 0,
            proof: proofs[0].clone(),
        };

        let err = lc.verify_state_proof(&state_proof).unwrap_err();
        assert!(matches!(err, LightClientError::ProofFailed(_)));
    }

    #[test]
    fn verify_proof_wrong_height() {
        let lc: LightClient<TestVerifier> = LightClient::new();
        let proof = StateProof {
            key: "k".into(),
            value: vec![],
            exists: false,
            height: 99,
            proof: InclusionProof {
                merkle_path: vec![],
                leaf_index: 0,
                root: [0u8; 32],
                block_hash: [0u8; 32],
                block_height: 99,
            },
        };
        let err = lc.verify_state_proof(&proof).unwrap_err();
        assert!(matches!(err, LightClientError::HeightNotFound(99)));
    }

    #[test]
    fn verify_proof_root_mismatch() {
        let mut lc: LightClient<TestVerifier> = LightClient::new();
        lc.sync_header(genesis()).unwrap();

        let proof = StateProof {
            key: "k".into(),
            value: vec![],
            exists: false,
            height: 0,
            proof: InclusionProof {
                merkle_path: vec![],
                leaf_index: 0,
                root: [0xFF; 32], // Doesn't match genesis state_root.
                block_hash: [0u8; 32],
                block_height: 0,
            },
        };
        let err = lc.verify_state_proof(&proof).unwrap_err();
        assert!(matches!(err, LightClientError::ProofFailed(_)));
    }

    // --- stress ---

    #[test]
    fn sync_100_headers() {
        let mut lc: LightClient<TestVerifier> = LightClient::new();
        let g = genesis();
        lc.sync_header(g.clone()).unwrap();

        let mut parent = g;
        for h in 1..100 {
            let c = child_with_qc(&parent, h, [h as u8; 32]);
            lc.sync_header(c.clone()).unwrap();
            parent = c;
        }
        assert_eq!(lc.header_count(), 100);
        assert_eq!(lc.synced_height(), Some(99));
    }
}
