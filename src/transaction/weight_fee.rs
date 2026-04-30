//! Weight-based fee model for SegWit/PQC transactions.
//!
//! Fees are proportional to the real cost of a transaction:
//! - `TxCore` is weighted higher (stored and re-executed by all nodes)
//! - `TxWitness` is weighted lower (verified once, can be pruned)
//!
//! ML-DSA-65 naturally costs more than Ed25519 because its witness is ~50x
//! larger (3309-byte sig + 1952-byte pk vs 64 + 32), not because of a
//! hard-coded surcharge.

use thiserror::Error;

use crate::transaction::segwit::{
    compute_tx_root, compute_witness_root, verify_witness, SegwitValidationError, TxCore, TxWitness,
};
use crate::transaction::verification_cache::VerificationCache;

// ── Constants ────────────────────────────────────────────────────────────

/// Core data is stored permanently and re-executed — weighted 4x.
pub const CORE_MULTIPLIER: u64 = 4;

/// Witness data is verified once and can be pruned — weighted 1x.
pub const WITNESS_MULTIPLIER: u64 = 1;

/// Minimum base fee regardless of weight (in smallest NOTA unit).
pub const BASE_TX_FEE: u64 = 1;

/// Fee per weight unit (in smallest NOTA unit).
pub const FEE_PER_WEIGHT_UNIT: u64 = 1;

// ── Weight Calculation ───────────────────────────────────────────────────

/// Compute the weight of a `(TxCore, TxWitness)` pair.
///
/// `weight = serialized_size(core) * CORE_MULTIPLIER + serialized_size(witness) * WITNESS_MULTIPLIER`
pub fn calculate_tx_weight(core: &TxCore, witness: &TxWitness) -> u64 {
    let core_size = serde_json::to_vec(core)
        .expect("TxCore serialization cannot fail")
        .len() as u64;
    let witness_size = serde_json::to_vec(witness)
        .expect("TxWitness serialization cannot fail")
        .len() as u64;

    core_size * CORE_MULTIPLIER + witness_size * WITNESS_MULTIPLIER
}

/// Compute the minimum required fee for a transaction based on its weight.
pub fn calculate_required_fee(core: &TxCore, witness: &TxWitness) -> u64 {
    let weight = calculate_tx_weight(core, witness);
    BASE_TX_FEE + weight * FEE_PER_WEIGHT_UNIT
}

// ── Fee Validation ───────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum FeeValidationError {
    #[error("fee too low: required {required}, got {got}")]
    FeeTooLow { required: u64, got: u64 },
}

/// Validate that a transaction's fee covers its weight cost.
pub fn validate_fee(core: &TxCore, witness: &TxWitness) -> Result<(), FeeValidationError> {
    let required = calculate_required_fee(core, witness);
    if core.fee < required {
        return Err(FeeValidationError::FeeTooLow {
            required,
            got: core.fee,
        });
    }
    Ok(())
}

// ── Block Validation with Fees ───────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SegwitFeeBlockError {
    #[error("segwit validation error: {0}")]
    Segwit(#[from] SegwitValidationError),
    #[error("fee validation error at index {index}: {source}")]
    Fee {
        index: usize,
        source: FeeValidationError,
    },
}

/// Validate a SegWit block with both cryptographic and fee checks.
///
/// 1. Length, root, and signature checks (with cache + parallel)
/// 2. Per-transaction fee validation
pub fn validate_segwit_block_with_fees(
    tx_cores: &[TxCore],
    witnesses: &[TxWitness],
    tx_root: &[u8; 32],
    witness_root: &[u8; 32],
    cache: &mut VerificationCache,
) -> Result<(), SegwitFeeBlockError> {
    // 1. Length check
    if tx_cores.len() != witnesses.len() {
        return Err(SegwitValidationError::LengthMismatch {
            cores: tx_cores.len(),
            witnesses: witnesses.len(),
        }
        .into());
    }

    // 2. Roots
    let computed_tx_root = compute_tx_root(tx_cores);
    if &computed_tx_root != tx_root {
        return Err(SegwitValidationError::TxRootMismatch.into());
    }
    let computed_witness_root = compute_witness_root(witnesses);
    if &computed_witness_root != witness_root {
        return Err(SegwitValidationError::WitnessRootMismatch.into());
    }

    // 3. Per-pair: fee + signature verification (cache-accelerated)
    for (i, (core, witness)) in tx_cores.iter().zip(witnesses.iter()).enumerate() {
        // Fee check first (cheap)
        validate_fee(core, witness)
            .map_err(|source| SegwitFeeBlockError::Fee { index: i, source })?;

        // Signature check (skip if cached)
        if !cache.contains_valid(core, witness) {
            let payload = core.signing_payload();
            let valid = verify_witness(&payload, witness)
                .map_err(|_| SegwitValidationError::InvalidWitness { index: i })?;
            if !valid {
                return Err(SegwitValidationError::InvalidWitness { index: i }.into());
            }
            cache.insert_valid(core, witness);
        }
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{
        MlDsaSigningProvider, SigningProvider, SoftwareSigningProvider,
    };
    use crate::transaction::native::TransactionKind;
    use crate::transaction::segwit::compute_tx_root;

    fn make_pair(
        provider: &dyn SigningProvider,
        from: &str,
        to: &str,
        amount: u64,
        fee: u64,
    ) -> (TxCore, TxWitness) {
        let core = TxCore {
            from: from.into(),
            to: to.into(),
            amount,
            fee,
            nonce: 0,
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

    // ── 1. Ed25519 has lower required_fee than ML-DSA ────────────────────

    #[test]
    fn ed25519_lower_fee_than_mldsa() {
        let ed_provider = SoftwareSigningProvider::generate();
        let pqc_provider = MlDsaSigningProvider::generate();

        let (ed_core, ed_witness) = make_pair(&ed_provider, "a", "b", 100, 10000);
        let (pqc_core, pqc_witness) = make_pair(&pqc_provider, "a", "b", 100, 10000);

        let ed_fee = calculate_required_fee(&ed_core, &ed_witness);
        let pqc_fee = calculate_required_fee(&pqc_core, &pqc_witness);

        eprintln!("Ed25519 required_fee={ed_fee}, ML-DSA required_fee={pqc_fee}");

        assert!(
            ed_fee < pqc_fee,
            "Ed25519 fee ({ed_fee}) should be less than ML-DSA fee ({pqc_fee})"
        );
    }

    // ── 2. ML-DSA pays more due to witness size ──────────────────────────

    #[test]
    fn mldsa_pays_more_due_to_size() {
        let ed_provider = SoftwareSigningProvider::generate();
        let pqc_provider = MlDsaSigningProvider::generate();

        let (ed_core, ed_witness) = make_pair(&ed_provider, "a", "b", 100, 10000);
        let (pqc_core, pqc_witness) = make_pair(&pqc_provider, "a", "b", 100, 10000);

        let ed_weight = calculate_tx_weight(&ed_core, &ed_witness);
        let pqc_weight = calculate_tx_weight(&pqc_core, &pqc_witness);

        // ML-DSA witness is ~50x larger, so weight diff comes from witness
        let ed_witness_size = serde_json::to_vec(&ed_witness).unwrap().len();
        let pqc_witness_size = serde_json::to_vec(&pqc_witness).unwrap().len();

        eprintln!(
            "Witness sizes: Ed25519={ed_witness_size}, ML-DSA={pqc_witness_size}, \
             Weights: Ed25519={ed_weight}, ML-DSA={pqc_weight}"
        );

        assert!(pqc_weight > ed_weight);
        assert!(pqc_witness_size > ed_witness_size * 10);
    }

    // ── 3. Insufficient fee fails ────────────────────────────────────────

    #[test]
    fn insufficient_fee_fails() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_pair(&provider, "a", "b", 100, 1); // fee=1, too low

        let err = validate_fee(&core, &witness).unwrap_err();
        assert!(matches!(err, FeeValidationError::FeeTooLow { .. }));
    }

    // ── 4. Exact fee passes ──────────────────────────────────────────────

    #[test]
    fn exact_fee_passes() {
        let provider = SoftwareSigningProvider::generate();
        // Create with a fixed fee, then check the required fee for that exact pair
        let (core, witness) = make_pair(&provider, "a", "b", 100, 5000);
        let required = calculate_required_fee(&core, &witness);
        // Manually set fee to exactly required (core is already signed, fee is just data)
        let mut exact_core = core;
        exact_core.fee = required;
        // Note: signature won't verify anymore, but validate_fee only checks fee amount
        assert!(validate_fee(&exact_core, &witness).is_ok());
    }

    // ── 5. Higher fee passes ─────────────────────────────────────────────

    #[test]
    fn higher_fee_passes() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_pair(&provider, "a", "b", 100, 99999);
        let required = calculate_required_fee(&core, &witness);
        assert!(core.fee > required);
        assert!(validate_fee(&core, &witness).is_ok());
    }

    // ── 6. Changing witness changes required_fee ─────────────────────────

    #[test]
    fn changing_witness_changes_fee() {
        let ed_provider = SoftwareSigningProvider::generate();
        let pqc_provider = MlDsaSigningProvider::generate();

        // Same core data, different witness (different provider)
        let core = TxCore {
            from: "a".into(),
            to: "b".into(),
            amount: 100,
            fee: 100000,
            nonce: 0,
            chain_id: 1,
            timestamp: 1000,
            kind: Some(TransactionKind::Transfer {
                from: "a".into(),
                to: "b".into(),
                amount: 100,
            }),
        };
        let payload = core.signing_payload();

        let ed_sig = ed_provider.sign(&payload).unwrap();
        let ed_witness = TxWitness {
            signature: ed_sig,
            public_key: ed_provider.public_key(),
            signature_scheme: ed_provider.algorithm(),
        };

        let pqc_sig = pqc_provider.sign(&payload).unwrap();
        let pqc_witness = TxWitness {
            signature: pqc_sig,
            public_key: pqc_provider.public_key(),
            signature_scheme: pqc_provider.algorithm(),
        };

        let fee_ed = calculate_required_fee(&core, &ed_witness);
        let fee_pqc = calculate_required_fee(&core, &pqc_witness);
        assert_ne!(fee_ed, fee_pqc);
    }

    // ── 7. Fee validation doesn't break crypto validation ────────────────

    #[test]
    fn fee_validation_does_not_break_crypto() {
        let provider = SoftwareSigningProvider::generate();
        let (probe_core, probe_witness) = make_pair(&provider, "a", "b", 100, 0);
        let required = calculate_required_fee(&probe_core, &probe_witness);

        // Create a block with sufficient fees
        let pairs: Vec<(TxCore, TxWitness)> = (0..5)
            .map(|i| {
                make_pair(
                    &provider,
                    &format!("s{i}"),
                    &format!("r{i}"),
                    100,
                    required + 100,
                )
            })
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();

        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);
        let mut cache = VerificationCache::new(100);

        let result = validate_segwit_block_with_fees(
            &cores,
            &witnesses,
            &tx_root,
            &witness_root,
            &mut cache,
        );
        assert!(result.is_ok());
    }
}
