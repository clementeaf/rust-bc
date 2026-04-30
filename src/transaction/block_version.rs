//! Block versioning and version-aware validation for SegWit/PQC migration.
//!
//! Introduces `BlockVersion` to distinguish legacy blocks from SegWit/PQC blocks,
//! `ChainConfig` with an activation height, and `validate_block_versioned` which
//! routes to the appropriate validation pipeline based on version and height.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::transaction::compact_block::SegWitBlock;
use crate::transaction::pqc_validation::{validate_pqc_block, PqcBlockError, PqcValidationConfig};
use crate::transaction::segwit::{compute_tx_root, TxCore};
use crate::transaction::verification_cache::VerificationCache;

// ── Block Version ────────────────────────────────────────────────────────

/// Block format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlockVersion {
    /// Pre-SegWit/PQC: no witnesses, no weight-based fees, Ed25519 only.
    Legacy = 0,
    /// SegWit/PQC v1: dual Merkle roots, witnesses required, weight-based fees.
    SegWitPqcV1 = 1,
}

// ── Versioned Header ─────────────────────────────────────────────────────

/// Block header with explicit version and optional witness_root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionedBlockHeader {
    pub version: BlockVersion,
    pub height: u64,
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
    pub proposer: String,
    pub tx_root: [u8; 32],
    /// `None` for Legacy, `Some(...)` for SegWitPqcV1.
    pub witness_root: Option<[u8; 32]>,
}

impl VersionedBlockHeader {
    /// Compute a hash incorporating the version field.
    pub fn compute_hash(&self) -> [u8; 32] {
        use pqc_crypto_module::legacy::sha256::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update([self.version as u8]);
        hasher.update(self.height.to_le_bytes());
        hasher.update(self.parent_hash);
        hasher.update(self.tx_root);
        if let Some(wr) = &self.witness_root {
            hasher.update(wr);
        }
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.proposer.as_bytes());
        hasher.finalize().into()
    }
}

// ── Legacy Block ─────────────────────────────────────────────────────────

/// A pre-SegWit block: transactions only, no separate witnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyBlock {
    pub header: VersionedBlockHeader,
    pub tx_cores: Vec<TxCore>,
    pub tx_root: [u8; 32],
}

// ── AnyBlock ─────────────────────────────────────────────────────────────

/// Discriminated union of supported block versions.
#[derive(Debug, Clone)]
pub enum AnyBlock {
    Legacy(LegacyBlock),
    SegWit(SegWitBlock),
}

impl AnyBlock {
    pub fn height(&self) -> u64 {
        match self {
            Self::Legacy(b) => b.header.height,
            Self::SegWit(b) => b.header.height,
        }
    }

    pub fn version(&self) -> BlockVersion {
        match self {
            Self::Legacy(_) => BlockVersion::Legacy,
            Self::SegWit(_) => BlockVersion::SegWitPqcV1,
        }
    }
}

// ── Chain Config ─────────────────────────────────────────────────────────

/// Chain-level configuration for block version activation.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Height at which SegWit/PQC blocks become mandatory.
    /// Before this height: only Legacy allowed.
    /// At and after this height: only SegWitPqcV1 allowed.
    pub segwit_pqc_activation_height: u64,
}

// ── Errors ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum BlockVersionError {
    #[error("legacy block not allowed at height {height} (activation at {activation})")]
    LegacyAfterActivation { height: u64, activation: u64 },
    #[error("segwit block not allowed at height {height} (activation at {activation})")]
    SegWitBeforeActivation { height: u64, activation: u64 },
    #[error("legacy block must not have witness_root")]
    LegacyWithWitnessRoot,
    #[error("segwit block must have witness_root in header")]
    SegWitWithoutWitnessRoot,
    #[error("tx_root mismatch in legacy block")]
    LegacyTxRootMismatch,
    #[error("pqc validation failed: {0}")]
    Pqc(#[from] PqcBlockError),
}

// ── Version-Aware Validation ─────────────────────────────────────────────

/// Validate a block according to its version and the chain's activation rules.
pub fn validate_block_versioned(
    block: &AnyBlock,
    cache: &mut VerificationCache,
    config: &PqcValidationConfig,
    chain: &ChainConfig,
) -> Result<(), BlockVersionError> {
    let height = block.height();

    match block {
        AnyBlock::Legacy(legacy) => {
            // Reject legacy after activation
            if height >= chain.segwit_pqc_activation_height {
                return Err(BlockVersionError::LegacyAfterActivation {
                    height,
                    activation: chain.segwit_pqc_activation_height,
                });
            }
            // Legacy must not have witness_root
            if legacy.header.witness_root.is_some() {
                return Err(BlockVersionError::LegacyWithWitnessRoot);
            }
            // Validate tx_root
            let computed = compute_tx_root(&legacy.tx_cores);
            if computed != legacy.tx_root {
                return Err(BlockVersionError::LegacyTxRootMismatch);
            }
            Ok(())
        }
        AnyBlock::SegWit(segwit) => {
            // Reject segwit before activation
            if height < chain.segwit_pqc_activation_height {
                return Err(BlockVersionError::SegWitBeforeActivation {
                    height,
                    activation: chain.segwit_pqc_activation_height,
                });
            }
            // Full PQC pipeline
            validate_pqc_block(segwit, cache, config)?;
            Ok(())
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{SigningProvider, SoftwareSigningProvider};
    use crate::transaction::compact_block::CompactBlockHeader;
    use crate::transaction::native::TransactionKind;
    use crate::transaction::segwit::{compute_tx_root, compute_witness_root, TxCore, TxWitness};

    fn make_segwit_block(n: usize, height: u64, fee: u64) -> SegWitBlock {
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
                    amount: 100,
                    fee,
                    nonce: 0,
                    chain_id: 1,
                    timestamp: 1000,
                    kind: Some(TransactionKind::Transfer {
                        from: format!("s{i}"),
                        to: format!("r{i}"),
                        amount: 100,
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

    fn make_legacy_block(n: usize, height: u64) -> LegacyBlock {
        let cores: Vec<TxCore> = (0..n)
            .map(|i| TxCore {
                from: format!("s{i}"),
                to: format!("r{i}"),
                amount: 100,
                fee: 5,
                nonce: 0,
                chain_id: 1,
                timestamp: 1000,
                kind: Some(TransactionKind::Transfer {
                    from: format!("s{i}"),
                    to: format!("r{i}"),
                    amount: 100,
                }),
            })
            .collect();

        let tx_root = compute_tx_root(&cores);

        LegacyBlock {
            header: VersionedBlockHeader {
                version: BlockVersion::Legacy,
                height,
                hash: [0u8; 32],
                parent_hash: [0u8; 32],
                timestamp: 1000,
                proposer: "validator".into(),
                tx_root,
                witness_root: None,
            },
            tx_cores: cores,
            tx_root,
        }
    }

    fn chain_config(activation: u64) -> ChainConfig {
        ChainConfig {
            segwit_pqc_activation_height: activation,
        }
    }

    fn full_config() -> PqcValidationConfig {
        PqcValidationConfig {
            enforce_fees: false, // legacy blocks don't need fees for these tests
            use_cache: true,
            parallel_verify: true,
        }
    }

    // ── 1. Legacy valid before activation ────────────────────────────────

    #[test]
    fn legacy_valid_before_activation() {
        let block = make_legacy_block(3, 5);
        let any = AnyBlock::Legacy(block);
        let mut cache = VerificationCache::new(100);

        assert!(
            validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100)).is_ok()
        );
    }

    // ── 2. SegWit rejected before activation ─────────────────────────────

    #[test]
    fn segwit_rejected_before_activation() {
        let block = make_segwit_block(3, 5, 5000);
        let any = AnyBlock::SegWit(block);
        let mut cache = VerificationCache::new(100);

        let err = validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100))
            .unwrap_err();
        assert!(matches!(
            err,
            BlockVersionError::SegWitBeforeActivation { .. }
        ));
    }

    // ── 3. SegWit valid after activation ─────────────────────────────────

    #[test]
    fn segwit_valid_after_activation() {
        let block = make_segwit_block(3, 100, 5000);
        let any = AnyBlock::SegWit(block);
        let mut cache = VerificationCache::new(100);

        assert!(
            validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100)).is_ok()
        );
    }

    // ── 4. Legacy rejected after activation ──────────────────────────────

    #[test]
    fn legacy_rejected_after_activation() {
        let block = make_legacy_block(3, 100);
        let any = AnyBlock::Legacy(block);
        let mut cache = VerificationCache::new(100);

        let err = validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100))
            .unwrap_err();
        assert!(matches!(
            err,
            BlockVersionError::LegacyAfterActivation { .. }
        ));
    }

    // ── 5. SegWit without witness_root fails ─────────────────────────────
    // (Tested via missing witnesses → LengthMismatch in pqc pipeline)

    #[test]
    fn segwit_without_witnesses_fails() {
        let mut block = make_segwit_block(3, 100, 5000);
        block.witnesses.clear();
        let any = AnyBlock::SegWit(block);
        let mut cache = VerificationCache::new(100);

        let err = validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100))
            .unwrap_err();
        assert!(matches!(err, BlockVersionError::Pqc(_)));
    }

    // ── 6. Legacy with witness_root fails ────────────────────────────────

    #[test]
    fn legacy_with_witness_root_fails() {
        let mut block = make_legacy_block(3, 5);
        block.header.witness_root = Some([1u8; 32]);
        let any = AnyBlock::Legacy(block);
        let mut cache = VerificationCache::new(100);

        let err = validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100))
            .unwrap_err();
        assert!(matches!(err, BlockVersionError::LegacyWithWitnessRoot));
    }

    // ── 7. validate_block_versioned routes correctly ─────────────────────

    #[test]
    fn routes_correctly() {
        let legacy = AnyBlock::Legacy(make_legacy_block(2, 5));
        let segwit = AnyBlock::SegWit(make_segwit_block(2, 200, 5000));
        let mut cache = VerificationCache::new(100);
        let chain = chain_config(100);

        assert_eq!(legacy.version(), BlockVersion::Legacy);
        assert_eq!(segwit.version(), BlockVersion::SegWitPqcV1);

        assert!(validate_block_versioned(&legacy, &mut cache, &full_config(), &chain).is_ok());
        assert!(validate_block_versioned(&segwit, &mut cache, &full_config(), &chain).is_ok());
    }

    // ── 8. Changing version invalidates block ────────────────────────────
    // (A legacy block at post-activation height fails, and vice versa)

    #[test]
    fn wrong_version_for_height_fails() {
        let chain = chain_config(50);
        let mut cache = VerificationCache::new(100);

        // Legacy at height 50 (activation height) → rejected
        let legacy = AnyBlock::Legacy(make_legacy_block(2, 50));
        assert!(validate_block_versioned(&legacy, &mut cache, &full_config(), &chain).is_err());

        // SegWit at height 49 → rejected
        let segwit = AnyBlock::SegWit(make_segwit_block(2, 49, 5000));
        assert!(validate_block_versioned(&segwit, &mut cache, &full_config(), &chain).is_err());
    }

    // ── 9. Block hash changes if version changes ─────────────────────────

    #[test]
    fn hash_changes_with_version() {
        let header_legacy = VersionedBlockHeader {
            version: BlockVersion::Legacy,
            height: 1,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            timestamp: 1000,
            proposer: "v".into(),
            tx_root: [1u8; 32],
            witness_root: None,
        };

        let header_segwit = VersionedBlockHeader {
            version: BlockVersion::SegWitPqcV1,
            height: 1,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            timestamp: 1000,
            proposer: "v".into(),
            tx_root: [1u8; 32],
            witness_root: Some([2u8; 32]),
        };

        let h1 = header_legacy.compute_hash();
        let h2 = header_segwit.compute_hash();
        assert_ne!(h1, h2);
    }

    // ── 10. Mixing structures fails ──────────────────────────────────────
    // (Legacy block with corrupted tx_root fails validation)

    #[test]
    fn corrupted_structure_fails() {
        let mut block = make_legacy_block(3, 5);
        block.tx_root = [0xFFu8; 32]; // corrupt
        let any = AnyBlock::Legacy(block);
        let mut cache = VerificationCache::new(100);

        let err = validate_block_versioned(&any, &mut cache, &full_config(), &chain_config(100))
            .unwrap_err();
        assert!(matches!(err, BlockVersionError::LegacyTxRootMismatch));
    }
}
