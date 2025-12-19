//! RocksDB storage adapter implementation
//!
//! Implements the BlockStore trait using RocksDB as the backing store.

use super::errors::{StorageError, StorageResult};
use super::traits::{Block, BlockStore, Credential, IdentityRecord, Transaction};
use std::path::Path;

/// RocksDB-backed block store
pub struct RocksDbBlockStore {
    db_path: String,
}

impl RocksDbBlockStore {
    /// Create a new RocksDB store instance
    pub fn new(path: impl AsRef<Path>) -> StorageResult<Self> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| StorageError::Other("Invalid path".to_string()))?
            .to_string();

        Ok(RocksDbBlockStore { db_path: path_str })
    }

    /// Format a block key for storage
    fn block_key(height: u64) -> String {
        format!("BLK:{:012}", height)
    }

    /// Format a transaction key for storage
    fn transaction_key(tx_id: &str) -> String {
        format!("TX:{}", tx_id)
    }

    /// Format an identity key for storage
    fn identity_key(did: &str) -> String {
        format!("DID:{}", did)
    }

    /// Format a credential key for storage
    fn credential_key(cred_id: &str) -> String {
        format!("CRED:{}", cred_id)
    }
}

impl BlockStore for RocksDbBlockStore {
    fn write_block(&self, block: &Block) -> StorageResult<()> {
        let key = Self::block_key(block.height);
        // Placeholder: actual RocksDB write would go here
        // This is scaffolding for Week 2 integration
        println!("Writing block at key: {}", key);
        Ok(())
    }

    fn read_block(&self, height: u64) -> StorageResult<Block> {
        let key = Self::block_key(height);
        // Placeholder: actual RocksDB read would go here
        Err(StorageError::KeyNotFound(key))
    }

    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()> {
        let key = Self::transaction_key(&tx.id);
        println!("Writing transaction at key: {}", key);
        Ok(())
    }

    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction> {
        let key = Self::transaction_key(tx_id);
        Err(StorageError::KeyNotFound(key))
    }

    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()> {
        let key = Self::identity_key(&identity.did);
        println!("Writing identity at key: {}", key);
        Ok(())
    }

    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord> {
        let key = Self::identity_key(did);
        Err(StorageError::KeyNotFound(key))
    }

    fn write_credential(&self, credential: &Credential) -> StorageResult<()> {
        let key = Self::credential_key(&credential.id);
        println!("Writing credential at key: {}", key);
        Ok(())
    }

    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential> {
        let key = Self::credential_key(cred_id);
        Err(StorageError::KeyNotFound(key))
    }

    fn write_batch(&self, blocks: &[Block], txs: &[Transaction]) -> StorageResult<()> {
        if blocks.is_empty() && txs.is_empty() {
            return Err(StorageError::BatchOperationFailed(
                "Empty batch".to_string(),
            ));
        }

        println!(
            "Writing batch: {} blocks, {} transactions",
            blocks.len(),
            txs.len()
        );
        Ok(())
    }

    fn get_latest_height(&self) -> StorageResult<u64> {
        // Placeholder: would iterate through blocks in RocksDB
        Ok(0)
    }

    fn block_exists(&self, height: u64) -> StorageResult<bool> {
        let key = Self::block_key(height);
        // Placeholder: would check RocksDB for key existence
        println!("Checking block existence at key: {}", key);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_key_format() {
        assert_eq!(RocksDbBlockStore::block_key(1), "BLK:000000000001");
        assert_eq!(
            RocksDbBlockStore::block_key(123456),
            "BLK:000000123456"
        );
    }

    #[test]
    fn test_transaction_key_format() {
        assert_eq!(
            RocksDbBlockStore::transaction_key("abc123"),
            "TX:abc123"
        );
    }

    #[test]
    fn test_identity_key_format() {
        assert_eq!(
            RocksDbBlockStore::identity_key("did:bc:xyz"),
            "DID:did:bc:xyz"
        );
    }

    #[test]
    fn test_credential_key_format() {
        assert_eq!(
            RocksDbBlockStore::credential_key("cred-1"),
            "CRED:cred-1"
        );
    }

    #[test]
    fn test_store_creation() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        assert_eq!(store.db_path, "/tmp/test_db");
    }

    #[test]
    fn test_write_block_success() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let block = Block {
            height: 1,
            timestamp: 1000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec!["tx1".to_string()],
            proposer: "proposer1".to_string(),
            signature: [2u8; 64],
        };

        assert!(store.write_block(&block).is_ok());
    }

    #[test]
    fn test_write_transaction_success() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let tx = Transaction {
            id: "tx123".to_string(),
            block_height: 1,
            timestamp: 1000,
            input_did: "did:bc:input".to_string(),
            output_recipient: "did:bc:output".to_string(),
            amount: 100,
            state: "confirmed".to_string(),
        };

        assert!(store.write_transaction(&tx).is_ok());
    }

    #[test]
    fn test_write_batch_empty_fails() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let result = store.write_batch(&[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_batch_with_data() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let block = Block {
            height: 1,
            timestamp: 1000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec!["tx1".to_string()],
            proposer: "proposer1".to_string(),
            signature: [2u8; 64],
        };
        let tx = Transaction {
            id: "tx123".to_string(),
            block_height: 1,
            timestamp: 1000,
            input_did: "did:bc:input".to_string(),
            output_recipient: "did:bc:output".to_string(),
            amount: 100,
            state: "confirmed".to_string(),
        };

        assert!(store.write_batch(&[block], &[tx]).is_ok());
    }

    #[test]
    fn test_get_latest_height() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let height = store.get_latest_height().unwrap();
        assert_eq!(height, 0);
    }

    #[test]
    fn test_write_identity_success() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let identity = IdentityRecord {
            did: "did:bc:123".to_string(),
            created_at: 1000,
            updated_at: 2000,
            status: "active".to_string(),
        };

        assert!(store.write_identity(&identity).is_ok());
    }

    #[test]
    fn test_write_credential_success() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let cred = Credential {
            id: "cred-1".to_string(),
            issuer_did: "did:bc:issuer".to_string(),
            subject_did: "did:bc:subject".to_string(),
            cred_type: "eid".to_string(),
            issued_at: 1000,
            expires_at: 2000,
            revoked_at: None,
        };

        assert!(store.write_credential(&cred).is_ok());
    }

    #[test]
    fn test_read_block_not_found() {
        let store = RocksDbBlockStore::new("/tmp/test_db").unwrap();
        let result = store.read_block(999);
        assert!(result.is_err());
    }
}
