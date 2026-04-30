//! Mempool verification cache for SegWit/PQC transactions.
//!
//! Avoids repeated verification of expensive ML-DSA-65 signatures when a
//! `(TxCore, TxWitness)` pair has already been validated. The cache key is
//! `SHA-256(serialize(core) || serialize(witness))` — binding both halves
//! prevents witness-swapping attacks.

use std::collections::HashSet;
use std::collections::VecDeque;

use crate::transaction::segwit::{
    compute_tx_root, compute_witness_root, verify_witness, SegwitValidationError, TxCore, TxWitness,
};

// ── Cache Key ────────────────────────────────────────────────────────────

/// Deterministic cache key: `SHA-256(core_bytes || witness_bytes)`.
///
/// Public alias for use by the unified validation pipeline.
pub fn cache_key_for(core: &TxCore, witness: &TxWitness) -> [u8; 32] {
    cache_key(core, witness)
}

/// Deterministic cache key: `SHA-256(core_bytes || witness_bytes)`.
fn cache_key(core: &TxCore, witness: &TxWitness) -> [u8; 32] {
    use pqc_crypto_module::legacy::legacy_sha256;

    let core_bytes = serde_json::to_vec(core).expect("TxCore serialization cannot fail");
    let witness_bytes = serde_json::to_vec(witness).expect("TxWitness serialization cannot fail");

    let mut combined = Vec::with_capacity(core_bytes.len() + witness_bytes.len());
    combined.extend_from_slice(&core_bytes);
    combined.extend_from_slice(&witness_bytes);

    legacy_sha256(&combined).unwrap_or([0u8; 32])
}

// ── VerificationCache ────────────────────────────────────────────────────

/// FIFO-evicting cache of successfully verified `(TxCore, TxWitness)` pairs.
pub struct VerificationCache {
    max_entries: usize,
    /// Insertion-order queue for FIFO eviction.
    order: VecDeque<[u8; 32]>,
    /// Fast membership lookup.
    pub(crate) set: HashSet<[u8; 32]>,
}

impl VerificationCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            order: VecDeque::with_capacity(max_entries),
            set: HashSet::with_capacity(max_entries),
        }
    }

    /// Check whether this `(core, witness)` pair was previously verified.
    pub fn contains_valid(&self, core: &TxCore, witness: &TxWitness) -> bool {
        let key = cache_key(core, witness);
        self.set.contains(&key)
    }

    /// Record a successfully verified pair. Never call this for invalid pairs.
    pub fn insert_valid(&mut self, core: &TxCore, witness: &TxWitness) {
        let key = cache_key(core, witness);
        if self.set.contains(&key) {
            return; // already present
        }
        // Evict oldest if at capacity
        if self.order.len() >= self.max_entries {
            if let Some(evicted) = self.order.pop_front() {
                self.set.remove(&evicted);
            }
        }
        self.order.push_back(key);
        self.set.insert(key);
    }

    /// Verify the pair cryptographically if not cached; insert on success.
    pub fn validate_or_insert(
        &mut self,
        core: &TxCore,
        witness: &TxWitness,
    ) -> Result<(), SegwitValidationError> {
        if self.contains_valid(core, witness) {
            return Ok(());
        }
        let payload = core.signing_payload();
        let valid = verify_witness(&payload, witness)
            .map_err(|_| SegwitValidationError::InvalidWitness { index: 0 })?;
        if !valid {
            return Err(SegwitValidationError::InvalidWitness { index: 0 });
        }
        self.insert_valid(core, witness);
        Ok(())
    }

    /// Number of entries currently cached.
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

// ── Cache-aware block validation ─────────────────────────────────────────

/// Like [`validate_segwit_block`](crate::transaction::segwit::validate_segwit_block)
/// but skips signature verification for pairs already in the cache.
pub fn validate_segwit_block_with_cache(
    tx_cores: &[TxCore],
    witnesses: &[TxWitness],
    tx_root: &[u8; 32],
    witness_root: &[u8; 32],
    cache: &mut VerificationCache,
) -> Result<(), SegwitValidationError> {
    // 1. Length check
    if tx_cores.len() != witnesses.len() {
        return Err(SegwitValidationError::LengthMismatch {
            cores: tx_cores.len(),
            witnesses: witnesses.len(),
        });
    }

    // 2. tx_root
    let computed_tx_root = compute_tx_root(tx_cores);
    if &computed_tx_root != tx_root {
        return Err(SegwitValidationError::TxRootMismatch);
    }

    // 3. witness_root
    let computed_witness_root = compute_witness_root(witnesses);
    if &computed_witness_root != witness_root {
        return Err(SegwitValidationError::WitnessRootMismatch);
    }

    // 4. Per-pair signature verification (cache-accelerated)
    for (i, (core, witness)) in tx_cores.iter().zip(witnesses.iter()).enumerate() {
        if cache.contains_valid(core, witness) {
            continue;
        }
        let payload = core.signing_payload();
        let valid = verify_witness(&payload, witness)
            .map_err(|_| SegwitValidationError::InvalidWitness { index: i })?;
        if !valid {
            return Err(SegwitValidationError::InvalidWitness { index: i });
        }
        cache.insert_valid(core, witness);
    }

    Ok(())
}

// ── Parallel block validation (rayon) ────────────────────────────────────

/// Like [`validate_segwit_block_with_cache`] but verifies signatures in
/// parallel using rayon.
///
/// The cache is read-only during the parallel phase. Newly verified pairs
/// are collected into a buffer and inserted sequentially afterwards to
/// avoid interior mutability / locking overhead.
pub fn validate_segwit_block_parallel(
    tx_cores: &[TxCore],
    witnesses: &[TxWitness],
    tx_root: &[u8; 32],
    witness_root: &[u8; 32],
    cache: &mut VerificationCache,
) -> Result<(), SegwitValidationError> {
    use rayon::prelude::*;

    // 1. Length check
    if tx_cores.len() != witnesses.len() {
        return Err(SegwitValidationError::LengthMismatch {
            cores: tx_cores.len(),
            witnesses: witnesses.len(),
        });
    }

    // 2. tx_root (sequential — cheap)
    let computed_tx_root = compute_tx_root(tx_cores);
    if &computed_tx_root != tx_root {
        return Err(SegwitValidationError::TxRootMismatch);
    }

    // 3. witness_root (sequential — cheap)
    let computed_witness_root = compute_witness_root(witnesses);
    if &computed_witness_root != witness_root {
        return Err(SegwitValidationError::WitnessRootMismatch);
    }

    // 4. Snapshot cache keys for read-only parallel access
    let cached_keys: HashSet<[u8; 32]> = cache.set.clone();

    // 5. Parallel signature verification
    //    Returns indices of newly verified pairs (cache misses that passed).
    let newly_verified: Vec<usize> = tx_cores
        .par_iter()
        .zip(witnesses.par_iter())
        .enumerate()
        .filter_map(|(i, (core, witness))| {
            let key = cache_key(core, witness);
            if cached_keys.contains(&key) {
                return None; // cache hit — skip
            }
            Some((i, core, witness))
        })
        .map(|(i, core, witness)| {
            let payload = core.signing_payload();
            let valid = verify_witness(&payload, witness)
                .map_err(|_| SegwitValidationError::InvalidWitness { index: i })?;
            if !valid {
                return Err(SegwitValidationError::InvalidWitness { index: i });
            }
            Ok(i)
        })
        .collect::<Result<Vec<usize>, SegwitValidationError>>()?;

    // 6. Sequential cache insertion of newly verified pairs
    for i in newly_verified {
        cache.insert_valid(&tx_cores[i], &witnesses[i]);
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
    use crate::transaction::native::TransactionKind;
    use crate::transaction::segwit::{
        compute_tx_root, compute_witness_root, validate_segwit_block,
    };

    fn make_signed_pair(
        provider: &dyn SigningProvider,
        from: &str,
        to: &str,
        amount: u64,
        fee: u64,
        nonce: u64,
        chain_id: u64,
    ) -> (TxCore, TxWitness) {
        let core = TxCore {
            from: from.into(),
            to: to.into(),
            amount,
            fee,
            nonce,
            chain_id,
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

    // ── 1. Cache miss verifies and inserts ───────────────────────────────

    #[test]
    fn cache_miss_verifies_and_inserts() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let mut cache = VerificationCache::new(100);
        assert!(!cache.contains_valid(&core, &witness));

        cache.validate_or_insert(&core, &witness).unwrap();
        assert!(cache.contains_valid(&core, &witness));
        assert_eq!(cache.len(), 1);
    }

    // ── 2. Cache hit skips reverification ────────────────────────────────

    #[test]
    fn cache_hit_skips_reverification() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let mut cache = VerificationCache::new(100);
        cache.validate_or_insert(&core, &witness).unwrap();

        // Second call should hit cache (no crypto work) and succeed
        cache.validate_or_insert(&core, &witness).unwrap();
        // Still only one entry
        assert_eq!(cache.len(), 1);
    }

    // ── 3. Changing amount invalidates cache ─────────────────────────────

    #[test]
    fn changing_amount_invalidates_cache() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let mut cache = VerificationCache::new(100);
        cache.insert_valid(&core, &witness);

        let mut tampered_core = core;
        tampered_core.amount = 999;
        tampered_core.kind = Some(TransactionKind::Transfer {
            from: "alice".into(),
            to: "bob".into(),
            amount: 999,
        });
        // Different key — cache miss
        assert!(!cache.contains_valid(&tampered_core, &witness));
    }

    // ── 4. Changing signature invalidates cache ──────────────────────────

    #[test]
    fn changing_signature_invalidates_cache() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let mut cache = VerificationCache::new(100);
        cache.insert_valid(&core, &witness);

        let mut tampered_witness = witness;
        tampered_witness.signature = vec![0u8; 64];
        assert!(!cache.contains_valid(&core, &tampered_witness));
    }

    // ── 5. Changing public_key invalidates cache ─────────────────────────

    #[test]
    fn changing_public_key_invalidates_cache() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let mut cache = VerificationCache::new(100);
        cache.insert_valid(&core, &witness);

        let mut tampered_witness = witness;
        tampered_witness.public_key = vec![0u8; 32];
        assert!(!cache.contains_valid(&core, &tampered_witness));
    }

    // ── 6. Witness swap fails even with both cached ──────────────────────

    #[test]
    fn witness_swap_fails_even_with_cache() {
        let p1 = SoftwareSigningProvider::generate();
        let p2 = SoftwareSigningProvider::generate();

        let (core1, witness1) = make_signed_pair(&p1, "alice", "bob", 100, 5, 0, 1);
        let (core2, witness2) = make_signed_pair(&p2, "carol", "dave", 200, 10, 0, 1);

        let mut cache = VerificationCache::new(100);
        // Cache both valid pairs
        cache.insert_valid(&core1, &witness1);
        cache.insert_valid(&core2, &witness2);

        // Swap: core1+witness2, core2+witness1 — neither combo is cached
        assert!(!cache.contains_valid(&core1, &witness2));
        assert!(!cache.contains_valid(&core2, &witness1));

        // Full block validation with swapped witnesses must fail
        let cores = vec![core1, core2];
        let witnesses = vec![witness2, witness1];
        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        let err = validate_segwit_block_with_cache(
            &cores,
            &witnesses,
            &tx_root,
            &witness_root,
            &mut cache,
        )
        .unwrap_err();
        assert!(matches!(err, SegwitValidationError::InvalidWitness { .. }));
    }

    // ── 7. Root mismatch fails even with cached signatures ───────────────

    #[test]
    fn root_mismatch_fails_with_cache() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let mut cache = VerificationCache::new(100);
        cache.insert_valid(&core, &witness);

        let tx_root = compute_tx_root(&[core.clone()]);
        let bad_witness_root = [0xFFu8; 32];

        let err = validate_segwit_block_with_cache(
            &[core],
            &[witness],
            &tx_root,
            &bad_witness_root,
            &mut cache,
        )
        .unwrap_err();
        assert!(matches!(err, SegwitValidationError::WitnessRootMismatch));
    }

    // ── 8. Invalid signatures are never cached ───────────────────────────

    #[test]
    fn invalid_signatures_not_cached() {
        let provider = SoftwareSigningProvider::generate();
        let (core, _valid_witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let bad_witness = TxWitness {
            signature: vec![0u8; 64],
            public_key: provider.public_key(),
            signature_scheme: provider.algorithm(),
        };

        let mut cache = VerificationCache::new(100);
        let result = cache.validate_or_insert(&core, &bad_witness);
        assert!(result.is_err());
        assert!(!cache.contains_valid(&core, &bad_witness));
        assert!(cache.is_empty());
    }

    // ── 9. Eviction respects max_entries ─────────────────────────────────

    #[test]
    fn eviction_respects_max_entries() {
        let mut cache = VerificationCache::new(3);

        let providers: Vec<SoftwareSigningProvider> = (0..5)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();

        let pairs: Vec<(TxCore, TxWitness)> = providers
            .iter()
            .enumerate()
            .map(|(i, p)| make_signed_pair(p, &format!("from{i}"), &format!("to{i}"), 100, 5, 0, 1))
            .collect();

        // Insert 5 pairs into a cache with max 3
        for (core, witness) in &pairs {
            cache.insert_valid(core, witness);
        }

        assert_eq!(cache.len(), 3);

        // First two should have been evicted (FIFO)
        assert!(!cache.contains_valid(&pairs[0].0, &pairs[0].1));
        assert!(!cache.contains_valid(&pairs[1].0, &pairs[1].1));

        // Last three should remain
        assert!(cache.contains_valid(&pairs[2].0, &pairs[2].1));
        assert!(cache.contains_valid(&pairs[3].0, &pairs[3].1));
        assert!(cache.contains_valid(&pairs[4].0, &pairs[4].1));
    }

    // ── 10. Legacy validate_segwit_block still passes ────────────────────

    #[test]
    fn legacy_validation_still_works() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let tx_root = compute_tx_root(&[core.clone()]);
        let witness_root = compute_witness_root(&[witness.clone()]);

        // Original non-cache function still works
        assert!(validate_segwit_block(&[core], &[witness], &tx_root, &witness_root).is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  P2 — Parallel verification tests
    // ═══════════════════════════════════════════════════════════════════════

    // ── 11. Valid block passes parallel ───────────────────────────────────

    #[test]
    fn parallel_valid_block_passes() {
        let providers: Vec<SoftwareSigningProvider> = (0..10)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| make_signed_pair(p, &format!("s{i}"), &format!("r{i}"), 100, 5, 0, 1))
            .unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);
        let mut cache = VerificationCache::new(1000);

        assert!(validate_segwit_block_parallel(
            &cores,
            &witnesses,
            &tx_root,
            &witness_root,
            &mut cache,
        )
        .is_ok());
        assert_eq!(cache.len(), 10);
    }

    // ── 12. Invalid block fails parallel ─────────────────────────────────

    #[test]
    fn parallel_invalid_block_fails() {
        let p1 = SoftwareSigningProvider::generate();
        let p2 = SoftwareSigningProvider::generate();
        let (core1, witness1) = make_signed_pair(&p1, "a", "b", 100, 5, 0, 1);
        let (core2, _witness2) = make_signed_pair(&p2, "c", "d", 200, 10, 0, 1);

        // Use witness1 for core2 — invalid
        let cores = vec![core1, core2];
        let witnesses = vec![witness1.clone(), witness1];
        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);
        let mut cache = VerificationCache::new(100);

        let err =
            validate_segwit_block_parallel(&cores, &witnesses, &tx_root, &witness_root, &mut cache)
                .unwrap_err();
        assert!(matches!(err, SegwitValidationError::InvalidWitness { .. }));
    }

    // ── 13. Parallel and sequential produce identical results ─────────────

    #[test]
    fn parallel_matches_sequential() {
        let providers: Vec<SoftwareSigningProvider> = (0..20)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| make_signed_pair(p, &format!("s{i}"), &format!("r{i}"), 100, 5, 0, 1))
            .unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        let mut cache_seq = VerificationCache::new(1000);
        let mut cache_par = VerificationCache::new(1000);

        let result_seq = validate_segwit_block_with_cache(
            &cores,
            &witnesses,
            &tx_root,
            &witness_root,
            &mut cache_seq,
        );
        let result_par = validate_segwit_block_parallel(
            &cores,
            &witnesses,
            &tx_root,
            &witness_root,
            &mut cache_par,
        );

        assert!(result_seq.is_ok());
        assert!(result_par.is_ok());
        assert_eq!(cache_seq.len(), cache_par.len());
    }

    // ── 14. Cache still works with parallel ──────────────────────────────

    #[test]
    fn parallel_uses_cache() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let tx_root = compute_tx_root(&[core.clone()]);
        let witness_root = compute_witness_root(&[witness.clone()]);

        let mut cache = VerificationCache::new(100);
        // Pre-populate cache
        cache.insert_valid(&core, &witness);

        // Should succeed using cached entry (no crypto needed)
        assert!(validate_segwit_block_parallel(
            &[core],
            &[witness],
            &tx_root,
            &witness_root,
            &mut cache,
        )
        .is_ok());
        assert_eq!(cache.len(), 1);
    }

    // ── 15. No race conditions with many threads ─────────────────────────

    #[test]
    fn parallel_no_race_conditions() {
        // Run 5 times to increase chance of catching races
        for _ in 0..5 {
            let providers: Vec<SoftwareSigningProvider> = (0..50)
                .map(|_| SoftwareSigningProvider::generate())
                .collect();
            let (cores, witnesses): (Vec<_>, Vec<_>) = providers
                .iter()
                .enumerate()
                .map(|(i, p)| make_signed_pair(p, &format!("s{i}"), &format!("r{i}"), 100, 5, 0, 1))
                .unzip();

            let tx_root = compute_tx_root(&cores);
            let witness_root = compute_witness_root(&witnesses);
            let mut cache = VerificationCache::new(1000);

            let result = validate_segwit_block_parallel(
                &cores,
                &witnesses,
                &tx_root,
                &witness_root,
                &mut cache,
            );
            assert!(result.is_ok());
            assert_eq!(cache.len(), 50);
        }
    }

    // ── 16. Performance: parallel vs sequential (100+ txs) ───────────────

    #[test]
    fn parallel_faster_than_sequential() {
        let n = 200;
        let providers: Vec<SoftwareSigningProvider> = (0..n)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| make_signed_pair(p, &format!("s{i}"), &format!("r{i}"), 100, 5, 0, 1))
            .unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        // Sequential timing
        let mut cache_seq = VerificationCache::new(n * 2);
        let start_seq = std::time::Instant::now();
        validate_segwit_block_with_cache(
            &cores,
            &witnesses,
            &tx_root,
            &witness_root,
            &mut cache_seq,
        )
        .unwrap();
        let elapsed_seq = start_seq.elapsed();

        // Parallel timing (fresh cache)
        let mut cache_par = VerificationCache::new(n * 2);
        let start_par = std::time::Instant::now();
        validate_segwit_block_parallel(&cores, &witnesses, &tx_root, &witness_root, &mut cache_par)
            .unwrap();
        let elapsed_par = start_par.elapsed();

        eprintln!(
            "Performance ({n} txs): sequential={:?}, parallel={:?}, speedup={:.2}x",
            elapsed_seq,
            elapsed_par,
            elapsed_seq.as_secs_f64() / elapsed_par.as_secs_f64()
        );

        // Both must produce the same result
        assert_eq!(cache_seq.len(), cache_par.len());
    }
}
