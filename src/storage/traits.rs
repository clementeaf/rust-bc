//! Storage trait definitions
//!
//! Defines the BlockStore trait and related interfaces for storage operations.

use std::sync::Arc;

use super::errors::StorageResult;
use crate::endorsement::types::Endorsement;

/// Block structure for storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub height: u64,
    pub timestamp: u64,
    pub parent_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub transactions: Vec<String>,
    pub proposer: String,
    #[serde(with = "sig_hex")]
    pub signature: [u8; 64],
    /// Endorsements collected for this block (empty for legacy blocks)
    #[serde(default)]
    pub endorsements: Vec<Endorsement>,
    /// Orderer signature over the block hash (absent for legacy blocks).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_sig_hex")]
    pub orderer_signature: Option<[u8; 64]>,
}

mod sig_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sig: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(sig))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let hex_str = String::deserialize(d)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("signature must be 64 bytes"))
    }
}

mod opt_sig_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sig: &Option<[u8; 64]>, s: S) -> Result<S::Ok, S::Error> {
        match sig {
            Some(bytes) => s.serialize_str(&hex::encode(bytes)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<[u8; 64]>, D::Error> {
        let opt: Option<String> = Option::deserialize(d)?;
        match opt {
            None => Ok(None),
            Some(hex_str) => {
                let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
                let arr: [u8; 64] = bytes
                    .try_into()
                    .map_err(|_| serde::de::Error::custom("orderer_signature must be 64 bytes"))?;
                Ok(Some(arr))
            }
        }
    }
}

/// Transaction structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Transaction {
    pub id: String,
    pub block_height: u64,
    pub timestamp: u64,
    pub input_did: String,
    pub output_recipient: String,
    pub amount: u64,
    pub state: String,
}

/// Identity record structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdentityRecord {
    pub did: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub status: String,
}

/// Credential structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Credential {
    pub id: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub cred_type: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub revoked_at: Option<u64>,
}

/// A single entry in the history of a world-state key.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub version: u64,
    pub data: Vec<u8>,
    pub tx_id: String,
    pub timestamp: u64,
    pub is_delete: bool,
}

/// BlockStore trait - main storage interface
pub trait BlockStore: Send + Sync {
    /// Write a block to storage
    fn write_block(&self, block: &Block) -> StorageResult<()>;

    /// Read a block by height
    fn read_block(&self, height: u64) -> StorageResult<Block>;

    /// Write a transaction
    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()>;

    /// Read a transaction by ID
    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction>;

    /// Write an identity record
    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()>;

    /// Read an identity record by DID
    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord>;

    /// Write a credential
    fn write_credential(&self, credential: &Credential) -> StorageResult<()>;

    /// Read a credential by ID
    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential>;

    /// Batch write operations (atomic)
    fn write_batch(&self, blocks: &[Block], txs: &[Transaction]) -> StorageResult<()>;

    /// Get latest block height
    fn get_latest_height(&self) -> StorageResult<u64>;

    /// Check if block exists
    fn block_exists(&self, height: u64) -> StorageResult<bool>;

    /// Return all transactions belonging to a given block height.
    ///
    /// Returns an empty `Vec` when no transactions are indexed for that height.
    fn transactions_by_block_height(&self, height: u64) -> StorageResult<Vec<Transaction>>;

    /// Return all credentials issued to a given subject DID.
    ///
    /// Returns an empty `Vec` when no credentials are indexed for that subject.
    fn credentials_by_subject_did(&self, subject_did: &str) -> StorageResult<Vec<Credential>>;

    /// Return a page of blocks plus the total count for pagination.
    ///
    /// Default implementation iterates `[offset, offset+limit)` by height.
    fn list_blocks(&self, offset: usize, limit: usize) -> StorageResult<(Vec<Block>, usize)> {
        let latest = self.get_latest_height().unwrap_or(0) as usize;
        let total = if latest == 0 && self.read_block(0).is_err() {
            0
        } else {
            latest + 1
        };
        let mut blocks = Vec::new();
        let start = offset;
        let end = (offset + limit).min(total);
        for h in start..end {
            if let Ok(b) = self.read_block(h as u64) {
                blocks.push(b);
            }
        }
        Ok((blocks, total))
    }
}

/// Blanket impl so `Arc<T>` can be used wherever `Box<dyn BlockStore>` is expected.
///
/// All methods delegate to the inner `T`.  Because `MemoryStore` uses interior
/// mutability (`Mutex`), `&self` is sufficient for writes.
impl<T: BlockStore> BlockStore for Arc<T> {
    fn write_block(&self, block: &Block) -> StorageResult<()> {
        (**self).write_block(block)
    }
    fn read_block(&self, height: u64) -> StorageResult<Block> {
        (**self).read_block(height)
    }
    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()> {
        (**self).write_transaction(tx)
    }
    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction> {
        (**self).read_transaction(tx_id)
    }
    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()> {
        (**self).write_identity(identity)
    }
    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord> {
        (**self).read_identity(did)
    }
    fn write_credential(&self, credential: &Credential) -> StorageResult<()> {
        (**self).write_credential(credential)
    }
    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential> {
        (**self).read_credential(cred_id)
    }
    fn write_batch(&self, blocks: &[Block], txs: &[Transaction]) -> StorageResult<()> {
        (**self).write_batch(blocks, txs)
    }
    fn get_latest_height(&self) -> StorageResult<u64> {
        (**self).get_latest_height()
    }
    fn block_exists(&self, height: u64) -> StorageResult<bool> {
        (**self).block_exists(height)
    }

    fn transactions_by_block_height(&self, height: u64) -> StorageResult<Vec<Transaction>> {
        (**self).transactions_by_block_height(height)
    }

    fn credentials_by_subject_did(&self, subject_did: &str) -> StorageResult<Vec<Credential>> {
        (**self).credentials_by_subject_did(subject_did)
    }

    fn list_blocks(&self, offset: usize, limit: usize) -> StorageResult<(Vec<Block>, usize)> {
        (**self).list_blocks(offset, limit)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::storage::MemoryStore;

    use super::*;

    fn sample_block(height: u64) -> Block {
        Block {
            height,
            timestamp: 1_000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec![],
            proposer: "node-1".to_string(),
            signature: [2u8; 64],
            endorsements: vec![],
            orderer_signature: None,
        }
    }

    #[test]
    fn block_serde_roundtrip_without_endorsements() {
        let block = sample_block(1);
        let json = serde_json::to_string(&block).unwrap();
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.height, 1);
        assert!(decoded.endorsements.is_empty());
    }

    #[test]
    fn block_serde_roundtrip_with_endorsements() {
        use crate::endorsement::types::Endorsement;
        let e = Endorsement {
            signer_did: "did:bc:alice".to_string(),
            org_id: "org1".to_string(),
            signature: [1u8; 64],
            payload_hash: [2u8; 32],
            timestamp: 9999,
        };
        let block = Block {
            height: 5,
            timestamp: 1_000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec![],
            proposer: "node-1".to_string(),
            signature: [2u8; 64],
            endorsements: vec![e.clone()],
            orderer_signature: None,
        };
        let json = serde_json::to_string(&block).unwrap();
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.endorsements.len(), 1);
        assert_eq!(decoded.endorsements[0].org_id, "org1");
    }

    #[test]
    fn block_serde_with_orderer_signature() {
        let mut block = sample_block(10);
        block.orderer_signature = Some([42u8; 64]);
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("orderer_signature"));
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.orderer_signature, Some([42u8; 64]));
    }

    #[test]
    fn block_serde_without_orderer_signature() {
        let block = sample_block(11);
        let json = serde_json::to_string(&block).unwrap();
        assert!(!json.contains("orderer_signature"));
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.orderer_signature, None);
    }

    #[test]
    fn block_without_endorsements_field_deserializes_to_empty_vec() {
        // Serialize a block, strip the endorsements field, then deserialize — simulates legacy JSON.
        let block = sample_block(3);
        let full_json = serde_json::to_string(&block).unwrap();
        // Remove `,"endorsements":[]` or `"endorsements":[],` from the JSON.
        let legacy_json = full_json
            .replace(",\"endorsements\":[]", "")
            .replace("\"endorsements\":[],", "");
        let decoded: Block = serde_json::from_str(&legacy_json).unwrap();
        assert!(decoded.endorsements.is_empty());
    }

    #[test]
    fn arc_store_write_and_read() {
        let store = Arc::new(MemoryStore::new());
        store.write_block(&sample_block(1)).unwrap();
        let block = store.read_block(1).unwrap();
        assert_eq!(block.height, 1);
    }

    #[test]
    fn shared_arc_sees_writes_from_all_clones() {
        let store = Arc::new(MemoryStore::new());
        let writer = Arc::clone(&store);
        let reader = Arc::clone(&store);

        writer.write_block(&sample_block(7)).unwrap();
        assert!(reader.block_exists(7).unwrap());
        assert_eq!(reader.get_latest_height().unwrap(), 7);
    }

    #[test]
    fn arc_store_passed_as_box_dyn() {
        let store: Arc<MemoryStore> = Arc::new(MemoryStore::new());
        // Verify it can be coerced into Box<dyn BlockStore>
        let boxed: Box<dyn BlockStore> = Box::new(Arc::clone(&store));
        boxed.write_block(&sample_block(3)).unwrap();
        // The original Arc sees the write
        assert!(store.block_exists(3).unwrap());
    }
}
