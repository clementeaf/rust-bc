//! Storage trait definitions
//!
//! Defines the BlockStore trait and related interfaces for storage operations.

use std::sync::Arc;

use super::errors::StorageResult;

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
        }
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
