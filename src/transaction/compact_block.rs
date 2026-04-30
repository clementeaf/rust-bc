//! Compact block propagation for SegWit/PQC blocks.
//!
//! Reduces network bandwidth by replacing full `TxCore` and `TxWitness` objects
//! with 8-byte short IDs. Peers reconstruct full blocks from their mempool,
//! requesting only missing objects. This is a transport optimization — full
//! validation is still required after reconstruction.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::crypto::hasher::{hash_with, HashAlgorithm};
use crate::transaction::segwit::{TxCore, TxWitness};

// ── Short ID ─────────────────────────────────────────────────────────────

/// 8-byte truncated hash used as a compact identifier for transport.
pub type ShortId = [u8; 8];

/// Compute a deterministic short ID for a `TxCore`.
pub fn short_id_tx_core(core: &TxCore) -> ShortId {
    let serialized = serde_json::to_vec(core).expect("TxCore serialization cannot fail");
    let hash = hash_with(HashAlgorithm::Sha3_256, &serialized);
    let mut id = [0u8; 8];
    id.copy_from_slice(&hash[..8]);
    id
}

/// Compute a deterministic short ID for a `TxWitness`.
pub fn short_id_witness(witness: &TxWitness) -> ShortId {
    let serialized = serde_json::to_vec(witness).expect("TxWitness serialization cannot fail");
    let hash = hash_with(HashAlgorithm::Sha3_256, &serialized);
    let mut id = [0u8; 8];
    id.copy_from_slice(&hash[..8]);
    id
}

// ── Block Types ──────────────────────────────────────────────────────────

/// A full SegWit block with separated transaction cores and witnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegWitBlock {
    pub header: CompactBlockHeader,
    pub tx_cores: Vec<TxCore>,
    pub witnesses: Vec<TxWitness>,
    pub tx_root: [u8; 32],
    pub witness_root: [u8; 32],
}

/// Minimal block header for compact block propagation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactBlockHeader {
    pub height: u64,
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
    pub proposer: String,
}

/// Compact representation: header + short IDs instead of full objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactBlock {
    pub header: CompactBlockHeader,
    pub tx_core_short_ids: Vec<ShortId>,
    pub witness_short_ids: Vec<ShortId>,
    pub tx_root: [u8; 32],
    pub witness_root: [u8; 32],
}

impl CompactBlock {
    /// Convert a full SegWit block into a compact block.
    pub fn from_segwit_block(block: &SegWitBlock) -> Self {
        let tx_core_short_ids = block.tx_cores.iter().map(short_id_tx_core).collect();
        let witness_short_ids = block.witnesses.iter().map(short_id_witness).collect();

        Self {
            header: block.header.clone(),
            tx_core_short_ids,
            witness_short_ids,
            tx_root: block.tx_root,
            witness_root: block.witness_root,
        }
    }
}

// ── Mempool ──────────────────────────────────────────────────────────────

/// In-memory pool of known SegWit transaction pairs, indexed by short ID.
pub struct SegWitMempool {
    cores: HashMap<ShortId, TxCore>,
    witnesses: HashMap<ShortId, TxWitness>,
}

impl SegWitMempool {
    pub fn new() -> Self {
        Self {
            cores: HashMap::new(),
            witnesses: HashMap::new(),
        }
    }

    pub fn insert(&mut self, core: TxCore, witness: TxWitness) {
        let core_id = short_id_tx_core(&core);
        let witness_id = short_id_witness(&witness);
        self.cores.insert(core_id, core);
        self.witnesses.insert(witness_id, witness);
    }

    pub fn get_core(&self, id: &ShortId) -> Option<&TxCore> {
        self.cores.get(id)
    }

    pub fn get_witness(&self, id: &ShortId) -> Option<&TxWitness> {
        self.witnesses.get(id)
    }
}

impl Default for SegWitMempool {
    fn default() -> Self {
        Self::new()
    }
}

// ── Missing Object Protocol ──────────────────────────────────────────────

/// Request for missing objects needed to reconstruct a compact block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingCompactRequest {
    pub block_hash: [u8; 32],
    pub missing_tx_core_ids: Vec<ShortId>,
    pub missing_witness_ids: Vec<ShortId>,
}

/// Response carrying the missing objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingCompactResponse {
    pub block_hash: [u8; 32],
    pub tx_cores: Vec<TxCore>,
    pub witnesses: Vec<TxWitness>,
}

/// Partially reconstructed block with slots for missing objects.
#[derive(Debug, Clone)]
pub struct PartialSegWitBlock {
    pub header: CompactBlockHeader,
    pub tx_cores: Vec<Option<TxCore>>,
    pub witnesses: Vec<Option<TxWitness>>,
    pub tx_root: [u8; 32],
    pub witness_root: [u8; 32],
    /// Short IDs from the compact block, for matching response objects.
    pub tx_core_short_ids: Vec<ShortId>,
    pub witness_short_ids: Vec<ShortId>,
}

// ── Errors ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum CompactBlockError {
    #[error("reconstruction incomplete: {missing_cores} cores and {missing_witnesses} witnesses still missing")]
    Incomplete {
        missing_cores: usize,
        missing_witnesses: usize,
    },
    #[error("response contains object with non-matching short_id at index {index}")]
    ShortIdMismatch { index: usize },
    #[error("response contains {got} extra objects (expected {expected})")]
    ExtraObjects { expected: usize, got: usize },
}

// ── Reconstruction ───────────────────────────────────────────────────────

/// Attempt to reconstruct a full block from a compact block using the mempool.
///
/// Returns `Ok(SegWitBlock)` if all objects are found, or
/// `Err(MissingCompactRequest)` listing what's needed.
pub fn reconstruct_compact_block(
    compact: &CompactBlock,
    mempool: &SegWitMempool,
) -> Result<SegWitBlock, MissingCompactRequest> {
    let mut tx_cores = Vec::with_capacity(compact.tx_core_short_ids.len());
    let mut witnesses = Vec::with_capacity(compact.witness_short_ids.len());
    let mut missing_tx_core_ids = Vec::new();
    let mut missing_witness_ids = Vec::new();

    for id in &compact.tx_core_short_ids {
        match mempool.get_core(id) {
            Some(core) => tx_cores.push(Some(core.clone())),
            None => {
                missing_tx_core_ids.push(*id);
                tx_cores.push(None);
            }
        }
    }

    for id in &compact.witness_short_ids {
        match mempool.get_witness(id) {
            Some(witness) => witnesses.push(Some(witness.clone())),
            None => {
                missing_witness_ids.push(*id);
                witnesses.push(None);
            }
        }
    }

    if missing_tx_core_ids.is_empty() && missing_witness_ids.is_empty() {
        // All found — reconstruct
        Ok(SegWitBlock {
            header: compact.header.clone(),
            tx_cores: tx_cores.into_iter().map(|o| o.unwrap()).collect(),
            witnesses: witnesses.into_iter().map(|o| o.unwrap()).collect(),
            tx_root: compact.tx_root,
            witness_root: compact.witness_root,
        })
    } else {
        Err(MissingCompactRequest {
            block_hash: compact.header.hash,
            missing_tx_core_ids,
            missing_witness_ids,
        })
    }
}

/// Apply a missing-objects response to a partial block, producing a full block.
///
/// Validates that each provided object's short ID matches the expected slot.
/// Rejects responses with extra or non-matching objects.
pub fn apply_missing_response(
    compact: &CompactBlock,
    mut partial: PartialSegWitBlock,
    response: MissingCompactResponse,
) -> Result<SegWitBlock, CompactBlockError> {
    // Count how many slots are empty
    let missing_core_count = partial.tx_cores.iter().filter(|o| o.is_none()).count();
    let missing_witness_count = partial.witnesses.iter().filter(|o| o.is_none()).count();

    // Reject extra objects
    if response.tx_cores.len() != missing_core_count {
        return Err(CompactBlockError::ExtraObjects {
            expected: missing_core_count,
            got: response.tx_cores.len(),
        });
    }
    if response.witnesses.len() != missing_witness_count {
        return Err(CompactBlockError::ExtraObjects {
            expected: missing_witness_count,
            got: response.witnesses.len(),
        });
    }

    // Fill in missing tx_cores
    let mut core_iter = response.tx_cores.into_iter();
    for (i, slot) in partial.tx_cores.iter_mut().enumerate() {
        if slot.is_none() {
            let core = core_iter.next().unwrap();
            // Verify short ID matches
            let computed_id = short_id_tx_core(&core);
            if computed_id != compact.tx_core_short_ids[i] {
                return Err(CompactBlockError::ShortIdMismatch { index: i });
            }
            *slot = Some(core);
        }
    }

    // Fill in missing witnesses
    let mut witness_iter = response.witnesses.into_iter();
    for (i, slot) in partial.witnesses.iter_mut().enumerate() {
        if slot.is_none() {
            let witness = witness_iter.next().unwrap();
            // Verify short ID matches
            let computed_id = short_id_witness(&witness);
            if computed_id != compact.witness_short_ids[i] {
                return Err(CompactBlockError::ShortIdMismatch { index: i });
            }
            *slot = Some(witness);
        }
    }

    Ok(SegWitBlock {
        header: partial.header,
        tx_cores: partial.tx_cores.into_iter().map(|o| o.unwrap()).collect(),
        witnesses: partial.witnesses.into_iter().map(|o| o.unwrap()).collect(),
        tx_root: partial.tx_root,
        witness_root: partial.witness_root,
    })
}

/// Build a `PartialSegWitBlock` from a compact block and mempool (for use
/// when reconstruction failed and we need to apply a response later).
pub fn build_partial(compact: &CompactBlock, mempool: &SegWitMempool) -> PartialSegWitBlock {
    let tx_cores: Vec<Option<TxCore>> = compact
        .tx_core_short_ids
        .iter()
        .map(|id| mempool.get_core(id).cloned())
        .collect();
    let witnesses: Vec<Option<TxWitness>> = compact
        .witness_short_ids
        .iter()
        .map(|id| mempool.get_witness(id).cloned())
        .collect();

    PartialSegWitBlock {
        header: compact.header.clone(),
        tx_cores,
        witnesses,
        tx_root: compact.tx_root,
        witness_root: compact.witness_root,
        tx_core_short_ids: compact.tx_core_short_ids.clone(),
        witness_short_ids: compact.witness_short_ids.clone(),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
    use crate::transaction::native::TransactionKind;
    use crate::transaction::segwit::{compute_tx_root, compute_witness_root};
    use crate::transaction::verification_cache::{
        validate_segwit_block_parallel, VerificationCache,
    };

    fn make_signed_pair(
        provider: &dyn SigningProvider,
        from: &str,
        to: &str,
        amount: u64,
        nonce: u64,
    ) -> (TxCore, TxWitness) {
        let core = TxCore {
            from: from.into(),
            to: to.into(),
            amount,
            fee: 5,
            nonce,
            chain_id: 1,
            timestamp: 1000,
            kind: Some(TransactionKind::Transfer {
                from: from.into(),
                to: to.into(),
                amount,
            }),
        };
        let payload = core.signing_payload();
        let sig = provider.sign(&payload).unwrap();
        let witness = TxWitness {
            signature: sig,
            public_key: provider.public_key(),
            signature_scheme: provider.algorithm(),
        };
        (core, witness)
    }

    fn make_test_block(n: usize) -> (SegWitBlock, Vec<SoftwareSigningProvider>) {
        let providers: Vec<SoftwareSigningProvider> = (0..n)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| {
                make_signed_pair(p, &format!("s{i}"), &format!("r{i}"), 100 + i as u64, 0)
            })
            .unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        let block = SegWitBlock {
            header: CompactBlockHeader {
                height: 1,
                hash: tx_root, // simplified — use tx_root as block hash for testing
                parent_hash: [0u8; 32],
                timestamp: 1000,
                proposer: "validator".into(),
            },
            tx_cores: cores,
            witnesses,
            tx_root,
            witness_root,
        };
        (block, providers)
    }

    // ── 1. Full block → compact preserves count and order ────────────────

    #[test]
    fn compact_preserves_count_and_order() {
        let (block, _) = make_test_block(5);
        let compact = CompactBlock::from_segwit_block(&block);

        assert_eq!(compact.tx_core_short_ids.len(), 5);
        assert_eq!(compact.witness_short_ids.len(), 5);
        assert_eq!(compact.header, block.header);

        // Verify order matches direct computation
        for (i, core) in block.tx_cores.iter().enumerate() {
            assert_eq!(compact.tx_core_short_ids[i], short_id_tx_core(core));
        }
        for (i, witness) in block.witnesses.iter().enumerate() {
            assert_eq!(compact.witness_short_ids[i], short_id_witness(witness));
        }
    }

    // ── 2. Full reconstruction from mempool ──────────────────────────────

    #[test]
    fn full_reconstruction_from_mempool() {
        let (block, _) = make_test_block(5);
        let compact = CompactBlock::from_segwit_block(&block);

        let mut mempool = SegWitMempool::new();
        for (core, witness) in block.tx_cores.iter().zip(block.witnesses.iter()) {
            mempool.insert(core.clone(), witness.clone());
        }

        let reconstructed = reconstruct_compact_block(&compact, &mempool).unwrap();
        assert_eq!(reconstructed, block);
    }

    // ── 3. Missing tx_core returns MissingCompactRequest ─────────────────

    #[test]
    fn missing_tx_core_returns_request() {
        let (block, _) = make_test_block(3);
        let compact = CompactBlock::from_segwit_block(&block);

        // Only insert 2 of 3 cores
        let mut mempool = SegWitMempool::new();
        for (core, witness) in block.tx_cores[..2].iter().zip(block.witnesses[..2].iter()) {
            mempool.insert(core.clone(), witness.clone());
        }
        // Insert all witnesses
        mempool.witnesses.insert(
            short_id_witness(&block.witnesses[2]),
            block.witnesses[2].clone(),
        );

        let err = reconstruct_compact_block(&compact, &mempool).unwrap_err();
        assert_eq!(err.missing_tx_core_ids.len(), 1);
        assert_eq!(err.missing_tx_core_ids[0], compact.tx_core_short_ids[2]);
    }

    // ── 4. Missing witness returns MissingCompactRequest ─────────────────

    #[test]
    fn missing_witness_returns_request() {
        let (block, _) = make_test_block(3);
        let compact = CompactBlock::from_segwit_block(&block);

        let mut mempool = SegWitMempool::new();
        // Insert all cores
        for core in &block.tx_cores {
            mempool.cores.insert(short_id_tx_core(core), core.clone());
        }
        // Insert only 2 of 3 witnesses
        for witness in &block.witnesses[..2] {
            mempool
                .witnesses
                .insert(short_id_witness(witness), witness.clone());
        }

        let err = reconstruct_compact_block(&compact, &mempool).unwrap_err();
        assert!(err.missing_tx_core_ids.is_empty());
        assert_eq!(err.missing_witness_ids.len(), 1);
        assert_eq!(err.missing_witness_ids[0], compact.witness_short_ids[2]);
    }

    // ── 5. Missing response reconstructs correctly ───────────────────────

    #[test]
    fn missing_response_reconstructs() {
        let (block, _) = make_test_block(3);
        let compact = CompactBlock::from_segwit_block(&block);

        // Mempool has only first 2 pairs
        let mut mempool = SegWitMempool::new();
        for (core, witness) in block.tx_cores[..2].iter().zip(block.witnesses[..2].iter()) {
            mempool.insert(core.clone(), witness.clone());
        }

        let partial = build_partial(&compact, &mempool);
        let response = MissingCompactResponse {
            block_hash: compact.header.hash,
            tx_cores: vec![block.tx_cores[2].clone()],
            witnesses: vec![block.witnesses[2].clone()],
        };

        let reconstructed = apply_missing_response(&compact, partial, response).unwrap();
        assert_eq!(reconstructed, block);
    }

    // ── 6. Response with wrong short_id fails ────────────────────────────

    #[test]
    fn response_wrong_short_id_fails() {
        let (block, _) = make_test_block(2);
        let compact = CompactBlock::from_segwit_block(&block);

        // Empty mempool — everything missing
        let mempool = SegWitMempool::new();
        let partial = build_partial(&compact, &mempool);

        // Provide wrong core (different amount → different short_id)
        let mut wrong_core = block.tx_cores[0].clone();
        wrong_core.amount = 99999;
        wrong_core.kind = Some(TransactionKind::Transfer {
            from: wrong_core.from.clone(),
            to: wrong_core.to.clone(),
            amount: 99999,
        });

        let response = MissingCompactResponse {
            block_hash: compact.header.hash,
            tx_cores: vec![wrong_core, block.tx_cores[1].clone()],
            witnesses: block.witnesses.clone(),
        };

        let err = apply_missing_response(&compact, partial, response).unwrap_err();
        assert!(matches!(err, CompactBlockError::ShortIdMismatch { .. }));
    }

    // ── 7. Response with extra objects fails ─────────────────────────────

    #[test]
    fn response_extra_objects_fails() {
        let (block, _) = make_test_block(2);
        let compact = CompactBlock::from_segwit_block(&block);

        // Mempool has first pair, missing second
        let mut mempool = SegWitMempool::new();
        mempool.insert(block.tx_cores[0].clone(), block.witnesses[0].clone());
        let partial = build_partial(&compact, &mempool);

        // Response has 2 cores but only 1 is missing
        let response = MissingCompactResponse {
            block_hash: compact.header.hash,
            tx_cores: vec![block.tx_cores[1].clone(), block.tx_cores[0].clone()],
            witnesses: vec![block.witnesses[1].clone()],
        };

        let err = apply_missing_response(&compact, partial, response).unwrap_err();
        assert!(matches!(err, CompactBlockError::ExtraObjects { .. }));
    }

    // ── 8. Witness swap fails after reconstruction ───────────────────────

    #[test]
    fn witness_swap_fails_after_reconstruction() {
        let (block, _) = make_test_block(2);

        // Swap witnesses in the "block"
        let mut swapped = block.clone();
        swapped.witnesses = vec![block.witnesses[1].clone(), block.witnesses[0].clone()];
        swapped.witness_root = compute_witness_root(&swapped.witnesses);

        let compact = CompactBlock::from_segwit_block(&swapped);
        let mut mempool = SegWitMempool::new();
        for (core, witness) in swapped.tx_cores.iter().zip(swapped.witnesses.iter()) {
            mempool.insert(core.clone(), witness.clone());
        }

        let reconstructed = reconstruct_compact_block(&compact, &mempool).unwrap();
        let mut cache = VerificationCache::new(100);
        let result = validate_segwit_block_parallel(
            &reconstructed.tx_cores,
            &reconstructed.witnesses,
            &reconstructed.tx_root,
            &reconstructed.witness_root,
            &mut cache,
        );
        assert!(result.is_err());
    }

    // ── 9. Root mismatch fails after reconstruction ──────────────────────

    #[test]
    fn root_mismatch_fails_after_reconstruction() {
        let (block, _) = make_test_block(3);

        // Corrupt tx_root
        let mut bad_block = block.clone();
        bad_block.tx_root = [0xFFu8; 32];

        let compact = CompactBlock::from_segwit_block(&bad_block);
        let mut mempool = SegWitMempool::new();
        for (core, witness) in bad_block.tx_cores.iter().zip(bad_block.witnesses.iter()) {
            mempool.insert(core.clone(), witness.clone());
        }

        let reconstructed = reconstruct_compact_block(&compact, &mempool).unwrap();
        let mut cache = VerificationCache::new(100);
        let result = validate_segwit_block_parallel(
            &reconstructed.tx_cores,
            &reconstructed.witnesses,
            &reconstructed.tx_root,
            &reconstructed.witness_root,
            &mut cache,
        );
        assert!(result.is_err());
    }

    // ── 10. Reconstructed block validates with parallel ──────────────────

    #[test]
    fn reconstructed_validates_parallel() {
        let (block, _) = make_test_block(10);
        let compact = CompactBlock::from_segwit_block(&block);

        let mut mempool = SegWitMempool::new();
        for (core, witness) in block.tx_cores.iter().zip(block.witnesses.iter()) {
            mempool.insert(core.clone(), witness.clone());
        }

        let reconstructed = reconstruct_compact_block(&compact, &mempool).unwrap();
        let mut cache = VerificationCache::new(100);
        let result = validate_segwit_block_parallel(
            &reconstructed.tx_cores,
            &reconstructed.witnesses,
            &reconstructed.tx_root,
            &reconstructed.witness_root,
            &mut cache,
        );
        assert!(result.is_ok());
    }

    // ── 11. Short ID collision doesn't accept invalid block ──────────────

    #[test]
    fn collision_does_not_accept_invalid() {
        let (block, _) = make_test_block(2);
        let compact = CompactBlock::from_segwit_block(&block);

        // Simulate: mempool has objects with correct short IDs but we tamper
        // the actual block data. Since short IDs match, reconstruction succeeds,
        // but validation must still fail.
        let mut mempool = SegWitMempool::new();
        for (core, witness) in block.tx_cores.iter().zip(block.witnesses.iter()) {
            mempool.insert(core.clone(), witness.clone());
        }

        let mut reconstructed = reconstruct_compact_block(&compact, &mempool).unwrap();
        // Tamper a core after reconstruction (simulates a collision scenario
        // where the short ID accidentally matched a different object)
        reconstructed.tx_cores[0].amount = 99999;
        reconstructed.tx_cores[0].kind = Some(TransactionKind::Transfer {
            from: reconstructed.tx_cores[0].from.clone(),
            to: reconstructed.tx_cores[0].to.clone(),
            amount: 99999,
        });

        let mut cache = VerificationCache::new(100);
        let result = validate_segwit_block_parallel(
            &reconstructed.tx_cores,
            &reconstructed.witnesses,
            &reconstructed.tx_root,
            &reconstructed.witness_root,
            &mut cache,
        );
        // Must fail — either tx_root mismatch or signature verification
        assert!(result.is_err());
    }

    // ── 12. Compact block reduces size versus full block ─────────────────

    #[test]
    fn compact_reduces_size() {
        let (block, _) = make_test_block(20);
        let compact = CompactBlock::from_segwit_block(&block);

        let full_size = serde_json::to_vec(&block).unwrap().len();
        let compact_size = serde_json::to_vec(&compact).unwrap().len();

        eprintln!(
            "Size comparison (20 txs): full={full_size} bytes, compact={compact_size} bytes, \
             reduction={:.1}%",
            (1.0 - compact_size as f64 / full_size as f64) * 100.0
        );

        // Compact must be significantly smaller
        assert!(
            compact_size < full_size / 2,
            "compact ({compact_size}) should be less than half of full ({full_size})"
        );
    }
}
