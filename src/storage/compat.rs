//! Compatibility layer between legacy types (`blockchain::Block`, `models::Transaction`)
//! and new storage types (`storage::traits::Block`, `storage::traits::Transaction`).
//!
//! These conversions enable gradual migration of handlers from the legacy
//! `Blockchain` struct to the `BlockStore` trait without a big-bang rewrite.

use crate::blockchain::Block as LegacyBlock;
use crate::models::Transaction as LegacyTransaction;
use crate::storage::traits::Block as StoreBlock;
use crate::storage::traits::Transaction as StoreTransaction;

use crate::crypto::hasher::HashAlgorithm;
use crate::identity::signing::SigningAlgorithm;

/// Convert a legacy block to a storage block.
///
/// Lossy: nonce and difficulty are dropped (not relevant to new consensus).
/// `transactions` field becomes a list of transaction IDs (not full objects).
impl From<&LegacyBlock> for StoreBlock {
    fn from(legacy: &LegacyBlock) -> Self {
        let parent_hash = hex_to_bytes32(&legacy.previous_hash);
        let merkle_root = hex_to_bytes32(&legacy.merkle_root);
        let tx_ids: Vec<String> = legacy.transactions.iter().map(|tx| tx.id.clone()).collect();

        StoreBlock {
            height: legacy.index,
            timestamp: legacy.timestamp,
            parent_hash,
            merkle_root,
            transactions: tx_ids,
            proposer: String::new(),
            signature: Vec::new(),
            signature_algorithm: SigningAlgorithm::default(),
            endorsements: Vec::new(),
            secondary_signature: None,
            secondary_signature_algorithm: None,
            hash_algorithm: HashAlgorithm::default(),
            orderer_signature: None,
        }
    }
}

/// Convert a legacy transaction to a storage transaction.
///
/// Field mapping: from → input_did, to → output_recipient.
/// Fee, data, and signature are dropped (not in storage schema).
impl From<&LegacyTransaction> for StoreTransaction {
    fn from(legacy: &LegacyTransaction) -> Self {
        StoreTransaction {
            id: legacy.id.clone(),
            block_height: 0, // Must be set by caller when associating with a block
            timestamp: legacy.timestamp,
            input_did: legacy.from.clone(),
            output_recipient: legacy.to.clone(),
            amount: legacy.amount,
            state: "confirmed".to_string(),
        }
    }
}

/// Convert a storage transaction back to a legacy transaction.
///
/// Fields not in storage (fee, data, signature) default to zero/empty.
impl From<&StoreTransaction> for LegacyTransaction {
    fn from(store: &StoreTransaction) -> Self {
        LegacyTransaction::new_with_fee(
            store.input_did.clone(),
            store.output_recipient.clone(),
            store.amount,
            0,
            None,
        )
    }
}

/// Parse a hex string into [u8; 32], zero-padding if too short.
fn hex_to_bytes32(hex_str: &str) -> [u8; 32] {
    let mut result = [0u8; 32];
    if let Ok(bytes) = hex::decode(hex_str) {
        let len = bytes.len().min(32);
        result[..len].copy_from_slice(&bytes[..len]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_transaction_to_store_roundtrip() {
        let legacy = LegacyTransaction::new_with_fee(
            "alice".to_string(),
            "bob".to_string(),
            100,
            5,
            Some("memo".to_string()),
        );

        let store: StoreTransaction = (&legacy).into();
        assert_eq!(store.input_did, "alice");
        assert_eq!(store.output_recipient, "bob");
        assert_eq!(store.amount, 100);
        assert_eq!(store.id, legacy.id);

        let back: LegacyTransaction = (&store).into();
        assert_eq!(back.from, "alice");
        assert_eq!(back.to, "bob");
        assert_eq!(back.amount, 100);
        // Fee and data are lost in roundtrip (by design)
        assert_eq!(back.fee, 0);
    }

    #[test]
    fn legacy_block_to_store() {
        let legacy = LegacyBlock::new(1, vec![], "00".repeat(32), 2);

        let store: StoreBlock = (&legacy).into();
        assert_eq!(store.height, 1);
        assert_eq!(store.timestamp, legacy.timestamp);
        assert_eq!(store.transactions.len(), 0);
    }

    #[test]
    fn hex_to_bytes32_handles_short_input() {
        let result = hex_to_bytes32("abcd");
        assert_eq!(result[0], 0xab);
        assert_eq!(result[1], 0xcd);
        assert_eq!(result[2], 0x00);
    }
}
