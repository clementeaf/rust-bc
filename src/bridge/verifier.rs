//! Light client proof verification for cross-chain bridges.
//!
//! Verifies Merkle inclusion proofs submitted by relayers to confirm that
//! a message was included in a source chain block.

use sha2::{Digest, Sha256};

use super::types::InclusionProof;

/// Verify a Merkle inclusion proof.
///
/// Recomputes the root from the leaf and the proof path, then compares
/// against the expected root in the proof.
///
/// The leaf is `SHA-256(data)`. At each level, the sibling is combined
/// with the current hash: if the current index is even, `H(current || sibling)`;
/// if odd, `H(sibling || current)`.
pub fn verify_merkle_proof(data: &[u8], proof: &InclusionProof) -> bool {
    let mut hash = sha256(data);
    let mut index = proof.leaf_index;

    for sibling in &proof.merkle_path {
        hash = if index % 2 == 0 {
            hash_pair(&hash, sibling)
        } else {
            hash_pair(sibling, &hash)
        };
        index /= 2;
    }

    hash == proof.root
}

/// SHA-256 hash of a byte slice.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash two 32-byte nodes: `SHA-256(left || right)`.
fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Build a Merkle tree from leaves and return `(root, proofs)`.
///
/// Each leaf is `SHA-256(data)`. Returns the root hash and a proof for
/// each leaf. Useful for testing and for the bridge to generate proofs
/// for outbound messages.
pub fn build_merkle_tree(leaves: &[&[u8]]) -> (Option<[u8; 32]>, Vec<InclusionProof>) {
    if leaves.is_empty() {
        return (None, vec![]);
    }

    let n = leaves.len().next_power_of_two();
    let mut layer: Vec<[u8; 32]> = leaves.iter().map(|l| sha256(l)).collect();
    // Pad to power of 2 with zero hashes.
    while layer.len() < n {
        layer.push([0u8; 32]);
    }

    // Collect all layers for proof extraction.
    let mut layers: Vec<Vec<[u8; 32]>> = vec![layer.clone()];

    while layer.len() > 1 {
        let mut next = Vec::with_capacity(layer.len() / 2);
        for chunk in layer.chunks(2) {
            next.push(hash_pair(&chunk[0], &chunk[1]));
        }
        layers.push(next.clone());
        layer = next;
    }

    let root = layer[0];

    // Build proofs for each original leaf.
    let mut proofs = Vec::with_capacity(leaves.len());
    for i in 0..leaves.len() {
        let mut merkle_path = Vec::new();
        let mut idx = i;

        for lvl in &layers[..layers.len() - 1] {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            if sibling_idx < lvl.len() {
                merkle_path.push(lvl[sibling_idx]);
            } else {
                merkle_path.push([0u8; 32]);
            }
            idx /= 2;
        }

        proofs.push(InclusionProof {
            merkle_path,
            leaf_index: i as u64,
            root,
            block_hash: [0u8; 32],
            block_height: 0,
        });
    }

    (Some(root), proofs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_leaf_proof() {
        let data = b"hello";
        let (root, proofs) = build_merkle_tree(&[data]);
        assert!(root.is_some());
        assert_eq!(proofs.len(), 1);
        assert!(verify_merkle_proof(data, &proofs[0]));
    }

    #[test]
    fn two_leaves_proof() {
        let (root, proofs) = build_merkle_tree(&[b"a", b"b"]);
        assert!(root.is_some());
        assert_eq!(proofs.len(), 2);
        assert!(verify_merkle_proof(b"a", &proofs[0]));
        assert!(verify_merkle_proof(b"b", &proofs[1]));
    }

    #[test]
    fn four_leaves_proof() {
        let leaves: Vec<&[u8]> = vec![b"w", b"x", b"y", b"z"];
        let (root, proofs) = build_merkle_tree(&leaves);
        assert!(root.is_some());
        assert_eq!(proofs.len(), 4);

        for (i, leaf) in leaves.iter().enumerate() {
            assert!(
                verify_merkle_proof(leaf, &proofs[i]),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn three_leaves_padded_to_four() {
        let leaves: Vec<&[u8]> = vec![b"a", b"b", b"c"];
        let (root, proofs) = build_merkle_tree(&leaves);
        assert!(root.is_some());
        assert_eq!(proofs.len(), 3);

        for (i, leaf) in leaves.iter().enumerate() {
            assert!(
                verify_merkle_proof(leaf, &proofs[i]),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn wrong_data_fails_verification() {
        let (_, proofs) = build_merkle_tree(&[b"correct"]);
        assert!(!verify_merkle_proof(b"wrong", &proofs[0]));
    }

    #[test]
    fn tampered_proof_fails() {
        let (_, mut proofs) = build_merkle_tree(&[b"a", b"b"]);
        // Tamper with the sibling hash.
        proofs[0].merkle_path[0] = [0xFF; 32];
        assert!(!verify_merkle_proof(b"a", &proofs[0]));
    }

    #[test]
    fn tampered_root_fails() {
        let (_, mut proofs) = build_merkle_tree(&[b"a", b"b"]);
        proofs[0].root = [0xFF; 32];
        assert!(!verify_merkle_proof(b"a", &proofs[0]));
    }

    #[test]
    fn empty_tree_returns_none() {
        let (root, proofs) = build_merkle_tree(&[]);
        assert!(root.is_none());
        assert!(proofs.is_empty());
    }

    #[test]
    fn stress_1000_leaves() {
        let data: Vec<Vec<u8>> = (0..1000u32).map(|i| i.to_le_bytes().to_vec()).collect();
        let leaves: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();

        let (root, proofs) = build_merkle_tree(&leaves);
        assert!(root.is_some());
        assert_eq!(proofs.len(), 1000);

        // Verify a sample of proofs (every 100th).
        for i in (0..1000).step_by(100) {
            assert!(
                verify_merkle_proof(&data[i], &proofs[i]),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn proof_deterministic() {
        let (root1, _) = build_merkle_tree(&[b"x", b"y"]);
        let (root2, _) = build_merkle_tree(&[b"x", b"y"]);
        assert_eq!(root1, root2);
    }
}
