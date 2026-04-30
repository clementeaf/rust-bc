//! Canonical binary serialization for consensus-critical data.
//!
//! Replaces `serde_json` as the serialization format for signing payloads,
//! Merkle roots, cache keys, and short IDs. The encoding is:
//!
//! - Deterministic: same input always produces same bytes
//! - Explicit: field order is defined by the trait implementation, not struct layout
//! - Compact: no delimiters, no key names, no whitespace
//! - Versionable: enums use explicit `u8` discriminants
//!
//! ## Primitive encoding rules
//!
//! | Type     | Encoding                          |
//! |----------|-----------------------------------|
//! | `u8`     | 1 byte                            |
//! | `u32`    | 4 bytes little-endian             |
//! | `u64`    | 8 bytes little-endian             |
//! | `bytes`  | u32 length prefix + raw bytes     |
//! | `string` | u32 length prefix + UTF-8 bytes   |
//! | `enum`   | u8 discriminant                   |
//! | `Option` | 0x00 (None) or 0x01 + value (Some)|

use crate::identity::signing::SigningAlgorithm;
use crate::transaction::block_version::BlockVersion;
use crate::transaction::native::TransactionKind;
use crate::transaction::segwit::{TxCore, TxWitness};

// ── Trait ─────────────────────────────────────────────────────────────────

/// Deterministic binary encoding for consensus-critical types.
pub trait CanonicalEncode {
    fn encode_canonical(&self, out: &mut Vec<u8>);

    fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_canonical(&mut out);
        out
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn encode_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

fn encode_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn encode_bytes(out: &mut Vec<u8>, v: &[u8]) {
    out.extend_from_slice(&(v.len() as u32).to_le_bytes());
    out.extend_from_slice(v);
}

fn encode_string(out: &mut Vec<u8>, v: &str) {
    encode_bytes(out, v.as_bytes());
}

// ── SigningAlgorithm ─────────────────────────────────────────────────────

impl CanonicalEncode for SigningAlgorithm {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        match self {
            Self::Ed25519 => encode_u8(out, 0),
            Self::MlDsa65 => encode_u8(out, 1),
        }
    }
}

// ── BlockVersion ─────────────────────────────────────────────────────────

impl CanonicalEncode for BlockVersion {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_u8(out, *self as u8);
    }
}

// ── TransactionKind ──────────────────────────────────────────────────────

impl CanonicalEncode for TransactionKind {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        match self {
            Self::Transfer { from, to, amount } => {
                encode_u8(out, 0);
                encode_string(out, from);
                encode_string(out, to);
                encode_u64(out, *amount);
            }
            Self::Coinbase { to, amount } => {
                encode_u8(out, 1);
                encode_string(out, to);
                encode_u64(out, *amount);
            }
        }
    }
}

// ── TxCore ───────────────────────────────────────────────────────────────

impl CanonicalEncode for TxCore {
    /// Fields encoded in fixed order:
    /// from, to, amount, fee, nonce, chain_id, timestamp, kind
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_string(out, &self.from);
        encode_string(out, &self.to);
        encode_u64(out, self.amount);
        encode_u64(out, self.fee);
        encode_u64(out, self.nonce);
        encode_u64(out, self.chain_id);
        encode_u64(out, self.timestamp);
        // Option<TransactionKind>
        match &self.kind {
            None => encode_u8(out, 0x00),
            Some(kind) => {
                encode_u8(out, 0x01);
                kind.encode_canonical(out);
            }
        }
    }
}

// ── TxWitness ────────────────────────────────────────────────────────────

impl CanonicalEncode for TxWitness {
    /// Fields encoded in fixed order:
    /// signature, public_key, signature_scheme
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_bytes(out, &self.signature);
        encode_bytes(out, &self.public_key);
        self.signature_scheme.encode_canonical(out);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_core() -> TxCore {
        TxCore {
            from: "alice".into(),
            to: "bob".into(),
            amount: 100,
            fee: 5,
            nonce: 0,
            chain_id: 1,
            timestamp: 1000,
            kind: Some(TransactionKind::Transfer {
                from: "alice".into(),
                to: "bob".into(),
                amount: 100,
            }),
        }
    }

    fn sample_witness() -> TxWitness {
        TxWitness {
            signature: vec![0xAA; 64],
            public_key: vec![0xBB; 32],
            signature_scheme: SigningAlgorithm::Ed25519,
        }
    }

    // ── 1. Same TxCore always produces same bytes ────────────────────────

    #[test]
    fn deterministic_txcore() {
        let core = sample_core();
        let b1 = core.to_canonical_bytes();
        let b2 = core.to_canonical_bytes();
        assert_eq!(b1, b2);
    }

    // ── 2. Struct field reordering cannot alter encoding ─────────────────

    #[test]
    fn field_order_independent_by_design() {
        // Encoding order is defined by encode_canonical, not struct definition.
        // Two cores with same data must produce identical bytes regardless of
        // how the struct was constructed.
        let core1 = TxCore {
            from: "a".into(),
            to: "b".into(),
            amount: 1,
            fee: 2,
            nonce: 3,
            chain_id: 4,
            timestamp: 5,
            kind: None,
        };
        let core2 = TxCore {
            amount: 1,
            fee: 2,
            from: "a".into(),
            nonce: 3,
            to: "b".into(),
            chain_id: 4,
            timestamp: 5,
            kind: None,
        };
        assert_eq!(core1.to_canonical_bytes(), core2.to_canonical_bytes());
    }

    // ── 3–8. Changing each field changes bytes ───────────────────────────

    #[test]
    fn changing_amount_changes_bytes() {
        let mut c = sample_core();
        let b1 = c.to_canonical_bytes();
        c.amount = 999;
        assert_ne!(b1, c.to_canonical_bytes());
    }

    #[test]
    fn changing_fee_changes_bytes() {
        let mut c = sample_core();
        let b1 = c.to_canonical_bytes();
        c.fee = 999;
        assert_ne!(b1, c.to_canonical_bytes());
    }

    #[test]
    fn changing_nonce_changes_bytes() {
        let mut c = sample_core();
        let b1 = c.to_canonical_bytes();
        c.nonce = 999;
        assert_ne!(b1, c.to_canonical_bytes());
    }

    #[test]
    fn changing_chain_id_changes_bytes() {
        let mut c = sample_core();
        let b1 = c.to_canonical_bytes();
        c.chain_id = 999;
        assert_ne!(b1, c.to_canonical_bytes());
    }

    #[test]
    fn changing_timestamp_changes_bytes() {
        let mut c = sample_core();
        let b1 = c.to_canonical_bytes();
        c.timestamp = 999;
        assert_ne!(b1, c.to_canonical_bytes());
    }

    #[test]
    fn changing_kind_changes_bytes() {
        let mut c = sample_core();
        let b1 = c.to_canonical_bytes();
        c.kind = Some(TransactionKind::Coinbase {
            to: "bob".into(),
            amount: 100,
        });
        assert_ne!(b1, c.to_canonical_bytes());
    }

    // ── 9–11. Witness field changes ──────────────────────────────────────

    #[test]
    fn changing_signature_scheme_changes_witness_bytes() {
        let mut w = sample_witness();
        let b1 = w.to_canonical_bytes();
        w.signature_scheme = SigningAlgorithm::MlDsa65;
        assert_ne!(b1, w.to_canonical_bytes());
    }

    #[test]
    fn changing_signature_changes_witness_bytes() {
        let mut w = sample_witness();
        let b1 = w.to_canonical_bytes();
        w.signature = vec![0xFF; 64];
        assert_ne!(b1, w.to_canonical_bytes());
    }

    #[test]
    fn changing_public_key_changes_witness_bytes() {
        let mut w = sample_witness();
        let b1 = w.to_canonical_bytes();
        w.public_key = vec![0xFF; 32];
        assert_ne!(b1, w.to_canonical_bytes());
    }

    // ── 15. No JSON delimiters ───────────────────────────────────────────

    #[test]
    fn canonical_encoding_is_not_json() {
        let core = sample_core();
        let bytes = core.to_canonical_bytes();
        let as_str = String::from_utf8_lossy(&bytes);
        assert!(!as_str.contains('{'));
        assert!(!as_str.contains('}'));
        assert!(!as_str.contains('"'));
        assert!(!as_str.contains(':'));
    }

    // ── Stability: encoding is stable across calls ───────────────────────

    #[test]
    fn canonical_encoding_is_stable() {
        let core = sample_core();
        let witness = sample_witness();

        let cb1 = core.to_canonical_bytes();
        let cb2 = core.to_canonical_bytes();
        assert_eq!(cb1, cb2);

        let wb1 = witness.to_canonical_bytes();
        let wb2 = witness.to_canonical_bytes();
        assert_eq!(wb1, wb2);
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  F-002 Audit Regression Tests
    // ═══════════════════════════════════════════════════════════════════════

    // ── 12. Cache key changes if core or witness changes ─────────────────

    #[test]
    fn cache_key_changes_on_core_or_witness_change() {
        use crate::transaction::verification_cache::cache_key_for;
        let core = sample_core();
        let witness = sample_witness();
        let k1 = cache_key_for(&core, &witness);

        let mut c2 = core.clone();
        c2.amount = 999;
        assert_ne!(k1, cache_key_for(&c2, &witness));

        let mut w2 = witness.clone();
        w2.signature = vec![0xFF; 64];
        assert_ne!(k1, cache_key_for(&core, &w2));
    }

    // ── 13. Short ID changes if core or witness changes ──────────────────

    #[test]
    fn short_id_changes_on_core_or_witness_change() {
        use crate::transaction::compact_block::{short_id_tx_core, short_id_witness};
        let core = sample_core();
        let witness = sample_witness();
        let sid_c = short_id_tx_core(&core);
        let sid_w = short_id_witness(&witness);

        let mut c2 = core;
        c2.fee = 999;
        assert_ne!(sid_c, short_id_tx_core(&c2));

        let mut w2 = witness;
        w2.public_key = vec![0xFF; 32];
        assert_ne!(sid_w, short_id_witness(&w2));
    }

    // ── 14. Roots change if core or witness changes ──────────────────────

    #[test]
    fn roots_change_on_core_or_witness_change() {
        use crate::transaction::segwit::{compute_tx_root, compute_witness_root};
        let core = sample_core();
        let witness = sample_witness();
        let r1 = compute_tx_root(&[core.clone()]);
        let wr1 = compute_witness_root(&[witness.clone()]);

        let mut c2 = core;
        c2.nonce = 42;
        assert_ne!(r1, compute_tx_root(&[c2]));

        let mut w2 = witness;
        w2.signature_scheme = SigningAlgorithm::MlDsa65;
        assert_ne!(wr1, compute_witness_root(&[w2]));
    }

    // ── F-002 regression: signing payload uses canonical binary ──────────

    #[test]
    fn signing_payload_uses_canonical_binary_for_segwit_pqc_v1() {
        use crate::transaction::block_version::BlockVersion;
        use crate::transaction::replay_protection::{
            signing_payload_for_version, SEGWIT_PQC_V1_DOMAIN,
        };
        let core = sample_core();
        let payload = signing_payload_for_version(&core, BlockVersion::SegWitPqcV1);

        // Must start with domain separator
        assert!(payload.starts_with(SEGWIT_PQC_V1_DOMAIN));
        // Must not contain JSON delimiters
        let as_str = String::from_utf8_lossy(&payload);
        assert!(!as_str.contains('{'));
        assert!(!as_str.contains('"'));
    }

    #[test]
    fn merkle_roots_use_canonical_binary() {
        use crate::transaction::segwit::{compute_tx_root, compute_witness_root};
        // Roots are computed from canonical bytes, not JSON.
        // Verify by checking that the root differs from a JSON-based root.
        let core = sample_core();
        let canonical_root = compute_tx_root(&[core.clone()]);
        // Compute a "json root" manually
        let json_bytes = serde_json::to_vec(&core).unwrap();
        use crate::crypto::hasher::{hash_with, HashAlgorithm};
        let json_hash = hash_with(HashAlgorithm::Sha256, &json_bytes);
        // They must differ (canonical != JSON serialization)
        assert_ne!(canonical_root, json_hash);
    }

    #[test]
    fn cache_keys_use_canonical_binary() {
        use crate::transaction::verification_cache::cache_key_for;
        let core = sample_core();
        let witness = sample_witness();
        let key = cache_key_for(&core, &witness);

        // A JSON-based key would be different
        use crate::crypto::hasher::{hash_with, HashAlgorithm};
        let json_core = serde_json::to_vec(&core).unwrap();
        let json_witness = serde_json::to_vec(&witness).unwrap();
        let mut json_combined = json_core;
        json_combined.extend_from_slice(&json_witness);
        let json_key = hash_with(HashAlgorithm::Sha256, &json_combined);
        assert_ne!(key, json_key);
    }

    #[test]
    fn compact_short_ids_use_canonical_binary() {
        use crate::transaction::compact_block::short_id_tx_core;
        let core = sample_core();
        let sid = short_id_tx_core(&core);

        // A JSON-based short ID would be different
        use crate::crypto::hasher::{hash_with, HashAlgorithm};
        let json_bytes = serde_json::to_vec(&core).unwrap();
        let json_hash = hash_with(HashAlgorithm::Sha3_256, &json_bytes);
        let mut json_sid = [0u8; 8];
        json_sid.copy_from_slice(&json_hash[..8]);
        assert_ne!(sid, json_sid);
    }
}
