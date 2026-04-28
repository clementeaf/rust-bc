use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use pqc_crypto_module::legacy::sha256::{Digest, Sha256};
use thiserror::Error;

use crate::storage::errors::{StorageError, StorageResult};

// ── Collection struct ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PrivateDataCollection {
    pub name: String,
    pub member_org_ids: Vec<String>,
    pub required_peer_count: usize,
    pub blocks_to_live: u64,
}

impl PrivateDataCollection {
    pub fn new(
        name: impl Into<String>,
        member_org_ids: Vec<String>,
        required_peer_count: usize,
        blocks_to_live: u64,
    ) -> Result<Self, PrivateDataError> {
        let name = name.into();
        if name.is_empty() {
            return Err(PrivateDataError::InvalidCollection(
                "collection name cannot be empty".to_string(),
            ));
        }
        if member_org_ids.is_empty() {
            return Err(PrivateDataError::InvalidCollection(
                "member_org_ids cannot be empty".to_string(),
            ));
        }
        if required_peer_count == 0 {
            return Err(PrivateDataError::InvalidCollection(
                "required_peer_count must be > 0".to_string(),
            ));
        }
        if required_peer_count > member_org_ids.len() {
            return Err(PrivateDataError::InvalidCollection(format!(
                "required_peer_count ({required_peer_count}) exceeds member count ({})",
                member_org_ids.len()
            )));
        }
        Ok(Self {
            name,
            member_org_ids,
            required_peer_count,
            blocks_to_live,
        })
    }

    pub fn is_member(&self, org_id: &str) -> bool {
        self.member_org_ids.iter().any(|id| id == org_id)
    }
}

// ── PrivateDataStore trait ────────────────────────────────────────────────────

/// Stores private data for a named collection.
///
/// The actual bytes are stored in a side store keyed by `(collection_name, key)`.
/// The SHA-256 hash of the bytes is returned from `put` so the caller can embed
/// it on-chain in the TX `data` field for integrity verification.
pub trait PrivateDataStore: Send + Sync {
    /// Store `value` under `(collection_name, key)`.
    ///
    /// Returns the SHA-256 hash of `value` so it can be recorded on-chain.
    fn put_private_data(
        &self,
        collection_name: &str,
        key: &str,
        value: &[u8],
    ) -> StorageResult<[u8; 32]>;

    #[allow(dead_code)]
    /// Store `value` together with TTL metadata so it can later be purged.
    ///
    /// `written_at_height` is the block height at which the data is written.
    /// `blocks_to_live` is how many blocks the data survives before becoming
    /// eligible for purge (0 means never expires).
    ///
    /// Default implementation ignores TTL and delegates to `put_private_data`.
    fn put_private_data_at(
        &self,
        collection_name: &str,
        key: &str,
        value: &[u8],
        written_at_height: u64,
        blocks_to_live: u64,
    ) -> StorageResult<[u8; 32]> {
        let _ = (written_at_height, blocks_to_live); // ignored by default
        self.put_private_data(collection_name, key, value)
    }

    /// Retrieve the bytes previously stored under `(collection_name, key)`.
    fn get_private_data(&self, collection_name: &str, key: &str) -> StorageResult<Option<Vec<u8>>>;

    /// Remove all entries whose TTL has expired.
    ///
    /// An entry expires when `written_at_height + blocks_to_live <= current_height`
    /// (and `blocks_to_live > 0`).
    ///
    /// Default implementation is a no-op (stores that don't track TTL simply
    /// never expire entries in-memory — a future phase can implement RocksDB
    /// compaction filters for durable purge).
    fn purge_expired(&self, current_height: u64) {
        let _ = current_height;
    }
}

// ── SHA-256 helper ────────────────────────────────────────────────────────────

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

// ── In-memory implementation ──────────────────────────────────────────────────

/// In-memory `PrivateDataStore` for tests and single-node dev.
pub struct MemoryPrivateDataStore {
    /// key = (collection_name, entry_key) → value bytes
    data: Mutex<HashMap<(String, String), Vec<u8>>>,
    /// TTL metadata for entries written via `put_private_data_at`.
    /// value = (written_at_height, blocks_to_live)
    ttl: Mutex<HashMap<(String, String), (u64, u64)>>,
}

impl MemoryPrivateDataStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
            ttl: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPrivateDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivateDataStore for MemoryPrivateDataStore {
    fn put_private_data(
        &self,
        collection_name: &str,
        key: &str,
        value: &[u8],
    ) -> StorageResult<[u8; 32]> {
        let hash = sha256(value);
        self.data
            .lock()
            .map_err(|_| StorageError::Other("mutex poisoned".to_string()))?
            .insert(
                (collection_name.to_string(), key.to_string()),
                value.to_vec(),
            );
        Ok(hash)
    }

    fn put_private_data_at(
        &self,
        collection_name: &str,
        key: &str,
        value: &[u8],
        written_at_height: u64,
        blocks_to_live: u64,
    ) -> StorageResult<[u8; 32]> {
        let hash = self.put_private_data(collection_name, key, value)?;
        if blocks_to_live > 0 {
            self.ttl
                .lock()
                .map_err(|_| StorageError::Other("mutex poisoned".to_string()))?
                .insert(
                    (collection_name.to_string(), key.to_string()),
                    (written_at_height, blocks_to_live),
                );
        }
        Ok(hash)
    }

    fn get_private_data(&self, collection_name: &str, key: &str) -> StorageResult<Option<Vec<u8>>> {
        let map = self
            .data
            .lock()
            .map_err(|_| StorageError::Other("mutex poisoned".to_string()))?;
        Ok(map
            .get(&(collection_name.to_string(), key.to_string()))
            .cloned())
    }

    /// Remove all entries whose `blocks_to_live` window has closed.
    ///
    /// An entry expires when `written_at + blocks_to_live <= current_height`.
    fn purge_expired(&self, current_height: u64) {
        let mut ttl_map = self.ttl.lock().expect("ttl mutex poisoned");
        let expired: Vec<(String, String)> = ttl_map
            .iter()
            .filter(|(_, (written_at, btl))| written_at + btl <= current_height)
            .map(|(k, _)| k.clone())
            .collect();

        if expired.is_empty() {
            return;
        }

        let mut data_map = self.data.lock().expect("data mutex poisoned");
        for key in &expired {
            data_map.remove(key);
            ttl_map.remove(key);
        }
    }
}

// ── CollectionRegistry trait ──────────────────────────────────────────────────

/// Registry of `PrivateDataCollection` definitions, keyed by collection name.
pub trait CollectionRegistry: Send + Sync {
    fn register(&self, collection: PrivateDataCollection) -> Result<(), PrivateDataError>;
    fn get(&self, name: &str) -> Option<PrivateDataCollection>;
    /// Return all registered collections.
    fn list(&self) -> Vec<PrivateDataCollection>;
}

/// In-memory `CollectionRegistry` for tests and single-node dev.
pub struct MemoryCollectionRegistry {
    inner: RwLock<HashMap<String, PrivateDataCollection>>,
}

impl MemoryCollectionRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryCollectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectionRegistry for MemoryCollectionRegistry {
    fn register(&self, collection: PrivateDataCollection) -> Result<(), PrivateDataError> {
        self.inner
            .write()
            .unwrap()
            .insert(collection.name.clone(), collection);
        Ok(())
    }

    fn get(&self, name: &str) -> Option<PrivateDataCollection> {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .cloned()
    }

    fn list(&self) -> Vec<PrivateDataCollection> {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }
}

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PrivateDataError {
    #[error("invalid collection: {0}")]
    InvalidCollection(String),
    #[allow(dead_code)]
    #[error("access denied: org '{0}' is not a member of collection '{1}'")]
    AccessDenied(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_collection() -> PrivateDataCollection {
        PrivateDataCollection::new("col1", vec!["org1".to_string(), "org2".to_string()], 1, 100)
            .unwrap()
    }

    // ── PrivateDataCollection tests ───────────────────────────────────────────

    #[test]
    fn creates_collection_with_valid_params() {
        let col = make_collection();
        assert_eq!(col.name, "col1");
        assert_eq!(col.member_org_ids, vec!["org1", "org2"]);
        assert_eq!(col.required_peer_count, 1);
        assert_eq!(col.blocks_to_live, 100);
    }

    #[test]
    fn rejects_empty_name() {
        let result = PrivateDataCollection::new("", vec!["org1".to_string()], 1, 10);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("name cannot be empty"));
    }

    #[test]
    fn rejects_empty_member_list() {
        let result = PrivateDataCollection::new("col1", vec![], 1, 10);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("member_org_ids cannot be empty"));
    }

    #[test]
    fn rejects_zero_required_peer_count() {
        let result = PrivateDataCollection::new("col1", vec!["org1".to_string()], 0, 10);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("required_peer_count must be > 0"));
    }

    #[test]
    fn rejects_required_peer_count_exceeding_members() {
        let result = PrivateDataCollection::new("col1", vec!["org1".to_string()], 3, 10);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds member count"));
    }

    #[test]
    fn member_check_returns_true_for_member() {
        let col = make_collection();
        assert!(col.is_member("org1"));
        assert!(col.is_member("org2"));
    }

    #[test]
    fn member_check_returns_false_for_non_member() {
        let col = make_collection();
        assert!(!col.is_member("org3"));
        assert!(!col.is_member(""));
    }

    // ── MemoryPrivateDataStore tests ──────────────────────────────────────────

    #[test]
    fn put_returns_sha256_hash_of_value() {
        let store = MemoryPrivateDataStore::new();
        let value = b"secret payload";
        let hash = store.put_private_data("col1", "key1", value).unwrap();
        assert_eq!(hash, sha256(value));
    }

    #[test]
    fn get_returns_original_value_after_put() {
        let store = MemoryPrivateDataStore::new();
        let value = b"hello private world";
        store.put_private_data("col1", "key1", value).unwrap();
        let got = store.get_private_data("col1", "key1").unwrap();
        assert_eq!(got, Some(value.to_vec()));
    }

    #[test]
    fn get_returns_none_for_unknown_key() {
        let store = MemoryPrivateDataStore::new();
        let got = store.get_private_data("col1", "missing").unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn collections_are_isolated() {
        let store = MemoryPrivateDataStore::new();
        store.put_private_data("col1", "k", b"alpha").unwrap();
        store.put_private_data("col2", "k", b"beta").unwrap();
        assert_eq!(
            store.get_private_data("col1", "k").unwrap(),
            Some(b"alpha".to_vec())
        );
        assert_eq!(
            store.get_private_data("col2", "k").unwrap(),
            Some(b"beta".to_vec())
        );
    }

    #[test]
    fn put_overwrites_existing_key() {
        let store = MemoryPrivateDataStore::new();
        store.put_private_data("col1", "k", b"v1").unwrap();
        store.put_private_data("col1", "k", b"v2").unwrap();
        assert_eq!(
            store.get_private_data("col1", "k").unwrap(),
            Some(b"v2".to_vec())
        );
    }

    // ── purge_expired tests ───────────────────────────────────────────────────

    #[test]
    fn purge_removes_expired_entry_data_absent_after_purge() {
        let store = MemoryPrivateDataStore::new();
        let value = b"sensitive payload";

        // Write at height 1 with blocks_to_live = 5 → expires when height >= 6.
        let hash = store
            .put_private_data_at("col1", "k1", value, 1, 5)
            .unwrap();
        assert_eq!(hash, sha256(value));

        // Still present before expiry.
        assert_eq!(
            store.get_private_data("col1", "k1").unwrap(),
            Some(value.to_vec())
        );

        // Purge at height 5 — not yet expired (1 + 5 = 6 > 5).
        store.purge_expired(5);
        assert_eq!(
            store.get_private_data("col1", "k1").unwrap(),
            Some(value.to_vec()),
            "should still be present at height 5"
        );

        // Purge at height 6 — now expired (1 + 5 = 6 <= 6).
        store.purge_expired(6);
        assert_eq!(
            store.get_private_data("col1", "k1").unwrap(),
            None,
            "data must be absent after purge"
        );

        // The hash is only on-chain (in the TX data field), not in the store;
        // verify it matches the original value for integrity proof.
        assert_eq!(
            hash,
            sha256(value),
            "on-chain hash must still match original value"
        );
    }

    #[test]
    fn purge_does_not_remove_unexpired_entry() {
        let store = MemoryPrivateDataStore::new();
        store
            .put_private_data_at("col1", "k", b"data", 1, 100)
            .unwrap();
        store.purge_expired(50);
        assert!(store.get_private_data("col1", "k").unwrap().is_some());
    }

    #[test]
    fn purge_removes_only_expired_leaves_others() {
        let store = MemoryPrivateDataStore::new();
        // Expires at height 6.
        store
            .put_private_data_at("col1", "short", b"short-lived", 1, 5)
            .unwrap();
        // Expires at height 101.
        store
            .put_private_data_at("col1", "long", b"long-lived", 1, 100)
            .unwrap();

        store.purge_expired(6);

        assert_eq!(store.get_private_data("col1", "short").unwrap(), None);
        assert_eq!(
            store.get_private_data("col1", "long").unwrap(),
            Some(b"long-lived".to_vec())
        );
    }

    #[test]
    fn entry_without_ttl_is_never_purged() {
        let store = MemoryPrivateDataStore::new();
        store.put_private_data("col1", "k", b"immortal").unwrap();
        store.purge_expired(u64::MAX);
        assert_eq!(
            store.get_private_data("col1", "k").unwrap(),
            Some(b"immortal".to_vec())
        );
    }

    #[test]
    fn zero_blocks_to_live_never_expires() {
        let store = MemoryPrivateDataStore::new();
        store
            .put_private_data_at("col1", "k", b"no-ttl", 1, 0)
            .unwrap();
        store.purge_expired(u64::MAX);
        assert_eq!(
            store.get_private_data("col1", "k").unwrap(),
            Some(b"no-ttl".to_vec())
        );
    }
}
