//! Key-level endorsement policy store
//!
//! Allows individual state keys to carry their own endorsement policy,
//! overriding the chaincode-level policy during endorsement validation.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::storage::errors::{StorageError, StorageResult};

use super::policy::EndorsementPolicy;

// ── Trait ────────────────────────────────────────────────────────────────────

/// Persist and retrieve per-key endorsement policies.
pub trait KeyEndorsementStore: Send + Sync {
    /// Attach `policy` to `key`, overwriting any existing entry.
    fn set_key_policy(&self, key: &str, policy: &EndorsementPolicy) -> StorageResult<()>;

    /// Return the policy for `key`, or `None` if no key-level policy exists.
    fn get_key_policy(&self, key: &str) -> StorageResult<Option<EndorsementPolicy>>;

    #[allow(dead_code)]
    /// Remove the key-level policy for `key` (no-op if absent).
    fn delete_key_policy(&self, key: &str) -> StorageResult<()>;
}

// ── In-memory implementation ─────────────────────────────────────────────────

/// In-memory key-level endorsement policy store (test / dev use).
pub struct MemoryKeyEndorsementStore {
    inner: Mutex<HashMap<String, EndorsementPolicy>>,
}

impl MemoryKeyEndorsementStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryKeyEndorsementStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyEndorsementStore for MemoryKeyEndorsementStore {
    fn set_key_policy(&self, key: &str, policy: &EndorsementPolicy) -> StorageResult<()> {
        self.inner
            .lock()
            .unwrap()
            .insert(key.to_string(), policy.clone());
        Ok(())
    }

    fn get_key_policy(&self, key: &str) -> StorageResult<Option<EndorsementPolicy>> {
        Ok(self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned())
    }

    fn delete_key_policy(&self, key: &str) -> StorageResult<()> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
        Ok(())
    }
}

// ── RocksDB implementation ────────────────────────────────────────────────────

impl KeyEndorsementStore for crate::storage::adapters::RocksDbBlockStore {
    fn set_key_policy(&self, key: &str, policy: &EndorsementPolicy) -> StorageResult<()> {
        let cf = self.cf_key_endorsement_policies()?;
        let value = serde_json::to_vec(policy)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.db
            .put_cf(&cf, key.as_bytes(), &value)
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }

    fn get_key_policy(&self, key: &str) -> StorageResult<Option<EndorsementPolicy>> {
        let cf = self.cf_key_endorsement_policies()?;
        match self
            .db
            .get_cf(&cf, key.as_bytes())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))?
        {
            None => Ok(None),
            Some(bytes) => {
                let policy = serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
                Ok(Some(policy))
            }
        }
    }

    fn delete_key_policy(&self, key: &str) -> StorageResult<()> {
        let cf = self.cf_key_endorsement_policies()?;
        self.db
            .delete_cf(&cf, key.as_bytes())
            .map_err(|e| StorageError::RocksDbError(e.to_string()))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::policy::EndorsementPolicy;

    fn any_of(orgs: &[&str]) -> EndorsementPolicy {
        EndorsementPolicy::AnyOf(orgs.iter().map(|s| s.to_string()).collect())
    }

    // ── MemoryKeyEndorsementStore ─────────────────────────────────────────────

    #[test]
    fn memory_set_and_get() {
        let store = MemoryKeyEndorsementStore::new();
        store
            .set_key_policy("asset:123", &any_of(&["org1"]))
            .unwrap();
        let got = store.get_key_policy("asset:123").unwrap();
        assert_eq!(got, Some(any_of(&["org1"])));
    }

    #[test]
    fn memory_get_missing_returns_none() {
        let store = MemoryKeyEndorsementStore::new();
        assert!(store.get_key_policy("missing").unwrap().is_none());
    }

    #[test]
    fn memory_overwrite_policy() {
        let store = MemoryKeyEndorsementStore::new();
        store.set_key_policy("k", &any_of(&["org1"])).unwrap();
        store.set_key_policy("k", &any_of(&["org2"])).unwrap();
        assert_eq!(store.get_key_policy("k").unwrap(), Some(any_of(&["org2"])));
    }

    #[test]
    fn memory_delete_policy() {
        let store = MemoryKeyEndorsementStore::new();
        store.set_key_policy("k", &any_of(&["org1"])).unwrap();
        store.delete_key_policy("k").unwrap();
        assert!(store.get_key_policy("k").unwrap().is_none());
    }

    #[test]
    fn memory_delete_missing_is_noop() {
        let store = MemoryKeyEndorsementStore::new();
        assert!(store.delete_key_policy("nonexistent").is_ok());
    }

    // ── RocksDbBlockStore ─────────────────────────────────────────────────────

    #[test]
    fn rocksdb_set_and_get() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        store
            .set_key_policy("asset:color", &any_of(&["org1"]))
            .unwrap();
        let got = store.get_key_policy("asset:color").unwrap();
        assert_eq!(got, Some(any_of(&["org1"])));
    }

    #[test]
    fn rocksdb_get_missing_returns_none() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        assert!(store.get_key_policy("nonexistent").unwrap().is_none());
    }

    #[test]
    fn rocksdb_overwrite_policy() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        store.set_key_policy("k", &any_of(&["org1"])).unwrap();
        store.set_key_policy("k", &any_of(&["org2"])).unwrap();
        assert_eq!(store.get_key_policy("k").unwrap(), Some(any_of(&["org2"])));
    }

    #[test]
    fn rocksdb_delete_policy() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        store.set_key_policy("k", &any_of(&["org1"])).unwrap();
        store.delete_key_policy("k").unwrap();
        assert!(store.get_key_policy("k").unwrap().is_none());
    }

    #[test]
    fn rocksdb_delete_missing_is_noop() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        assert!(store.delete_key_policy("nonexistent").is_ok());
    }

    #[test]
    fn rocksdb_policies_isolated_from_world_state() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = crate::storage::adapters::RocksDbBlockStore::new(dir.path()).unwrap();
        // Write a key-level policy for "asset:x"
        store.set_key_policy("asset:x", &any_of(&["org1"])).unwrap();
        // The same key in world state must be absent
        let ws_val = store.world_state_get("asset:x").unwrap();
        assert!(
            ws_val.is_none(),
            "key-level policy must not bleed into world state"
        );
    }
}
