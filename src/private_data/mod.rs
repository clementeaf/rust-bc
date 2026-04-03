use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::storage::errors::{StorageError, StorageResult};

// ── Collection struct ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Retrieve the bytes previously stored under `(collection_name, key)`.
    fn get_private_data(
        &self,
        collection_name: &str,
        key: &str,
    ) -> StorageResult<Option<Vec<u8>>>;
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
    /// key = (collection_name, entry_key)
    data: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl MemoryPrivateDataStore {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
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
            .insert((collection_name.to_string(), key.to_string()), value.to_vec());
        Ok(hash)
    }

    fn get_private_data(
        &self,
        collection_name: &str,
        key: &str,
    ) -> StorageResult<Option<Vec<u8>>> {
        let map = self
            .data
            .lock()
            .map_err(|_| StorageError::Other("mutex poisoned".to_string()))?;
        Ok(map.get(&(collection_name.to_string(), key.to_string())).cloned())
    }
}

// ── CollectionRegistry trait ──────────────────────────────────────────────────

/// Registry of `PrivateDataCollection` definitions, keyed by collection name.
pub trait CollectionRegistry: Send + Sync {
    fn register(&self, collection: PrivateDataCollection) -> Result<(), PrivateDataError>;
    fn get(&self, name: &str) -> Option<PrivateDataCollection>;
}

/// In-memory `CollectionRegistry` for tests and single-node dev.
pub struct MemoryCollectionRegistry {
    inner: RwLock<HashMap<String, PrivateDataCollection>>,
}

impl MemoryCollectionRegistry {
    pub fn new() -> Self {
        Self { inner: RwLock::new(HashMap::new()) }
    }
}

impl Default for MemoryCollectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectionRegistry for MemoryCollectionRegistry {
    fn register(&self, collection: PrivateDataCollection) -> Result<(), PrivateDataError> {
        self.inner.write().unwrap().insert(collection.name.clone(), collection);
        Ok(())
    }

    fn get(&self, name: &str) -> Option<PrivateDataCollection> {
        self.inner.read().unwrap().get(name).cloned()
    }
}

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PrivateDataError {
    #[error("invalid collection: {0}")]
    InvalidCollection(String),
    #[error("access denied: org '{0}' is not a member of collection '{1}'")]
    AccessDenied(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_collection() -> PrivateDataCollection {
        PrivateDataCollection::new(
            "col1",
            vec!["org1".to_string(), "org2".to_string()],
            1,
            100,
        )
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
        assert!(result.unwrap_err().to_string().contains("name cannot be empty"));
    }

    #[test]
    fn rejects_empty_member_list() {
        let result = PrivateDataCollection::new("col1", vec![], 1, 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("member_org_ids cannot be empty"));
    }

    #[test]
    fn rejects_zero_required_peer_count() {
        let result = PrivateDataCollection::new("col1", vec!["org1".to_string()], 0, 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("required_peer_count must be > 0"));
    }

    #[test]
    fn rejects_required_peer_count_exceeding_members() {
        let result = PrivateDataCollection::new("col1", vec!["org1".to_string()], 3, 10);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds member count"));
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
}
