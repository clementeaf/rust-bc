//! Cross-version replay protection for SegWit/PQC transactions.
//!
//! Prevents a transaction signed for one block version (e.g. Legacy) from
//! being accepted in another version (e.g. SegWitPqcV1) by binding the
//! signing payload to a domain separator and version byte.
//!
//! Legacy blocks continue using `TxCore::signing_payload()` (unchanged).
//! SegWitPqcV1 blocks use `signing_payload_for_version()` which prepends
//! a domain separator and version tag.

use crate::transaction::block_version::BlockVersion;
use crate::transaction::native::TransactionKind;
use crate::transaction::segwit::{SegwitValidationError, TxCore, TxWitness};

// ── Domain Separator ─────────────────────────────────────────────────────

/// Domain separator for SegWitPqcV1 transaction signing.
pub const SEGWIT_PQC_V1_DOMAIN: &[u8] = b"RUST_BC_SEGWIT_PQC_V1_TX";

// ── Versioned Signing Payload ────────────────────────────────────────────

/// Compute the signing payload for a transaction bound to a specific block version.
///
/// - `Legacy`: uses the original canonical JSON format (backward compatible).
/// - `SegWitPqcV1`: prepends domain separator + version byte to prevent replay.
pub fn signing_payload_for_version(core: &TxCore, version: BlockVersion) -> Vec<u8> {
    match version {
        BlockVersion::Legacy => core.signing_payload(),
        BlockVersion::SegWitPqcV1 => {
            let kind = core.kind.clone().unwrap_or(TransactionKind::Transfer {
                from: core.from.clone(),
                to: core.to.clone(),
                amount: core.amount,
            });
            let canonical = serde_json::json!({
                "chain_id": core.chain_id,
                "kind": kind,
                "nonce": core.nonce,
                "fee": core.fee,
                "timestamp": core.timestamp,
            });
            let core_bytes = canonical.to_string().into_bytes();

            let mut payload = Vec::with_capacity(SEGWIT_PQC_V1_DOMAIN.len() + 1 + core_bytes.len());
            payload.extend_from_slice(SEGWIT_PQC_V1_DOMAIN);
            payload.push(version as u8);
            payload.extend_from_slice(&core_bytes);
            payload
        }
    }
}

// ── Versioned Witness Verification ───────────────────────────────────────

/// Verify a witness using the version-aware signing payload.
pub fn verify_witness_versioned(
    core: &TxCore,
    witness: &TxWitness,
    version: BlockVersion,
) -> Result<bool, SegwitValidationError> {
    let payload = signing_payload_for_version(core, version);
    crate::transaction::segwit::verify_witness(&payload, witness)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
    use crate::transaction::block_version::{
        validate_block_versioned, AnyBlock, BlockVersion, ChainConfig,
    };
    use crate::transaction::compact_block::{CompactBlockHeader, SegWitBlock};
    use crate::transaction::pqc_validation::PqcValidationConfig;
    use crate::transaction::segwit::{compute_tx_root, compute_witness_root, TxCore, TxWitness};
    use crate::transaction::verification_cache::VerificationCache;

    /// Sign a TxCore with the versioned payload.
    fn sign_versioned(
        provider: &dyn SigningProvider,
        core: &TxCore,
        version: BlockVersion,
    ) -> TxWitness {
        let payload = signing_payload_for_version(core, version);
        let sig = provider.sign(&payload).unwrap();
        TxWitness {
            signature: sig,
            public_key: provider.public_key(),
            signature_scheme: provider.algorithm(),
        }
    }

    fn make_core(idx: usize, fee: u64) -> TxCore {
        TxCore {
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
        }
    }

    // ── 1. SegWitPqcV1 tx valid with versioned payload ──────────────────

    #[test]
    fn segwit_v1_valid_with_versioned_payload() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);
        let witness = sign_versioned(&provider, &core, BlockVersion::SegWitPqcV1);

        assert!(verify_witness_versioned(&core, &witness, BlockVersion::SegWitPqcV1).unwrap());
    }

    // ── 2. Same signature fails if verified as Legacy ────────────────────

    #[test]
    fn segwit_sig_fails_as_legacy() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);
        let witness = sign_versioned(&provider, &core, BlockVersion::SegWitPqcV1);

        // Verify with Legacy payload — must fail
        assert!(!verify_witness_versioned(&core, &witness, BlockVersion::Legacy).unwrap());
    }

    // ── 3. Same signature fails if BlockVersion changed ──────────────────

    #[test]
    fn legacy_sig_fails_as_segwit() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);
        // Sign with Legacy payload
        let witness = sign_versioned(&provider, &core, BlockVersion::Legacy);

        // Verify with SegWitPqcV1 payload — must fail
        assert!(!verify_witness_versioned(&core, &witness, BlockVersion::SegWitPqcV1).unwrap());
    }

    // ── 4. Changing chain_id fails ───────────────────────────────────────

    #[test]
    fn changing_chain_id_fails() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);
        let witness = sign_versioned(&provider, &core, BlockVersion::SegWitPqcV1);

        let mut tampered = core;
        tampered.chain_id = 9999;
        assert!(!verify_witness_versioned(&tampered, &witness, BlockVersion::SegWitPqcV1).unwrap());
    }

    // ── 5. Changing nonce fails ──────────────────────────────────────────

    #[test]
    fn changing_nonce_fails() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);
        let witness = sign_versioned(&provider, &core, BlockVersion::SegWitPqcV1);

        let mut tampered = core;
        tampered.nonce = 99;
        assert!(!verify_witness_versioned(&tampered, &witness, BlockVersion::SegWitPqcV1).unwrap());
    }

    // ── 6. Changing fee fails ────────────────────────────────────────────

    #[test]
    fn changing_fee_fails() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);
        let witness = sign_versioned(&provider, &core, BlockVersion::SegWitPqcV1);

        let mut tampered = core;
        tampered.fee = 0;
        assert!(!verify_witness_versioned(&tampered, &witness, BlockVersion::SegWitPqcV1).unwrap());
    }

    // ── 7. Different domain separator fails ──────────────────────────────

    #[test]
    fn different_domain_separator_fails() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);

        // Sign with a fake "wrong" domain by manually constructing payload
        let kind = core.kind.clone().unwrap();
        let canonical = serde_json::json!({
            "chain_id": core.chain_id,
            "kind": kind,
            "nonce": core.nonce,
            "fee": core.fee,
            "timestamp": core.timestamp,
        });
        let core_bytes = canonical.to_string().into_bytes();
        let mut wrong_payload = Vec::new();
        wrong_payload.extend_from_slice(b"WRONG_DOMAIN_SEPARATOR_XX");
        wrong_payload.push(BlockVersion::SegWitPqcV1 as u8);
        wrong_payload.extend_from_slice(&core_bytes);

        let sig = provider.sign(&wrong_payload).unwrap();
        let witness = TxWitness {
            signature: sig,
            public_key: provider.public_key(),
            signature_scheme: provider.algorithm(),
        };

        // Must fail with correct domain
        assert!(!verify_witness_versioned(&core, &witness, BlockVersion::SegWitPqcV1).unwrap());
    }

    // ── 8. Versioned block validation still passes ─────────────────��─────

    #[test]
    fn versioned_block_validation_passes() {
        // Build a SegWit block using legacy signing (current pipeline)
        let providers: Vec<SoftwareSigningProvider> = (0..3)
            .map(|_| SoftwareSigningProvider::generate())
            .collect();
        let (cores, witnesses): (Vec<_>, Vec<_>) = providers
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let core = make_core(i, 5000);
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

        let block = SegWitBlock {
            header: CompactBlockHeader {
                height: 100,
                hash: tx_root,
                parent_hash: [0u8; 32],
                timestamp: 1000,
                proposer: "v".into(),
            },
            tx_cores: cores,
            witnesses,
            tx_root,
            witness_root,
        };

        let any = AnyBlock::SegWit(block);
        let mut cache = VerificationCache::new(100);
        let config = PqcValidationConfig {
            enforce_fees: false,
            use_cache: true,
            parallel_verify: true,
        };
        let chain = ChainConfig {
            segwit_pqc_activation_height: 100,
        };

        assert!(validate_block_versioned(&any, &mut cache, &config, &chain).is_ok());
    }

    // ── 9. Legacy tests still pass ───────────────────────────────────────

    #[test]
    fn legacy_signing_still_works() {
        let provider = SoftwareSigningProvider::generate();
        let core = make_core(0, 5000);

        // Sign with legacy payload
        let witness = sign_versioned(&provider, &core, BlockVersion::Legacy);

        // Verify with legacy — passes
        assert!(verify_witness_versioned(&core, &witness, BlockVersion::Legacy).unwrap());

        // The legacy payload matches TxCore::signing_payload()
        let legacy_payload = signing_payload_for_version(&core, BlockVersion::Legacy);
        assert_eq!(legacy_payload, core.signing_payload());
    }
}
