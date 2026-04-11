//! In-memory BlockStore implementation for testing and development.

use std::collections::HashMap;
use std::sync::Mutex;

use super::errors::{StorageError, StorageResult};
use super::traits::{Block, BlockStore, Credential, IdentityRecord, Transaction};

/// In-memory store backed by HashMaps; safe for concurrent use via Mutex.
pub struct MemoryStore {
    blocks: Mutex<HashMap<u64, Block>>,
    transactions: Mutex<HashMap<String, Transaction>>,
    identities: Mutex<HashMap<String, IdentityRecord>>,
    credentials: Mutex<HashMap<String, Credential>>,
    latest_height: Mutex<u64>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            blocks: Mutex::new(HashMap::new()),
            transactions: Mutex::new(HashMap::new()),
            identities: Mutex::new(HashMap::new()),
            credentials: Mutex::new(HashMap::new()),
            latest_height: Mutex::new(0),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockStore for MemoryStore {
    fn write_block(&self, block: &Block) -> StorageResult<()> {
        let mut blocks = self.blocks.lock().unwrap_or_else(|e| e.into_inner());
        let mut latest = self.latest_height.lock().unwrap_or_else(|e| e.into_inner());
        if block.height > *latest {
            *latest = block.height;
        }
        blocks.insert(block.height, block.clone());
        Ok(())
    }

    fn read_block(&self, height: u64) -> StorageResult<Block> {
        self.blocks
            .lock()
            .unwrap()
            .get(&height)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(format!("BLK:{height:012}")))
    }

    fn write_transaction(&self, tx: &Transaction) -> StorageResult<()> {
        self.transactions
            .lock()
            .unwrap()
            .insert(tx.id.clone(), tx.clone());
        Ok(())
    }

    fn read_transaction(&self, tx_id: &str) -> StorageResult<Transaction> {
        self.transactions
            .lock()
            .unwrap()
            .get(tx_id)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(format!("TX:{tx_id}")))
    }

    fn write_identity(&self, identity: &IdentityRecord) -> StorageResult<()> {
        self.identities
            .lock()
            .unwrap()
            .insert(identity.did.clone(), identity.clone());
        Ok(())
    }

    fn read_identity(&self, did: &str) -> StorageResult<IdentityRecord> {
        self.identities
            .lock()
            .unwrap()
            .get(did)
            .cloned()
            .ok_or_else(|| StorageError::IdentityNotFound(did.to_string()))
    }

    fn write_credential(&self, credential: &Credential) -> StorageResult<()> {
        self.credentials
            .lock()
            .unwrap()
            .insert(credential.id.clone(), credential.clone());
        Ok(())
    }

    fn read_credential(&self, cred_id: &str) -> StorageResult<Credential> {
        self.credentials
            .lock()
            .unwrap()
            .get(cred_id)
            .cloned()
            .ok_or_else(|| StorageError::CredentialNotFound(cred_id.to_string()))
    }

    fn write_batch(&self, blocks: &[Block], txs: &[Transaction]) -> StorageResult<()> {
        if blocks.is_empty() && txs.is_empty() {
            return Err(StorageError::BatchOperationFailed(
                "Empty batch".to_string(),
            ));
        }
        for block in blocks {
            self.write_block(block)?;
        }
        for tx in txs {
            self.write_transaction(tx)?;
        }
        Ok(())
    }

    fn get_latest_height(&self) -> StorageResult<u64> {
        Ok(*self.latest_height.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn block_exists(&self, height: u64) -> StorageResult<bool> {
        Ok(self
            .blocks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&height))
    }

    fn transactions_by_block_height(&self, height: u64) -> StorageResult<Vec<Transaction>> {
        let txs = self
            .transactions
            .lock()
            .unwrap()
            .values()
            .filter(|tx| tx.block_height == height)
            .cloned()
            .collect();
        Ok(txs)
    }

    fn credentials_by_subject_did(&self, subject_did: &str) -> StorageResult<Vec<Credential>> {
        let creds = self
            .credentials
            .lock()
            .unwrap()
            .values()
            .filter(|c| c.subject_did == subject_did)
            .cloned()
            .collect();
        Ok(creds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_block(height: u64) -> Block {
        Block {
            height,
            timestamp: 1_000 + height,
            parent_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            transactions: vec![format!("tx-{}", height)],
            proposer: "node-1".to_string(),
            signature: vec![2u8; 64],
            endorsements: vec![],
            orderer_signature: None,
        }
    }

    fn sample_tx(id: &str, height: u64) -> Transaction {
        Transaction {
            id: id.to_string(),
            block_height: height,
            timestamp: 1_000,
            input_did: "did:bc:sender".to_string(),
            output_recipient: "did:bc:receiver".to_string(),
            amount: 42,
            state: "confirmed".to_string(),
        }
    }

    #[test]
    fn write_and_read_block() {
        let store = MemoryStore::new();
        let block = sample_block(1);
        store.write_block(&block).unwrap();
        let fetched = store.read_block(1).unwrap();
        assert_eq!(fetched.height, 1);
        assert_eq!(fetched.proposer, "node-1");
    }

    #[test]
    fn read_missing_block_returns_error() {
        let store = MemoryStore::new();
        assert!(store.read_block(99).is_err());
    }

    #[test]
    fn latest_height_tracks_max() {
        let store = MemoryStore::new();
        assert_eq!(store.get_latest_height().unwrap(), 0);
        store.write_block(&sample_block(3)).unwrap();
        store.write_block(&sample_block(1)).unwrap();
        store.write_block(&sample_block(5)).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 5);
    }

    #[test]
    fn block_exists_returns_correct_result() {
        let store = MemoryStore::new();
        assert!(!store.block_exists(1).unwrap());
        store.write_block(&sample_block(1)).unwrap();
        assert!(store.block_exists(1).unwrap());
    }

    #[test]
    fn write_and_read_transaction() {
        let store = MemoryStore::new();
        let tx = sample_tx("tx-abc", 1);
        store.write_transaction(&tx).unwrap();
        let fetched = store.read_transaction("tx-abc").unwrap();
        assert_eq!(fetched.amount, 42);
    }

    #[test]
    fn read_missing_transaction_returns_error() {
        let store = MemoryStore::new();
        assert!(store.read_transaction("ghost").is_err());
    }

    #[test]
    fn write_and_read_identity() {
        let store = MemoryStore::new();
        let id = IdentityRecord {
            did: "did:bc:alice".to_string(),
            created_at: 100,
            updated_at: 200,
            status: "active".to_string(),
        };
        store.write_identity(&id).unwrap();
        let fetched = store.read_identity("did:bc:alice").unwrap();
        assert_eq!(fetched.status, "active");
    }

    #[test]
    fn read_missing_identity_returns_error() {
        let store = MemoryStore::new();
        assert!(store.read_identity("did:bc:ghost").is_err());
    }

    #[test]
    fn write_and_read_credential() {
        let store = MemoryStore::new();
        let cred = Credential {
            id: "cred-1".to_string(),
            issuer_did: "did:bc:issuer".to_string(),
            subject_did: "did:bc:subject".to_string(),
            cred_type: "eid".to_string(),
            issued_at: 100,
            expires_at: 999,
            revoked_at: None,
        };
        store.write_credential(&cred).unwrap();
        let fetched = store.read_credential("cred-1").unwrap();
        assert_eq!(fetched.cred_type, "eid");
        assert!(fetched.revoked_at.is_none());
    }

    #[test]
    fn read_missing_credential_returns_error() {
        let store = MemoryStore::new();
        assert!(store.read_credential("ghost").is_err());
    }

    #[test]
    fn write_batch_succeeds() {
        let store = MemoryStore::new();
        let blocks = vec![sample_block(1), sample_block(2)];
        let txs = vec![sample_tx("tx-1", 1), sample_tx("tx-2", 2)];
        store.write_batch(&blocks, &txs).unwrap();
        assert_eq!(store.get_latest_height().unwrap(), 2);
        assert!(store.read_transaction("tx-2").is_ok());
    }

    #[test]
    fn write_batch_empty_returns_error() {
        let store = MemoryStore::new();
        assert!(store.write_batch(&[], &[]).is_err());
    }

    // ── Secondary index: transactions_by_block_height ─────────────────────────

    #[test]
    fn transactions_by_block_height_returns_empty_for_unknown_height() {
        let store = MemoryStore::new();
        assert!(store.transactions_by_block_height(99).unwrap().is_empty());
    }

    #[test]
    fn transactions_by_block_height_filters_correctly() {
        let store = MemoryStore::new();
        store.write_transaction(&sample_tx("tx-1", 3)).unwrap();
        store.write_transaction(&sample_tx("tx-2", 3)).unwrap();
        store.write_transaction(&sample_tx("tx-3", 4)).unwrap();

        let result = store.transactions_by_block_height(3).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|t| t.block_height == 3));
    }

    #[test]
    fn transactions_by_block_height_returns_all_for_height() {
        let store = MemoryStore::new();
        for i in 0..5u64 {
            store
                .write_transaction(&sample_tx(&format!("tx-{i}"), 10))
                .unwrap();
        }
        assert_eq!(store.transactions_by_block_height(10).unwrap().len(), 5);
    }

    // ── Secondary index: credentials_by_subject_did ───────────────────────────

    fn sample_cred(id: &str, subject_did: &str) -> Credential {
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
    fn credentials_by_subject_did_returns_empty_for_unknown_subject() {
        let store = MemoryStore::new();
        assert!(store
            .credentials_by_subject_did("did:bc:ghost")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn credentials_by_subject_did_filters_correctly() {
        let store = MemoryStore::new();
        store
            .write_credential(&sample_cred("cred-1", "did:bc:alice"))
            .unwrap();
        store
            .write_credential(&sample_cred("cred-2", "did:bc:alice"))
            .unwrap();
        store
            .write_credential(&sample_cred("cred-3", "did:bc:bob"))
            .unwrap();

        let alice = store.credentials_by_subject_did("did:bc:alice").unwrap();
        assert_eq!(alice.len(), 2);
        assert!(alice.iter().all(|c| c.subject_did == "did:bc:alice"));

        let bob = store.credentials_by_subject_did("did:bc:bob").unwrap();
        assert_eq!(bob.len(), 1);
        assert_eq!(bob[0].id, "cred-3");
    }
}
