//! Segregated witness model — separates transaction data from signatures.
//!
//! `TxCore` holds the executable payload (from, to, amount, fee, nonce, etc.)
//! while `TxWitness` holds the cryptographic proof (signature + public key +
//! algorithm tag). This separation allows light clients to skip witness data
//! and supports large PQC signatures without bloating block propagation.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::identity::signing::SigningAlgorithm;
use crate::transaction::native::{NativeTransaction, TransactionKind};

// ── Core Types ───────────────────────────────────────────────────────────

/// The executable part of a transaction — everything that affects state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxCore {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub chain_id: u64,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<TransactionKind>,
}

/// The witness (proof) part of a transaction — signature + public key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxWitness {
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub signature_scheme: SigningAlgorithm,
}

// ── Signing Payload ──────────────────────────────────────────────────────

impl TxCore {
    /// Canonical bytes for signature verification.
    ///
    /// Must match `NativeTransaction::signing_payload()` for backward compat
    /// when the core was converted from a legacy transaction.
    pub fn signing_payload(&self) -> Vec<u8> {
        let kind = self.kind.clone().unwrap_or(TransactionKind::Transfer {
            from: self.from.clone(),
            to: self.to.clone(),
            amount: self.amount,
        });
        let canonical = serde_json::json!({
            "chain_id": self.chain_id,
            "kind": kind,
            "nonce": self.nonce,
            "fee": self.fee,
            "timestamp": self.timestamp,
        });
        canonical.to_string().into_bytes()
    }
}

// ── Conversion from Legacy ───────────────────────────────────────────────

impl NativeTransaction {
    /// Split a legacy transaction into its core data and witness.
    pub fn to_segwit(&self, public_key: Vec<u8>) -> (TxCore, TxWitness) {
        let (from, to, amount) = match &self.kind {
            TransactionKind::Transfer { from, to, amount } => (from.clone(), to.clone(), *amount),
            TransactionKind::Coinbase { to, amount } => (String::new(), to.clone(), *amount),
        };

        let scheme = match self.signature.len() {
            3309 => SigningAlgorithm::MlDsa65,
            _ => SigningAlgorithm::Ed25519,
        };

        let core = TxCore {
            from,
            to,
            amount,
            fee: self.fee,
            nonce: self.nonce,
            chain_id: self.chain_id,
            timestamp: self.timestamp,
            kind: Some(self.kind.clone()),
        };

        let witness = TxWitness {
            signature: self.signature.clone(),
            public_key,
            signature_scheme: scheme,
        };

        (core, witness)
    }
}

// ── Merkle Roots ─────────────────────────────────────────────────────────

/// Compute the Merkle root of a list of `TxCore` entries.
pub fn compute_tx_root(cores: &[TxCore]) -> [u8; 32] {
    compute_merkle_root(
        cores
            .iter()
            .map(|c| serde_json::to_vec(c).expect("TxCore serialization cannot fail")),
    )
}

/// Compute the Merkle root of a list of `TxWitness` entries.
pub fn compute_witness_root(witnesses: &[TxWitness]) -> [u8; 32] {
    compute_merkle_root(
        witnesses
            .iter()
            .map(|w| serde_json::to_vec(w).expect("TxWitness serialization cannot fail")),
    )
}

/// Generic Merkle root over an iterator of byte sequences.
fn compute_merkle_root(items: impl Iterator<Item = Vec<u8>>) -> [u8; 32] {
    use pqc_crypto_module::legacy::legacy_sha256;

    let mut hashes: Vec<[u8; 32]> = items
        .map(|data| legacy_sha256(&data).unwrap_or([0u8; 32]))
        .collect();

    if hashes.is_empty() {
        return [0u8; 32];
    }

    while hashes.len() > 1 {
        let mut next = Vec::with_capacity(hashes.len().div_ceil(2));
        for pair in hashes.chunks(2) {
            let mut combined = Vec::with_capacity(64);
            combined.extend_from_slice(&pair[0]);
            if pair.len() == 2 {
                combined.extend_from_slice(&pair[1]);
            } else {
                combined.extend_from_slice(&pair[0]);
            }
            next.push(legacy_sha256(&combined).unwrap_or([0u8; 32]));
        }
        hashes = next;
    }

    hashes[0]
}

// ── Block Validation ─────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SegwitValidationError {
    #[error("tx_cores/witnesses length mismatch: {cores} cores vs {witnesses} witnesses")]
    LengthMismatch { cores: usize, witnesses: usize },
    #[error("tx_root mismatch")]
    TxRootMismatch,
    #[error("witness_root mismatch")]
    WitnessRootMismatch,
    #[error("witness {index} failed signature verification")]
    InvalidWitness { index: usize },
}

/// Validate segregated-witness block data.
///
/// 1. `tx_cores.len() == witnesses.len()`
/// 2. Each `witnesses[i]` verifies against `tx_cores[i].signing_payload()`
/// 3. `tx_root` matches recomputed Merkle root of `tx_cores`
/// 4. `witness_root` matches recomputed Merkle root of `witnesses`
pub fn validate_segwit_block(
    tx_cores: &[TxCore],
    witnesses: &[TxWitness],
    tx_root: &[u8; 32],
    witness_root: &[u8; 32],
) -> Result<(), SegwitValidationError> {
    // 1. Length check
    if tx_cores.len() != witnesses.len() {
        return Err(SegwitValidationError::LengthMismatch {
            cores: tx_cores.len(),
            witnesses: witnesses.len(),
        });
    }

    // 2. Signature verification per pair
    for (i, (core, witness)) in tx_cores.iter().zip(witnesses.iter()).enumerate() {
        let payload = core.signing_payload();
        let valid = verify_witness(&payload, witness)
            .map_err(|_| SegwitValidationError::InvalidWitness { index: i })?;
        if !valid {
            return Err(SegwitValidationError::InvalidWitness { index: i });
        }
    }

    // 3. tx_root
    let computed_tx_root = compute_tx_root(tx_cores);
    if &computed_tx_root != tx_root {
        return Err(SegwitValidationError::TxRootMismatch);
    }

    // 4. witness_root
    let computed_witness_root = compute_witness_root(witnesses);
    if &computed_witness_root != witness_root {
        return Err(SegwitValidationError::WitnessRootMismatch);
    }

    Ok(())
}

/// Verify a single witness against a signing payload.
pub fn verify_witness(payload: &[u8], witness: &TxWitness) -> Result<bool, SegwitValidationError> {
    match witness.signature_scheme {
        SigningAlgorithm::Ed25519 => {
            use pqc_crypto_module::legacy::ed25519::{Signature, Verifier, VerifyingKey};
            let vk = VerifyingKey::from_bytes(
                witness
                    .public_key
                    .as_slice()
                    .try_into()
                    .map_err(|_| SegwitValidationError::InvalidWitness { index: 0 })?,
            )
            .map_err(|_| SegwitValidationError::InvalidWitness { index: 0 })?;
            let sig_bytes: [u8; 64] = witness
                .signature
                .as_slice()
                .try_into()
                .map_err(|_| SegwitValidationError::InvalidWitness { index: 0 })?;
            let sig = Signature::from_bytes(&sig_bytes);
            Ok(vk.verify(payload, &sig).is_ok())
        }
        SigningAlgorithm::MlDsa65 => {
            use pqc_crypto_module::legacy::mldsa_raw::mldsa65;
            use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _};
            let pk = mldsa65::PublicKey::from_bytes(&witness.public_key)
                .map_err(|_| SegwitValidationError::InvalidWitness { index: 0 })?;
            let sig = mldsa65::DetachedSignature::from_bytes(&witness.signature)
                .map_err(|_| SegwitValidationError::InvalidWitness { index: 0 })?;
            Ok(mldsa65::verify_detached_signature(&sig, payload, &pk).is_ok())
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{
        MlDsaSigningProvider, SigningProvider, SoftwareSigningProvider,
    };

    /// Helper: create a signed (TxCore, TxWitness) pair using the given provider.
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

    // ── 1. Valid block with ML-DSA passes ────────────────────────────────

    #[test]
    fn valid_block_with_mldsa_passes() {
        let provider = MlDsaSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 9999);

        let tx_root = compute_tx_root(&[core.clone()]);
        let witness_root = compute_witness_root(&[witness.clone()]);

        assert!(validate_segwit_block(&[core], &[witness], &tx_root, &witness_root).is_ok());
    }

    // ── 2. Witness swap fails ────────────────────────────────────────────

    #[test]
    fn witness_swap_fails() {
        let p1 = SoftwareSigningProvider::generate();
        let p2 = SoftwareSigningProvider::generate();

        let (core1, witness1) = make_signed_pair(&p1, "alice", "bob", 100, 5, 0, 1);
        let (core2, witness2) = make_signed_pair(&p2, "carol", "dave", 200, 10, 0, 1);

        // Swap witnesses: witness2 on core1, witness1 on core2
        let cores = vec![core1, core2];
        let witnesses = vec![witness2, witness1];
        let tx_root = compute_tx_root(&cores);
        let witness_root = compute_witness_root(&witnesses);

        let err = validate_segwit_block(&cores, &witnesses, &tx_root, &witness_root).unwrap_err();
        assert!(matches!(err, SegwitValidationError::InvalidWitness { .. }));
    }

    // ── 3. Tampering amount/fee/nonce/chain_id fails ─────────────────────

    #[test]
    fn tampering_core_fields_fails() {
        let provider = SoftwareSigningProvider::generate();
        let (original_core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 9999);

        // Tamper amount
        let mut tampered = original_core.clone();
        tampered.amount = 999;
        tampered.kind = Some(TransactionKind::Transfer {
            from: "alice".into(),
            to: "bob".into(),
            amount: 999,
        });
        assert!(verify_witness(&tampered.signing_payload(), &witness).is_ok());
        // The verify returns false because payload changed
        assert!(!verify_witness(&tampered.signing_payload(), &witness).unwrap());

        // Tamper fee
        let mut tampered = original_core.clone();
        tampered.fee = 999;
        assert!(!verify_witness(&tampered.signing_payload(), &witness).unwrap());

        // Tamper nonce
        let mut tampered = original_core.clone();
        tampered.nonce = 999;
        assert!(!verify_witness(&tampered.signing_payload(), &witness).unwrap());

        // Tamper chain_id
        let mut tampered = original_core;
        tampered.chain_id = 1;
        assert!(!verify_witness(&tampered.signing_payload(), &witness).unwrap());
    }

    // ── 4. Wrong witness_root fails ──────────────────────────────────────

    #[test]
    fn wrong_witness_root_fails() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let tx_root = compute_tx_root(&[core.clone()]);
        let bad_witness_root = [0xFFu8; 32];

        let err =
            validate_segwit_block(&[core], &[witness], &tx_root, &bad_witness_root).unwrap_err();
        assert!(matches!(err, SegwitValidationError::WitnessRootMismatch));
    }

    // ── 5. Wrong tx_root fails ───────────────────────────────────────────

    #[test]
    fn wrong_tx_root_fails() {
        let provider = SoftwareSigningProvider::generate();
        let (core, witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let bad_tx_root = [0xFFu8; 32];
        let witness_root = compute_witness_root(&[witness.clone()]);

        let err =
            validate_segwit_block(&[core], &[witness], &bad_tx_root, &witness_root).unwrap_err();
        assert!(matches!(err, SegwitValidationError::TxRootMismatch));
    }

    // ── 6. Length mismatch fails ─────────────────────────────────────────

    #[test]
    fn length_mismatch_fails() {
        let provider = SoftwareSigningProvider::generate();
        let (core, _witness) = make_signed_pair(&provider, "alice", "bob", 100, 5, 0, 1);

        let tx_root = compute_tx_root(&[core.clone()]);
        let witness_root = compute_witness_root(&[]);

        let err = validate_segwit_block(&[core], &[], &tx_root, &witness_root).unwrap_err();
        assert!(matches!(
            err,
            SegwitValidationError::LengthMismatch {
                cores: 1,
                witnesses: 0
            }
        ));
    }

    // ── 7. Legacy NativeTransaction -> TxCore + TxWitness roundtrip ──────

    #[test]
    fn legacy_conversion_preserves_valid_signature() {
        let provider = SoftwareSigningProvider::generate();
        let pk = provider.public_key();

        let mut tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 100, 0, 5, 9999);
        let payload = tx.signing_payload();
        tx.signature = provider.sign(&payload).unwrap();
        tx.signature_algorithm = "ed25519".to_string();

        let (core, witness) = tx.to_segwit(pk);

        // The core's signing_payload must match the original tx's signing_payload
        assert_eq!(core.signing_payload(), tx.signing_payload());

        // The witness must verify against the core
        assert!(verify_witness(&core.signing_payload(), &witness).unwrap());

        // Full block validation
        let tx_root = compute_tx_root(&[core.clone()]);
        let witness_root = compute_witness_root(&[witness.clone()]);
        assert!(validate_segwit_block(&[core], &[witness], &tx_root, &witness_root).is_ok());
    }
}
