//! Block header — lightweight representation of a block for light client sync.
//!
//! Headers contain just enough information to verify state proofs without
//! downloading full block data (transactions, endorsements, etc.).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::consensus::bft::types::QuorumCertificate;

/// Compact block header for light client verification.
///
/// Approximately 200-400 bytes per header (vs ~10 KB+ for a full block),
/// enabling IoT devices with limited storage to track chain progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block height.
    pub height: u64,
    /// SHA-256 hash of this header's canonical fields.
    pub hash: [u8; 32],
    /// Hash of the parent header.
    pub parent_hash: [u8; 32],
    /// Merkle root of all transactions in this block.
    pub tx_merkle_root: [u8; 32],
    /// Merkle root of the world state after applying this block.
    pub state_root: [u8; 32],
    /// Block timestamp (UNIX seconds).
    pub timestamp: u64,
    /// Block proposer identity.
    pub proposer: String,
    /// Number of transactions in this block.
    pub tx_count: u32,
    /// BFT commit QC (proves 2f+1 validators agreed on this block).
    /// `None` for genesis or pre-BFT blocks.
    pub commit_qc: Option<QuorumCertificate>,
}

impl BlockHeader {
    /// Compute the canonical hash of this header.
    ///
    /// Hash = SHA-256(height || parent_hash || tx_merkle_root || state_root || timestamp || proposer)
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.parent_hash);
        hasher.update(self.tx_merkle_root);
        hasher.update(self.state_root);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.proposer.as_bytes());
        hasher.finalize().into()
    }

    /// Verify this header's hash is correct.
    pub fn verify_hash(&self) -> bool {
        self.hash == self.compute_hash()
    }

    /// Verify the parent chain: this header's parent_hash matches the given parent.
    pub fn verify_parent(&self, parent: &BlockHeader) -> bool {
        self.parent_hash == parent.hash && self.height == parent.height + 1
    }
}

/// Header chain — ordered sequence of verified headers.
///
/// Used by light clients to track the chain without full blocks.
/// Headers are stored in height order for O(1) lookup by height.
#[derive(Debug, Clone, Default)]
pub struct HeaderChain {
    headers: Vec<BlockHeader>,
}

impl HeaderChain {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    /// Append a header to the chain.
    ///
    /// Validates:
    /// 1. Hash integrity
    /// 2. Parent linkage (unless genesis)
    /// 3. Monotonic height
    pub fn append(&mut self, header: BlockHeader) -> Result<(), HeaderError> {
        if !header.verify_hash() {
            return Err(HeaderError::InvalidHash {
                height: header.height,
            });
        }

        if let Some(tip) = self.headers.last() {
            if !header.verify_parent(tip) {
                return Err(HeaderError::InvalidParent {
                    height: header.height,
                });
            }
        } else if header.height != 0 {
            return Err(HeaderError::InvalidGenesis);
        }

        self.headers.push(header);
        Ok(())
    }

    /// Get the current chain tip.
    pub fn tip(&self) -> Option<&BlockHeader> {
        self.headers.last()
    }

    /// Get a header by height.
    pub fn get(&self, height: u64) -> Option<&BlockHeader> {
        self.headers.get(height as usize)
    }

    /// Current chain height (0-indexed).
    pub fn height(&self) -> Option<u64> {
        self.tip().map(|h| h.height)
    }

    /// Number of headers stored.
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
}

/// Errors from header operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
    #[error("invalid header hash at height {height}")]
    InvalidHash { height: u64 },
    #[error("invalid parent linkage at height {height}")]
    InvalidParent { height: u64 },
    #[error("first header must be genesis (height 0)")]
    InvalidGenesis,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn child(parent: &BlockHeader, height: u64) -> BlockHeader {
        let mut h = BlockHeader {
            height,
            hash: [0u8; 32],
            parent_hash: parent.hash,
            tx_merkle_root: [height as u8; 32],
            state_root: [(height + 10) as u8; 32],
            timestamp: 1000 + height * 15,
            proposer: format!("v{}", height % 4),
            tx_count: height as u32,
            commit_qc: None,
        };
        h.hash = h.compute_hash();
        h
    }

    // --- BlockHeader ---

    #[test]
    fn compute_hash_deterministic() {
        let g = genesis();
        assert_eq!(g.compute_hash(), g.compute_hash());
    }

    #[test]
    fn verify_hash_valid() {
        let g = genesis();
        assert!(g.verify_hash());
    }

    #[test]
    fn verify_hash_tampered() {
        let mut g = genesis();
        g.timestamp = 9999; // Tamper without recomputing hash.
        assert!(!g.verify_hash());
    }

    #[test]
    fn verify_parent_valid() {
        let g = genesis();
        let c = child(&g, 1);
        assert!(c.verify_parent(&g));
    }

    #[test]
    fn verify_parent_wrong_hash() {
        let g = genesis();
        let mut c = child(&g, 1);
        c.parent_hash = [99u8; 32]; // Wrong parent.
        c.hash = c.compute_hash();
        assert!(!c.verify_parent(&g));
    }

    #[test]
    fn verify_parent_wrong_height() {
        let g = genesis();
        let mut c = child(&g, 1);
        c.height = 5; // Gap in height.
        c.hash = c.compute_hash();
        assert!(!c.verify_parent(&g));
    }

    // --- HeaderChain ---

    #[test]
    fn append_genesis() {
        let mut chain = HeaderChain::new();
        chain.append(genesis()).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.height(), Some(0));
    }

    #[test]
    fn append_chain_of_10() {
        let mut chain = HeaderChain::new();
        let g = genesis();
        chain.append(g.clone()).unwrap();

        let mut parent = g;
        for h in 1..10 {
            let c = child(&parent, h);
            chain.append(c.clone()).unwrap();
            parent = c;
        }
        assert_eq!(chain.len(), 10);
        assert_eq!(chain.height(), Some(9));
    }

    #[test]
    fn append_invalid_hash_rejected() {
        let mut chain = HeaderChain::new();
        let mut g = genesis();
        g.hash = [0xFF; 32]; // Bad hash.
        let err = chain.append(g).unwrap_err();
        assert!(matches!(err, HeaderError::InvalidHash { .. }));
    }

    #[test]
    fn append_invalid_parent_rejected() {
        let mut chain = HeaderChain::new();
        chain.append(genesis()).unwrap();

        let mut bad_child = BlockHeader {
            height: 1,
            hash: [0u8; 32],
            parent_hash: [99u8; 32], // Wrong parent.
            tx_merkle_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp: 2000,
            proposer: "v1".into(),
            tx_count: 0,
            commit_qc: None,
        };
        bad_child.hash = bad_child.compute_hash();

        let err = chain.append(bad_child).unwrap_err();
        assert!(matches!(err, HeaderError::InvalidParent { .. }));
    }

    #[test]
    fn append_non_genesis_first_rejected() {
        let mut chain = HeaderChain::new();
        let g = genesis();
        let c = child(&g, 1);
        let err = chain.append(c).unwrap_err();
        assert!(matches!(err, HeaderError::InvalidGenesis));
    }

    #[test]
    fn get_by_height() {
        let mut chain = HeaderChain::new();
        let g = genesis();
        chain.append(g.clone()).unwrap();
        let c1 = child(&g, 1);
        chain.append(c1.clone()).unwrap();

        assert_eq!(chain.get(0).unwrap().height, 0);
        assert_eq!(chain.get(1).unwrap().height, 1);
        assert!(chain.get(2).is_none());
    }

    #[test]
    fn empty_chain() {
        let chain = HeaderChain::new();
        assert!(chain.is_empty());
        assert!(chain.tip().is_none());
        assert!(chain.height().is_none());
    }

    // --- serde ---

    #[test]
    fn header_serde_roundtrip() {
        let g = genesis();
        let json = serde_json::to_string(&g).unwrap();
        let back: BlockHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
