//! Policy store trait and in-memory implementation

use std::collections::HashMap;
use std::sync::Mutex;

use crate::storage::errors::{StorageError, StorageResult};

use super::policy::EndorsementPolicy;

/// Trait for persisting endorsement policies keyed by resource ID
pub trait PolicyStore: Send + Sync {
    fn set_policy(&self, resource_id: &str, policy: &EndorsementPolicy) -> StorageResult<()>;
    fn get_policy(&self, resource_id: &str) -> StorageResult<EndorsementPolicy>;
}

/// In-memory policy store
pub struct MemoryPolicyStore {
    inner: Mutex<HashMap<String, EndorsementPolicy>>,
}

impl MemoryPolicyStore {
    pub fn new() -> Self {
        MemoryPolicyStore {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryPolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyStore for MemoryPolicyStore {
    fn set_policy(&self, resource_id: &str, policy: &EndorsementPolicy) -> StorageResult<()> {
        self.inner
            .lock()
            .unwrap()
            .insert(resource_id.to_string(), policy.clone());
        Ok(())
    }

    fn get_policy(&self, resource_id: &str) -> StorageResult<EndorsementPolicy> {
        self.inner
            .lock()
            .unwrap()
            .get(resource_id)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(resource_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::policy::EndorsementPolicy;

    #[test]
    fn set_and_get() {
        let store = MemoryPolicyStore::new();
        let policy = EndorsementPolicy::AnyOf(vec!["org1".into()]);
        store.set_policy("channel/cc1", &policy).unwrap();
        let retrieved = store.get_policy("channel/cc1").unwrap();
        assert_eq!(retrieved, policy);
    }

    #[test]
    fn override_policy() {
        let store = MemoryPolicyStore::new();
        store
            .set_policy("res1", &EndorsementPolicy::AnyOf(vec!["org1".into()]))
            .unwrap();
        let new_policy = EndorsementPolicy::AllOf(vec!["org1".into(), "org2".into()]);
        store.set_policy("res1", &new_policy).unwrap();
        assert_eq!(store.get_policy("res1").unwrap(), new_policy);
    }

    #[test]
    fn get_not_found() {
        let store = MemoryPolicyStore::new();
        assert!(matches!(
            store.get_policy("missing"),
            Err(StorageError::KeyNotFound(_))
        ));
    }
}
