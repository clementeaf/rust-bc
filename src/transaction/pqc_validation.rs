//! Unified PQC block validation pipeline.
//!
//! `validate_pqc_block` is the single official entry point combining:
//! - Structural checks (length match)
//! - Merkle root verification (tx_root, witness_root)
//! - Weight-based fee validation (optional via config)
//! - Signature verification with cache acceleration and optional parallelism

use std::collections::HashSet;

use thiserror::Error;

use crate::transaction::compact_block::SegWitBlock;
use crate::transaction::segwit::{
    compute_tx_root, compute_witness_root, verify_witness, SegwitValidationError, TxCore, TxWitness,
};
use crate::transaction::verification_cache::{self, VerificationCache};
use crate::transaction::weight_fee::{validate_fee, FeeValidationError};

// ── Configuration ────────────────────────────────────────────────────────

/// Configuration for the PQC validation pipeline.
#[derive(Debug, Clone)]
pub struct PqcValidationConfig {
    /// Whether to enforce weight-based fee requirements.
    pub enforce_fees: bool,
    /// Whether to use the verification cache for signature checks.
    pub use_cache: bool,
    /// Whether to verify signatures in parallel (rayon).
    pub parallel_verify: bool,
}

impl Default for PqcValidationConfig {
    fn default() -> Self {
        Self {
            enforce_fees: true,
            use_cache: true,
            parallel_verify: true,
        }
    }
}

// ── Errors ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PqcBlockError {
    #[error("structural: {0}")]
    Structure(#[from] SegwitValidationError),
    #[error("fee at index {index}: {source}")]
    Fee {
        index: usize,
        source: FeeValidationError,
    },
}

// ── Pipeline ─────────────────────────────────────────────────────────────

/// Official unified validation pipeline for SegWit/PQC blocks.
///
/// Validation order:
/// 1. Structure: `tx_cores.len() == witnesses.len()`
/// 2. Roots: recompute and compare `tx_root` and `witness_root`
/// 3. Fees: if `config.enforce_fees`, validate each pair's fee
/// 4. Signatures: verify each (core, witness) pair, with optional cache and parallelism
pub fn validate_pqc_block(
    block: &SegWitBlock,
    cache: &mut VerificationCache,
    config: &PqcValidationConfig,
) -> Result<(), PqcBlockError> {
    let tx_cores = &block.tx_cores;
    let witnesses = &block.witnesses;

    // 1. Structure
    if tx_cores.len() != witnesses.len() {
        return Err(SegwitValidationError::LengthMismatch {
            cores: tx_cores.len(),
            witnesses: witnesses.len(),
        }
        .into());
    }

    // 2. Roots
    let computed_tx_root = compute_tx_root(tx_cores);
    if computed_tx_root != block.tx_root {
        return Err(SegwitValidationError::TxRootMismatch.into());
    }
    let computed_witness_root = compute_witness_root(witnesses);
    if computed_witness_root != block.witness_root {
        return Err(SegwitValidationError::WitnessRootMismatch.into());
    }

    // 3. Fees (before expensive sig verification)
    if config.enforce_fees {
        for (i, (core, witness)) in tx_cores.iter().zip(witnesses.iter()).enumerate() {
            validate_fee(core, witness)
                .map_err(|source| PqcBlockError::Fee { index: i, source })?;
        }
    }

    // 4. Signatures
    if config.parallel_verify {
        verify_signatures_parallel(tx_cores, witnesses, cache, config.use_cache)?;
    } else {
        verify_signatures_sequential(tx_cores, witnesses, cache, config.use_cache)?;
    }

    Ok(())
}

fn verify_signatures_sequential(
    tx_cores: &[TxCore],
    witnesses: &[TxWitness],
    cache: &mut VerificationCache,
    use_cache: bool,
) -> Result<(), SegwitValidationError> {
    for (i, (core, witness)) in tx_cores.iter().zip(witnesses.iter()).enumerate() {
        if use_cache && cache.contains_valid(core, witness) {
            continue;
        }
        let payload = core.signing_payload();
        let valid = verify_witness(&payload, witness)
            .map_err(|_| SegwitValidationError::InvalidWitness { index: i })?;
        if !valid {
            return Err(SegwitValidationError::InvalidWitness { index: i });
        }
        if use_cache {
            cache.insert_valid(core, witness);
        }
    }
    Ok(())
}

fn verify_signatures_parallel(
    tx_cores: &[TxCore],
    witnesses: &[TxWitness],
    cache: &mut VerificationCache,
    use_cache: bool,
) -> Result<(), SegwitValidationError> {
    use rayon::prelude::*;

    // Snapshot cache for read-only parallel access
    let cached_keys: HashSet<[u8; 32]> = if use_cache {
        cache.set.clone()
    } else {
        HashSet::new()
    };

    let newly_verified: Vec<usize> = tx_cores
        .par_iter()
        .zip(witnesses.par_iter())
        .enumerate()
        .filter_map(|(i, (core, witness))| {
            if use_cache {
                let key = verification_cache::cache_key_for(core, witness);
                if cached_keys.contains(&key) {
                    return None;
                }
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

    // Sequential cache insertion
    if use_cache {
        for i in newly_verified {
            cache.insert_valid(&tx_cores[i], &witnesses[i]);
        }
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
    use crate::transaction::compact_block::{CompactBlockHeader, SegWitBlock};
    use crate::transaction::native::TransactionKind;
    use crate::transaction::segwit::{
        compute_tx_root, compute_witness_root, validate_segwit_block,
    };
    fn make_pair(provider: &dyn SigningProvider, idx: usize, fee: u64) -> (TxCore, TxWitness) {
        let core = TxCore {
            from: format!("s{idx}"),
            to: format!("r{idx}"),
            amount: 100,
            fee,
            nonce: 0,
            chain_id: 1,
            timestamp: 1000,
            kind: Some(TransactionKind::Transfer {
                from: format!("s{idx}"),
                to: format!("r{idx}"),
                amount: 100,
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

    fn make_block_with_fee(n: usize, fee: u64) -> SegWitBlock {
        let providers: Vec<SoftwareSigningProvider> = (0..n)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| make_pair(p, i, fee))
            .unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        SegWitBlock {
            header: CompactBlockHeader {
                height: 1,
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

    fn sufficient_fee() -> u64 {
        // Large enough for any Ed25519 tx
        5000
    }

    fn full_config() -> PqcValidationConfig {
        PqcValidationConfig {
            enforce_fees: true,
            use_cache: true,
            parallel_verify: true,
        }
    }

    // ── 1. Valid block passes with full config ───────────────────────────

    #[test]
    fn valid_block_passes_full_config() {
        let block = make_block_with_fee(5, sufficient_fee());
        let mut cache = VerificationCache::new(100);
        assert!(validate_pqc_block(&block, &mut cache, &full_config()).is_ok());
    }

    // ── 2. Wrong root fails even if cached ───────────────────────────────

    #[test]
    fn wrong_root_fails_even_if_cached() {
        let block = make_block_with_fee(3, sufficient_fee());
        let mut cache = VerificationCache::new(100);

        // Pre-populate cache
        for (core, witness) in block.tx_cores.iter().zip(block.witnesses.iter()) {
            cache.insert_valid(core, witness);
        }

        // Corrupt tx_root
        let mut bad_block = block;
        bad_block.tx_root = [0xFFu8; 32];

        let err = validate_pqc_block(&bad_block, &mut cache, &full_config()).unwrap_err();
        assert!(matches!(
            err,
            PqcBlockError::Structure(SegwitValidationError::TxRootMismatch)
        ));
    }

    // ── 3. Insufficient fee fails even if sig cached ─────────────────────

    #[test]
    fn insufficient_fee_fails_even_if_cached() {
        let block = make_block_with_fee(3, 1); // fee=1, too low
        let mut cache = VerificationCache::new(100);

        // Pre-populate cache
        for (core, witness) in block.tx_cores.iter().zip(block.witnesses.iter()) {
            cache.insert_valid(core, witness);
        }

        let err = validate_pqc_block(&block, &mut cache, &full_config()).unwrap_err();
        assert!(matches!(err, PqcBlockError::Fee { .. }));
    }

    // ── 4. Witness swap fails ────────────────────────────────────────────

    #[test]
    fn witness_swap_fails() {
        let mut block = make_block_with_fee(2, sufficient_fee());
        // Swap witnesses
        block.witnesses.swap(0, 1);
        block.witness_root = compute_witness_root(&block.witnesses);

        let mut cache = VerificationCache::new(100);
        let err = validate_pqc_block(&block, &mut cache, &full_config()).unwrap_err();
        assert!(matches!(
            err,
            PqcBlockError::Structure(SegwitValidationError::InvalidWitness { .. })
        ));
    }

    // ── 5. Valid without fees when enforce_fees=false ─────────────────────

    #[test]
    fn passes_without_fees_when_disabled() {
        let block = make_block_with_fee(3, 1); // fee=1, would fail with fees
        let mut cache = VerificationCache::new(100);

        let config = PqcValidationConfig {
            enforce_fees: false,
            use_cache: true,
            parallel_verify: true,
        };
        assert!(validate_pqc_block(&block, &mut cache, &config).is_ok());
    }

    // ── 6. Cache hit works ───────────────────────────────────────────────

    #[test]
    fn cache_hit_works() {
        let block = make_block_with_fee(3, sufficient_fee());
        let mut cache = VerificationCache::new(100);

        // First pass populates cache
        validate_pqc_block(&block, &mut cache, &full_config()).unwrap();
        assert_eq!(cache.len(), 3);

        // Second pass uses cache (still passes)
        validate_pqc_block(&block, &mut cache, &full_config()).unwrap();
        assert_eq!(cache.len(), 3); // no new insertions
    }

    // ── 7. Cache miss inserts ────────────────────────────────────────────

    #[test]
    fn cache_miss_inserts() {
        let block = make_block_with_fee(5, sufficient_fee());
        let mut cache = VerificationCache::new(100);
        assert!(cache.is_empty());

        validate_pqc_block(&block, &mut cache, &full_config()).unwrap();
        assert_eq!(cache.len(), 5);
    }

    // ── 8. Parallel and sequential same result ───────────────────────────

    #[test]
    fn parallel_and_sequential_same_result() {
        let block = make_block_with_fee(10, sufficient_fee());

        let mut cache_seq = VerificationCache::new(100);
        let mut cache_par = VerificationCache::new(100);

        let config_seq = PqcValidationConfig {
            enforce_fees: true,
            use_cache: true,
            parallel_verify: false,
        };
        let config_par = PqcValidationConfig {
            enforce_fees: true,
            use_cache: true,
            parallel_verify: true,
        };

        let r1 = validate_pqc_block(&block, &mut cache_seq, &config_seq);
        let r2 = validate_pqc_block(&block, &mut cache_par, &config_par);

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert_eq!(cache_seq.len(), cache_par.len());
    }

    // ── 9. use_cache=false doesn't read or write cache ───────────────────

    #[test]
    fn no_cache_mode() {
        let block = make_block_with_fee(3, sufficient_fee());
        let mut cache = VerificationCache::new(100);

        let config = PqcValidationConfig {
            enforce_fees: true,
            use_cache: false,
            parallel_verify: false,
        };

        validate_pqc_block(&block, &mut cache, &config).unwrap();
        assert!(cache.is_empty()); // nothing inserted
    }

    // ── 10. Field tampering invalidates ──────────────────────────────────

    #[test]
    fn field_tampering_invalidates() {
        let block = make_block_with_fee(1, sufficient_fee());
        let mut cache = VerificationCache::new(100);

        // Valid first
        validate_pqc_block(&block, &mut cache, &full_config()).unwrap();

        // Tamper amount → tx_root mismatch
        let mut tampered = block.clone();
        tampered.tx_cores[0].amount = 99999;
        tampered.tx_cores[0].kind = Some(TransactionKind::Transfer {
            from: tampered.tx_cores[0].from.clone(),
            to: tampered.tx_cores[0].to.clone(),
            amount: 99999,
        });
        assert!(validate_pqc_block(&tampered, &mut cache, &full_config()).is_err());

        // Tamper nonce → tx_root mismatch
        let mut tampered = block.clone();
        tampered.tx_cores[0].nonce = 99;
        assert!(validate_pqc_block(&tampered, &mut cache, &full_config()).is_err());

        // Tamper chain_id → tx_root mismatch
        let mut tampered = block.clone();
        tampered.tx_cores[0].chain_id = 9999;
        assert!(validate_pqc_block(&tampered, &mut cache, &full_config()).is_err());

        // Tamper fee → tx_root mismatch
        let mut tampered = block;
        tampered.tx_cores[0].fee = 0;
        assert!(validate_pqc_block(&tampered, &mut cache, &full_config()).is_err());
    }

    // ── 11. Old validators still work ────────────────────────────────────

    #[test]
    fn old_validators_still_work() {
        let block = make_block_with_fee(5, sufficient_fee());

        // Original validate_segwit_block
        assert!(validate_segwit_block(
            &block.tx_cores,
            &block.witnesses,
            &block.tx_root,
            &block.witness_root,
        )
        .is_ok());

        // validate_segwit_block_parallel
        let mut cache = VerificationCache::new(100);
        assert!(
            crate::transaction::verification_cache::validate_segwit_block_parallel(
                &block.tx_cores,
                &block.witnesses,
                &block.tx_root,
                &block.witness_root,
                &mut cache,
            )
            .is_ok()
        );
    }
}
