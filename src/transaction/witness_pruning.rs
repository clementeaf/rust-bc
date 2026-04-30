//! Witness pruning for SegWit/PQC blocks.
//!
//! Non-archival nodes can discard witnesses after sufficient confirmations,
//! retaining only the executable `TxCore` data, `tx_root`, and `witness_root`.
//! The `witness_root` serves as a cryptographic commitment that the witnesses
//! existed and were valid at the time of block acceptance.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::transaction::compact_block::{CompactBlockHeader, SegWitBlock};
use crate::transaction::segwit::{compute_tx_root, TxCore};

// ── Pruned Block ─────────────────────────────────────────────────────────

/// A block with witnesses removed. Retains header, tx_cores, and both Merkle
/// roots for historical verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrunedSegWitBlock {
    pub header: CompactBlockHeader,
    pub tx_cores: Vec<TxCore>,
    pub tx_root: [u8; 32],
    /// Preserved as a commitment — witnesses were validated before pruning.
    pub witness_root: [u8; 32],
}

// ── Pruning Logic ────────────────────────────────────────────────────────

/// Prune witnesses from a block if it has enough confirmations.
///
/// Returns `Some(PrunedSegWitBlock)` if `current_height >= block.header.height + pruning_depth`,
/// or `None` if the block is too recent to prune.
pub fn prune_witnesses(
    block: &SegWitBlock,
    current_height: u64,
    pruning_depth: u64,
) -> Option<PrunedSegWitBlock> {
    if current_height < block.header.height.saturating_add(pruning_depth) {
        return None;
    }

    Some(PrunedSegWitBlock {
        header: block.header.clone(),
        tx_cores: block.tx_cores.clone(),
        tx_root: block.tx_root,
        witness_root: block.witness_root,
    })
}

// ── Validation ───────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PrunedValidationError {
    #[error("tx_root mismatch")]
    TxRootMismatch,
    #[error("tx_cores is empty but witness_root is non-zero")]
    InconsistentStructure,
}

/// Validate a pruned block's structural integrity.
///
/// Since witnesses are gone, we can only verify:
/// 1. `tx_root` matches recomputed Merkle root of `tx_cores`
/// 2. `witness_root` is preserved (non-zero if block had transactions)
///
/// This does NOT validate signatures — that was done before pruning.
pub fn validate_pruned_block(block: &PrunedSegWitBlock) -> Result<(), PrunedValidationError> {
    let computed_tx_root = compute_tx_root(&block.tx_cores);
    if computed_tx_root != block.tx_root {
        return Err(PrunedValidationError::TxRootMismatch);
    }

    // If there are tx_cores but witness_root is all zeros, structure is inconsistent
    if !block.tx_cores.is_empty() && block.witness_root == [0u8; 32] {
        return Err(PrunedValidationError::InconsistentStructure);
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
    use crate::transaction::compact_block::CompactBlockHeader;
    use crate::transaction::native::TransactionKind;
    use crate::transaction::segwit::{
        compute_tx_root, compute_witness_root, validate_segwit_block, TxCore, TxWitness,
    };
    use crate::transaction::verification_cache::{
        validate_segwit_block_parallel, VerificationCache,
    };

    fn make_test_block(n: usize, height: u64) -> SegWitBlock {
        let providers: Vec<SoftwareSigningProvider> = (0..n)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let core = TxCore {
                    from: format!("s{i}"),
                    to: format!("r{i}"),
                    amount: 100 + i as u64,
                    fee: 5,
                    nonce: 0,
                    chain_id: 1,
                    timestamp: 1000,
                    kind: Some(TransactionKind::Transfer {
                        from: format!("s{i}"),
                        to: format!("r{i}"),
                        amount: 100 + i as u64,
                    }),
                };
                let payload = core.signing_payload();
                let sig = p.sign(&payload).unwrap();
                let witness = TxWitness {
                    signature: sig,
                    public_key: p.public_key(),
                    signature_scheme: p.algorithm(),
                };
                (core, witness)
            })
            .unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        SegWitBlock {
            header: CompactBlockHeader {
                height,
                hash: tx_root,
                parent_hash: [0u8; 32],
                timestamp: 1000,
                proposer: "validator".into(),
            },
            tx_cores: cores,
            witnesses,
            tx_root,
            witness_root,
        }
    }

    // ── 1. Does not prune before depth ───────────────────────────────────

    #[test]
    fn no_prune_before_depth() {
        let block = make_test_block(3, 10);
        // current_height=15, depth=10 → need height >= 10+10=20
        assert!(prune_witnesses(&block, 15, 10).is_none());
        assert!(prune_witnesses(&block, 19, 10).is_none());
    }

    // ── 2. Prunes after depth ────────────────────────────────────────────

    #[test]
    fn prunes_after_depth() {
        let block = make_test_block(3, 10);
        // current_height=20, depth=10 → 20 >= 10+10 → prune
        let pruned = prune_witnesses(&block, 20, 10).unwrap();
        assert!(pruned.tx_cores.len() == 3);
        // Also at exactly the boundary
        let pruned2 = prune_witnesses(&block, 25, 10).unwrap();
        assert_eq!(pruned, pruned2);
    }

    // ── 3. tx_cores are preserved ────────────────────────────────────────

    #[test]
    fn tx_cores_preserved() {
        let block = make_test_block(5, 1);
        let pruned = prune_witnesses(&block, 100, 10).unwrap();
        assert_eq!(pruned.tx_cores, block.tx_cores);
    }

    // ── 4. witness_root is preserved ─────────────────────────────────────

    #[test]
    fn witness_root_preserved() {
        let block = make_test_block(5, 1);
        let pruned = prune_witnesses(&block, 100, 10).unwrap();
        assert_eq!(pruned.witness_root, block.witness_root);
        assert_ne!(pruned.witness_root, [0u8; 32]);
    }

    // ── 5. Pruned block cannot pass as full block ────────────────────────

    #[test]
    fn pruned_cannot_pass_as_full() {
        let block = make_test_block(3, 1);
        let pruned = prune_witnesses(&block, 100, 10).unwrap();

        // Try to validate with empty witnesses — must fail
        let result = validate_segwit_block(
            &pruned.tx_cores,
            &[], // no witnesses
            &pruned.tx_root,
            &pruned.witness_root,
        );
        assert!(result.is_err());

        // Also fails with parallel validator
        let mut cache = VerificationCache::new(100);
        let result = validate_segwit_block_parallel(
            &pruned.tx_cores,
            &[],
            &pruned.tx_root,
            &pruned.witness_root,
            &mut cache,
        );
        assert!(result.is_err());
    }

    // ── 6. Full block still validates normally ───────────────────────────

    #[test]
    fn full_block_still_validates() {
        let block = make_test_block(5, 1);
        let mut cache = VerificationCache::new(100);
        let result = validate_segwit_block_parallel(
            &block.tx_cores,
            &block.witnesses,
            &block.tx_root,
            &block.witness_root,
            &mut cache,
        );
        assert!(result.is_ok());
    }

    // ── 7. Pruning does not alter tx_root ────────────────────────────────

    #[test]
    fn pruning_does_not_alter_tx_root() {
        let block = make_test_block(5, 1);
        let pruned = prune_witnesses(&block, 100, 10).unwrap();

        assert_eq!(pruned.tx_root, block.tx_root);
        // Recompute to be sure
        let recomputed = compute_tx_root(&pruned.tx_cores);
        assert_eq!(recomputed, block.tx_root);

        // Pruned block passes structural validation
        assert!(validate_pruned_block(&pruned).is_ok());
    }
}
