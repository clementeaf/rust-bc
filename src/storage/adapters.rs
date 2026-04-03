//! RocksDB storage adapter implementation
//!
//! Implements the BlockStore trait using RocksDB with dedicated Column Families:
//!
//! | CF name       | Key schema            | Value          |
//! |---------------|-----------------------|----------------|
//! | `blocks`      | zero-padded height    | JSON Block     |
//! | `transactions`| tx_id                 | JSON Tx        |
//! | `identities`  | DID string            | JSON Identity  |
//! | `credentials` | cred_id string        | JSON Credential|
//! | `meta`        | well-known byte keys  | raw bytes      |

use rocksdb::{ColumnFamilyDescriptor, Direction, IteratorMode, Options, WriteBatch, DB};
use std::path::Path;

use super::errors::{StorageError, StorageResult};
use super::traits::{Block, BlockStore, Credential, IdentityRecord, Transaction};
use crate::endorsement::org::Organization;
use crate::endorsement::registry::OrgRegistry;

const CF_BLOCKS: &str = "blocks";
const CF_TRANSACTIONS: &str = "transactions";
const CF_IDENTITIES: &str = "identities";
const CF_CREDENTIALS: &str = "credentials";
const CF_META: &str = "meta";
/// Secondary index: `{012-padded-height}:{tx_id}` → `""` (empty)
const CF_TX_BY_BLOCK: &str = "tx_by_block";
/// Secondary index: `{subject_did}:{cred_id}` → `""` (empty)
const CF_CRED_BY_SUBJECT: &str = "cred_by_subject";
/// Organizations registry
const CF_ORGANIZATIONS: &str = "organizations";

const META_LATEST_HEIGHT: &[u8] = b"latest_height";

const ALL_CFS: &[&str] = &[
    CF_BLOCKS,
    CF_TRANSACTIONS,
    CF_IDENTITIES,
    CF_CREDENTIALS,
    CF_META,
    CF_TX_BY_BLOCK,
    CF_CRED_BY_SUBJECT,
    CF_ORGANIZATIONS,
];

/// RocksDB-backed block store using Column Families for data isolation
pub struct RocksDbBlockStore {
    db: DB,
}

impl RocksDbBlockStore {
    /// Open (or create) a RocksDB database at the given path.
    ///
    /// All five column families are created automatically when missing,
    /// so this works on both new and existing databases.
    pub fn new(path: impl AsRef<Path>) -> StorageResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descriptors: Vec<ColumnFamilyDescriptor> = ALL_CFS
            .iter()
            .map(|&name| ColumnFamilyDescriptor::new(name, Options::default()))
            .collect();

        let db = DB::open_cf_descriptors(&opts, path.as_ref(), cf_descriptors)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?;

        Ok(RocksDbBlockStore { db })
    }

    // ── Column Family handle helpers ──────────────────────────────────────────

    fn cf_blocks(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_BLOCKS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_BLOCKS.to_string()))
    }

    fn cf_transactions(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TRANSACTIONS.to_string()))
    }

    fn cf_identities(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_IDENTITIES)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_IDENTITIES.to_string()))
    }

    fn cf_credentials(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_CREDENTIALS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_CREDENTIALS.to_string()))
    }

    fn cf_meta(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_META)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_META.to_string()))
    }

    fn cf_tx_by_block(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_TX_BY_BLOCK)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_TX_BY_BLOCK.to_string()))
    }

    fn cf_cred_by_subject(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_CRED_BY_SUBJECT)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_CRED_BY_SUBJECT.to_string()))
    }

    fn cf_organizations(&self) -> StorageResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(CF_ORGANIZATIONS)
            .ok_or_else(|| StorageError::ColumnFamilyNotFound(CF_ORGANIZATIONS.to_string()))
    }

    // ── Key encoders ─────────────────────────────────────────────────────────

    /// Zero-padded decimal height gives lexicographic == numeric ordering.
    fn block_key(height: u64) -> Vec<u8> {
        format!("{:012}", height).into_bytes()
    }

    /// Secondary-index key: `{012-padded-height}:{tx_id}`.
    ///
    /// The fixed-width height prefix keeps all entries for a block contiguous
    /// and in numeric order, enabling a simple prefix range scan.
    fn tx_block_index_key(height: u64, tx_id: &str) -> Vec<u8> {
        format!("{:012}:{}", height, tx_id).into_bytes()
    }

    /// The prefix used to scan all index entries for `height`.
    fn tx_block_prefix(height: u64) -> Vec<u8> {
        format!("{:012}:", height).into_bytes()
    }

    /// Secondary-index key for the subject-DID index: `{subject_did}\x00{cred_id}`.
    ///
    /// `\x00` is used as separator because DID characters and cred IDs never
    /// contain a NUL byte, making the prefix scan unambiguous.
    fn cred_subject_index_key(subject_did: &str, cred_id: &str) -> Vec<u8> {
        let mut key = Vec::with_capacity(subject_did.len() + 1 + cred_id.len());
        key.extend_from_slice(subject_did.as_bytes());
        key.push(0x00);
        key.extend_from_slice(cred_id.as_bytes());
        key
    }

    /// Prefix used to scan all index entries for `subject_did`.
    fn cred_subject_prefix(subject_did: &str) -> Vec<u8> {
        let mut prefix = Vec::with_capacity(subject_did.len() + 1);
        prefix.extend_from_slice(subject_did.as_bytes());
        prefix.push(0x00);
        prefix
    }
}

impl BlockStore for RocksDbBlockStore {
    fn write_block(&self, block: &Block) -> StorageResult<()> {
        let value = serde_json::to_vec(block)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let current_latest = self.get_latest_height().unwrap_or(0);
        let cf_b = self.cf_blocks()?;
        let cf_m = self.cf_meta()?;

        let mut batch = WriteBatch::default();
        batch.put_cf(cf_b, Self::block_key(block.height), &value);
        if block.height >= current_latest {
            batch.put_cf(cf_m, META_LATEST_HEIGHT, block.height.to_le_bytes());
        }
        self.db
            .write(batch)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_block(&self, height: u64) -> StorageResult<Block> {
        let key = Self::block_key(height);
        match self
            .db
            .get_cf(self.cf_blocks()?, &key)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::KeyNotFound(format!("block:{height}"))),
        }
    }

    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()> {
        let value = serde_json::to_vec(tx)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let mut batch = WriteBatch::default();
        batch.put_cf(self.cf_transactions()?, tx.id.as_bytes(), &value);
        batch.put_cf(
            self.cf_tx_by_block()?,
            Self::tx_block_index_key(tx.block_height, &tx.id),
            b"",
        );
        self.db
            .write(batch)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction> {
        match self
            .db
            .get_cf(self.cf_transactions()?, tx_id.as_bytes())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::KeyNotFound(format!("tx:{tx_id}"))),
        }
    }

    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()> {
        let value = serde_json::to_vec(identity)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put_cf(self.cf_identities()?, identity.did.as_bytes(), &value)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord> {
        match self
            .db
            .get_cf(self.cf_identities()?, did.as_bytes())
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

        let mut batch = WriteBatch::default();
        batch.put_cf(self.cf_credentials()?, credential.id.as_bytes(), &value);
        batch.put_cf(
            self.cf_cred_by_subject()?,
            Self::cred_subject_index_key(&credential.subject_did, &credential.id),
            b"",
        );
        self.db
            .write(batch)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential> {
        match self
            .db
            .get_cf(self.cf_credentials()?, cred_id.as_bytes())
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

        let cf_b = self.cf_blocks()?;
        let cf_t = self.cf_transactions()?;
        let cf_m = self.cf_meta()?;
        let cf_idx = self.cf_tx_by_block()?;

        let mut batch = WriteBatch::default();

        for block in blocks {
            let value = serde_json::to_vec(block)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            batch.put_cf(cf_b, Self::block_key(block.height), &value);
            if block.height > new_latest {
                new_latest = block.height;
            }
        }

        for tx in txs {
            let value = serde_json::to_vec(tx)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            batch.put_cf(cf_t, tx.id.as_bytes(), &value);
            batch.put_cf(
                cf_idx,
                Self::tx_block_index_key(tx.block_height, &tx.id),
                b"",
            );
        }

        if new_latest > current_latest {
            batch.put_cf(cf_m, META_LATEST_HEIGHT, new_latest.to_le_bytes());
        }

        self.db
            .write(batch)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn get_latest_height(&self) -> StorageResult<u64> {
        match self
            .db
            .get_cf(self.cf_meta()?, META_LATEST_HEIGHT)
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
            .get_cf(self.cf_blocks()?, Self::block_key(height))
            .map(|v| v.is_some())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn transactions_by_block_height(&self, height: u64) -> StorageResult<Vec<Transaction>> {
        let prefix = Self::tx_block_prefix(height);
        let cf_idx = self.cf_tx_by_block()?;
        let cf_t = self.cf_transactions()?;

        let iter = self
            .db
            .iterator_cf(cf_idx, IteratorMode::From(&prefix, Direction::Forward));

        let mut txs = Vec::new();
        for item in iter {
            let (key, _) = item.map_err(|e| StorageError::RocksDbError(e.to_string()))?;

            // Stop once we've passed all keys with this height prefix.
            if !key.starts_with(&prefix) {
                break;
            }

            // Extract tx_id: everything after `{012}:`
            let tx_id = std::str::from_utf8(&key[prefix.len()..])
                .map_err(|e| StorageError::DataCorrupted(e.to_string()))?;

            let tx_bytes = self
                .db
                .get_cf(cf_t, tx_id.as_bytes())
                .map_err(|e| StorageError::RocksDbError(e.to_string()))?
                .ok_or_else(|| StorageError::KeyNotFound(format!("tx:{tx_id}")))?;

            let tx: Transaction = serde_json::from_slice(&tx_bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
            txs.push(tx);
        }

        Ok(txs)
    }

    fn credentials_by_subject_did(&self, subject_did: &str) -> StorageResult<Vec<Credential>> {
        let prefix = Self::cred_subject_prefix(subject_did);
        let cf_idx = self.cf_cred_by_subject()?;
        let cf_c = self.cf_credentials()?;

        let iter = self
            .db
            .iterator_cf(cf_idx, IteratorMode::From(&prefix, Direction::Forward));

        let mut creds = Vec::new();
        for item in iter {
            let (key, _) = item.map_err(|e| StorageError::RocksDbError(e.to_string()))?;

            if !key.starts_with(&prefix) {
                break;
            }

            // Extract cred_id: everything after `{subject_did}\x00`
            let cred_id = std::str::from_utf8(&key[prefix.len()..])
                .map_err(|e| StorageError::DataCorrupted(e.to_string()))?;

            let cred_bytes = self
                .db
                .get_cf(cf_c, cred_id.as_bytes())
                .map_err(|e| StorageError::RocksDbError(e.to_string()))?
                .ok_or_else(|| StorageError::KeyNotFound(format!("cred:{cred_id}")))?;

            let cred: Credential = serde_json::from_slice(&cred_bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
            creds.push(cred);
        }

        Ok(creds)
    }
}

impl OrgRegistry for RocksDbBlockStore {
    fn register_org(&self, org: &Organization) -> StorageResult<()> {
        let cf = self.cf_organizations()?;
        let value = serde_json::to_vec(org)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put_cf(&cf, org.org_id.as_bytes(), &value)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn get_org(&self, org_id: &str) -> StorageResult<Organization> {
        let cf = self.cf_organizations()?;
        match self
            .db
            .get_cf(&cf, org_id.as_bytes())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::DeserializationError(e.to_string())),
            None => Err(StorageError::KeyNotFound(org_id.to_string())),
        }
    }

    fn list_orgs(&self) -> StorageResult<Vec<Organization>> {
        let cf = self.cf_organizations()?;
        let mut orgs = Vec::new();
        for item in self.db.iterator_cf(&cf, IteratorMode::Start) {
            let (_, value) = item.map_err(|e| StorageError::RocksDbError(e.to_string()))?;
            let org: Organization = serde_json::from_slice(&value)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
            orgs.push(org);
        }
        Ok(orgs)
    }

    fn remove_org(&self, org_id: &str) -> StorageResult<()> {
        let cf = self.cf_organizations()?;
        // Verify it exists first
        if self
            .db
            .get_cf(&cf, org_id.as_bytes())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
            .is_none()
        {
            return Err(StorageError::KeyNotFound(org_id.to_string()));
        }
        self.db
            .delete_cf(&cf, org_id.as_bytes())
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
            endorsements: vec![],
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

    // ── Key encoding ─────────────────────────────────────────────────────────

    #[test]
    fn block_key_is_zero_padded() {
        assert_eq!(RocksDbBlockStore::block_key(1), b"000000000001");
        assert_eq!(RocksDbBlockStore::block_key(123456), b"000000123456");
    }

    #[test]
    fn block_key_lexicographic_order_matches_numeric() {
        let k1 = RocksDbBlockStore::block_key(9);
        let k2 = RocksDbBlockStore::block_key(10);
        assert!(k1 < k2, "lexicographic order must match numeric order");
    }

    // ── Column Family presence ────────────────────────────────────────────────

    #[test]
    fn all_column_families_exist_after_open() {
        let (store, _dir) = tmp_store();
        assert!(store.cf_blocks().is_ok());
        assert!(store.cf_transactions().is_ok());
        assert!(store.cf_identities().is_ok());
        assert!(store.cf_credentials().is_ok());
        assert!(store.cf_meta().is_ok());
    }

    #[test]
    fn reopening_existing_db_preserves_data() {
        let dir = TempDir::new().unwrap();
        {
            let store = RocksDbBlockStore::new(dir.path()).unwrap();
            store.write_block(&sample_block(1)).unwrap();
        }
        // Re-open same path
        let store2 = RocksDbBlockStore::new(dir.path()).unwrap();
        assert!(store2.block_exists(1).unwrap());
        assert_eq!(store2.get_latest_height().unwrap(), 1);
    }

    // ── Block operations ─────────────────────────────────────────────────────

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

    // ── Transaction operations ───────────────────────────────────────────────

    #[test]
    fn write_and_read_transaction_roundtrip() {
        let (store, _dir) = tmp_store();
        store.write_transaction(&sample_tx("tx123")).unwrap();
        let tx = store.read_transaction("tx123").unwrap();
        assert_eq!(tx.id, "tx123");
        assert_eq!(tx.amount, 100);
    }

    #[test]
    fn transaction_stored_in_own_cf_not_visible_as_block() {
        let (store, _dir) = tmp_store();
        store.write_transaction(&sample_tx("tx-cf-isolation")).unwrap();
        // height 0 should not exist (tx keys are strings, block keys are numbers)
        assert!(!store.block_exists(0).unwrap());
    }

    // ── Identity operations ──────────────────────────────────────────────────

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
    fn read_identity_not_found_returns_identity_error() {
        let (store, _dir) = tmp_store();
        let err = store.read_identity("did:bc:ghost").unwrap_err();
        assert!(matches!(err, StorageError::IdentityNotFound(_)));
    }

    // ── Credential operations ────────────────────────────────────────────────

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
    fn read_credential_not_found_returns_credential_error() {
        let (store, _dir) = tmp_store();
        let err = store.read_credential("ghost").unwrap_err();
        assert!(matches!(err, StorageError::CredentialNotFound(_)));
    }

    // ── Batch operations ─────────────────────────────────────────────────────

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

    #[test]
    fn write_batch_txs_only_does_not_update_latest_height() {
        let (store, _dir) = tmp_store();
        store.write_block(&sample_block(5)).unwrap();
        store.write_batch(&[], &[sample_tx("only-tx")]).unwrap();
        // Latest height unchanged — no blocks in batch
        assert_eq!(store.get_latest_height().unwrap(), 5);
    }

    // ── Secondary index: tx_by_block_height ───────────────────────────────────

    fn tx_at_height(id: &str, height: u64) -> Transaction {
        Transaction {
            id: id.to_string(),
            block_height: height,
            timestamp: 1_000,
            input_did: "did:bc:in".to_string(),
            output_recipient: "did:bc:out".to_string(),
            amount: 1,
            state: "confirmed".to_string(),
        }
    }

    #[test]
    fn index_key_format_is_correct() {
        let key = RocksDbBlockStore::tx_block_index_key(7, "abc");
        assert_eq!(key, b"000000000007:abc");
        let prefix = RocksDbBlockStore::tx_block_prefix(7);
        assert!(key.starts_with(&prefix));
    }

    #[test]
    fn transactions_by_block_height_returns_empty_for_unknown_height() {
        let (store, _dir) = tmp_store();
        let txs = store.transactions_by_block_height(99).unwrap();
        assert!(txs.is_empty());
    }

    #[test]
    fn write_transaction_is_queryable_by_block_height() {
        let (store, _dir) = tmp_store();
        store.write_transaction(&tx_at_height("tx-a", 5)).unwrap();
        store.write_transaction(&tx_at_height("tx-b", 5)).unwrap();
        store.write_transaction(&tx_at_height("tx-c", 6)).unwrap();

        let block5 = store.transactions_by_block_height(5).unwrap();
        let ids: Vec<&str> = block5.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"tx-a"));
        assert!(ids.contains(&"tx-b"));

        let block6 = store.transactions_by_block_height(6).unwrap();
        assert_eq!(block6.len(), 1);
        assert_eq!(block6[0].id, "tx-c");
    }

    #[test]
    fn write_batch_indexes_transactions_by_block_height() {
        let (store, _dir) = tmp_store();
        let txs = vec![tx_at_height("btx-1", 10), tx_at_height("btx-2", 10)];
        store.write_batch(&[sample_block(10)], &txs).unwrap();

        let result = store.transactions_by_block_height(10).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn index_does_not_cross_height_boundaries() {
        let (store, _dir) = tmp_store();
        // heights 9 and 10 have similar decimal prefix — confirm no bleed-over
        store.write_transaction(&tx_at_height("tx-9", 9)).unwrap();
        store.write_transaction(&tx_at_height("tx-10", 10)).unwrap();

        assert_eq!(store.transactions_by_block_height(9).unwrap().len(), 1);
        assert_eq!(store.transactions_by_block_height(10).unwrap().len(), 1);
    }

    // ── Secondary index: cred_by_subject_did ─────────────────────────────────

    fn cred_for_subject(id: &str, subject_did: &str) -> Credential {
        Credential {
            id: id.to_string(),
            issuer_did: "did:bc:issuer".to_string(),
            subject_did: subject_did.to_string(),
            cred_type: "eid".to_string(),
            issued_at: 1_000,
            expires_at: 9_999,
            revoked_at: None,
        }
    }

    #[test]
    fn cred_subject_index_key_format_is_correct() {
        let key = RocksDbBlockStore::cred_subject_index_key("did:bc:alice", "cred-1");
        let prefix = RocksDbBlockStore::cred_subject_prefix("did:bc:alice");
        assert!(key.starts_with(&prefix));
        // cred_id follows the NUL separator
        assert_eq!(&key[prefix.len()..], b"cred-1");
    }

    #[test]
    fn credentials_by_subject_did_returns_empty_for_unknown_subject() {
        let (store, _dir) = tmp_store();
        assert!(store.credentials_by_subject_did("did:bc:ghost").unwrap().is_empty());
    }

    #[test]
    fn write_credential_is_queryable_by_subject_did() {
        let (store, _dir) = tmp_store();
        store.write_credential(&cred_for_subject("cred-1", "did:bc:alice")).unwrap();
        store.write_credential(&cred_for_subject("cred-2", "did:bc:alice")).unwrap();
        store.write_credential(&cred_for_subject("cred-3", "did:bc:bob")).unwrap();

        let alice = store.credentials_by_subject_did("did:bc:alice").unwrap();
        let ids: Vec<&str> = alice.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"cred-1"));
        assert!(ids.contains(&"cred-2"));

        let bob = store.credentials_by_subject_did("did:bc:bob").unwrap();
        assert_eq!(bob.len(), 1);
        assert_eq!(bob[0].id, "cred-3");
    }

    #[test]
    fn cred_subject_index_does_not_cross_subject_boundaries() {
        let (store, _dir) = tmp_store();
        // "did:bc:ali" is a prefix of "did:bc:alice" — confirm no bleed-over
        store.write_credential(&cred_for_subject("cred-a", "did:bc:ali")).unwrap();
        store.write_credential(&cred_for_subject("cred-b", "did:bc:alice")).unwrap();

        assert_eq!(store.credentials_by_subject_did("did:bc:ali").unwrap().len(), 1);
        assert_eq!(store.credentials_by_subject_did("did:bc:alice").unwrap().len(), 1);
    }

    // ── OrgRegistry (RocksDB) ─────────────────────────────────────────────────

    fn make_org(id: &str) -> Organization {
        Organization::new(
            id,
            &format!("{id}MSP"),
            vec![format!("did:bc:{id}:admin")],
            vec![],
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn org_write_read_roundtrip() {
        let (store, _dir) = tmp_store();
        let org = make_org("org1");
        OrgRegistry::register_org(&store, &org).unwrap();
        let retrieved = OrgRegistry::get_org(&store, "org1").unwrap();
        assert_eq!(retrieved.org_id, "org1");
        assert_eq!(retrieved.msp_id, "org1MSP");
    }

    #[test]
    fn org_list() {
        let (store, _dir) = tmp_store();
        OrgRegistry::register_org(&store, &make_org("org1")).unwrap();
        OrgRegistry::register_org(&store, &make_org("org2")).unwrap();
        let orgs = OrgRegistry::list_orgs(&store).unwrap();
        assert_eq!(orgs.len(), 2);
    }

    #[test]
    fn org_remove() {
        let (store, _dir) = tmp_store();
        OrgRegistry::register_org(&store, &make_org("org1")).unwrap();
        OrgRegistry::remove_org(&store, "org1").unwrap();
        assert!(OrgRegistry::get_org(&store, "org1").is_err());
    }
}
