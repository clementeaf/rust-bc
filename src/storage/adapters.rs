//! RocksDB storage adapter implementation
//!
//! Implements the BlockStore trait using RocksDB as the backing store.

use rocksdb::{Options, WriteBatch, DB};
use std::path::Path;

use super::errors::{StorageError, StorageResult};
use super::traits::{Block, BlockStore, Credential, IdentityRecord, Transaction};

const META_LATEST_HEIGHT: &[u8] = b"META:latest_height";

/// RocksDB-backed block store
pub struct RocksDbBlockStore {
    db: DB,
}

impl RocksDbBlockStore {
    /// Open (or create) a RocksDB database at the given path.
    pub fn new(path: impl AsRef<Path>) -> StorageResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path.as_ref())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?;
        Ok(RocksDbBlockStore { db })
    }

    fn block_key(height: u64) -> Vec<u8> {
        format!("BLK:{:012}", height).into_bytes()
    }

    fn transaction_key(tx_id: &str) -> Vec<u8> {
        format!("TX:{}", tx_id).into_bytes()
    }

    fn identity_key(did: &str) -> Vec<u8> {
        format!("DID:{}", did).into_bytes()
    }

    fn credential_key(cred_id: &str) -> Vec<u8> {
        format!("CRED:{}", cred_id).into_bytes()
    }
}

impl BlockStore for RocksDbBlockStore {
    fn write_block(&self, block: &Block) -> StorageResult<()> {
        let value = serde_json::to_vec(block)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let current_latest = self.get_latest_height().unwrap_or(0);

        let mut batch = WriteBatch::default();
        batch.put(Self::block_key(block.height), &value);
        if block.height >= current_latest {
            batch.put(META_LATEST_HEIGHT, block.height.to_le_bytes());
        }
        self.db
            .write(batch)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_block(&self, height: u64) -> StorageResult<Block> {
        let key = Self::block_key(height);
        match self
            .db
            .get(&key)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::KeyNotFound(
                String::from_utf8_lossy(&key).into_owned(),
            )),
        }
    }

    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()> {
        let value = serde_json::to_vec(tx)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put(Self::transaction_key(&tx.id), &value)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction> {
        let key = Self::transaction_key(tx_id);
        match self
            .db
            .get(&key)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::KeyNotFound(
                String::from_utf8_lossy(&key).into_owned(),
            )),
        }
    }

    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()> {
        let value = serde_json::to_vec(identity)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put(Self::identity_key(&identity.did), &value)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord> {
        let key = Self::identity_key(did);
        match self
            .db
            .get(&key)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::IdentityNotFound(did.to_string())),
        }
    }

    fn write_credential(&self, credential: &Credential) -> StorageResult<()> {
        let value = serde_json::to_vec(credential)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put(Self::credential_key(&credential.id), &value)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential> {
        let key = Self::credential_key(cred_id);
        match self
            .db
            .get(&key)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::CredentialNotFound(cred_id.to_string())),
        }
    }

    fn write_batch(&self, blocks: &[Block], txs: &[Transaction]) -> StorageResult<()> {
        if blocks.is_empty() && txs.is_empty() {
            return Err(StorageError::BatchOperationFailed(
                "Empty batch".to_string(),
            ));
        }

        let current_latest = self.get_latest_height().unwrap_or(0);
        let mut new_latest = current_latest;
        let mut batch = WriteBatch::default();

        for block in blocks {
            let value = serde_json::to_vec(block)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            batch.put(Self::block_key(block.height), &value);
            if block.height > new_latest {
                new_latest = block.height;
            }
        }

        for tx in txs {
            let value = serde_json::to_vec(tx)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            batch.put(Self::transaction_key(&tx.id), &value);
        }

        if new_latest > current_latest {
            batch.put(META_LATEST_HEIGHT, new_latest.to_le_bytes());
        }

        self.db
            .write(batch)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn get_latest_height(&self) -> StorageResult<u64> {
        match self
            .db
            .get(META_LATEST_HEIGHT)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => {
                let arr: [u8; 8] = bytes.as_slice().try_into().map_err(|_| {
                    StorageError::DataCorrupted("latest_height is not 8 bytes".to_string())
                })?;
                Ok(u64::from_le_bytes(arr))
            }
            None => Ok(0),
        }
    }

    fn block_exists(&self, height: u64) -> StorageResult<bool> {
        self.db
            .get(Self::block_key(height))
            .map(|v| v.is_some())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_store() -> (RocksDbBlockStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = RocksDbBlockStore::new(dir.path()).unwrap();
        (store, dir)
    }

    fn sample_block(height: u64) -> Block {
        Block {
            height,
            timestamp: 1_000,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec!["tx1".to_string()],
            proposer: "proposer1".to_string(),
            signature: [2u8; 64],
        }
    }

    fn sample_tx(id: &str) -> Transaction {
        Transaction {
            id: id.to_string(),
            block_height: 1,
            timestamp: 1_000,
            input_did: "did:bc:input".to_string(),
            output_recipient: "did:bc:output".to_string(),
            amount: 100,
            state: "confirmed".to_string(),
        }
    }

    #[test]
    fn test_block_key_format() {
        assert_eq!(RocksDbBlockStore::block_key(1), b"BLK:000000000001");
        assert_eq!(RocksDbBlockStore::block_key(123456), b"BLK:000000123456");
    }

    #[test]
    fn test_transaction_key_format() {
        assert_eq!(RocksDbBlockStore::transaction_key("abc123"), b"TX:abc123");
    }

    #[test]
    fn test_identity_key_format() {
        assert_eq!(
            RocksDbBlockStore::identity_key("did:bc:xyz"),
            b"DID:did:bc:xyz"
        );
    }

    #[test]
    fn test_credential_key_format() {
        assert_eq!(
            RocksDbBlockStore::credential_key("cred-1"),
            b"CRED:cred-1"
        );
    }

    #[test]
    fn write_and_read_block_roundtrip() {
        let (store, _dir) = tmp_store();
        store.write_block(&sample_block(1)).unwrap();
        let block = store.read_block(1).unwrap();
        assert_eq!(block.height, 1);
        assert_eq!(block.proposer, "proposer1");
    }

    #[test]
    fn read_block_not_found() {
        let (store, _dir) = tmp_store();
        assert!(store.read_block(999).is_err());
    }

    #[test]
    fn block_exists_after_write() {
        let (store, _dir) = tmp_store();
        assert!(!store.block_exists(5).unwrap());
        store.write_block(&sample_block(5)).unwrap();
        assert!(store.block_exists(5).unwrap());
    }

    #[test]
    fn latest_height_tracks_writes() {
        let (store, _dir) = tmp_store();
        assert_eq!(store.get_latest_height().unwrap(), 0);
        store.write_block(&sample_block(3)).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 3);
        store.write_block(&sample_block(7)).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 7);
        // Writing an older block does not decrease latest
        store.write_block(&sample_block(2)).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 7);
    }

    #[test]
    fn write_and_read_transaction_roundtrip() {
        let (store, _dir) = tmp_store();
        store.write_transaction(&sample_tx("tx123")).unwrap();
        let tx = store.read_transaction("tx123").unwrap();
        assert_eq!(tx.id, "tx123");
        assert_eq!(tx.amount, 100);
    }

    #[test]
    fn write_and_read_identity_roundtrip() {
        let (store, _dir) = tmp_store();
        let identity = IdentityRecord {
            did: "did:bc:123".to_string(),
            created_at: 1_000,
            updated_at: 2_000,
            status: "active".to_string(),
        };
        store.write_identity(&identity).unwrap();
        let loaded = store.read_identity("did:bc:123").unwrap();
        assert_eq!(loaded.did, "did:bc:123");
        assert_eq!(loaded.status, "active");
    }

    #[test]
    fn write_and_read_credential_roundtrip() {
        let (store, _dir) = tmp_store();
        let cred = Credential {
            id: "cred-1".to_string(),
            issuer_did: "did:bc:issuer".to_string(),
            subject_did: "did:bc:subject".to_string(),
            cred_type: "eid".to_string(),
            issued_at: 1_000,
            expires_at: 2_000,
            revoked_at: None,
        };
        store.write_credential(&cred).unwrap();
        let loaded = store.read_credential("cred-1").unwrap();
        assert_eq!(loaded.id, "cred-1");
        assert_eq!(loaded.cred_type, "eid");
    }

    #[test]
    fn write_batch_empty_fails() {
        let (store, _dir) = tmp_store();
        assert!(store.write_batch(&[], &[]).is_err());
    }

    #[test]
    fn write_batch_atomically_stores_blocks_and_txs() {
        let (store, _dir) = tmp_store();
        let blocks = vec![sample_block(10), sample_block(11)];
        let txs = vec![sample_tx("batch-tx-1")];
        store.write_batch(&blocks, &txs).unwrap();
        assert!(store.block_exists(10).unwrap());
        assert!(store.block_exists(11).unwrap());
        assert_eq!(store.read_transaction("batch-tx-1").unwrap().id, "batch-tx-1");
        assert_eq!(store.get_latest_height().unwrap(), 11);
    }
}
