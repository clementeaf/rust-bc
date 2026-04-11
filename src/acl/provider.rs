//! ACL provider trait and in-memory implementation.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::acl::AclEntry;
use crate::storage::errors::{StorageError, StorageResult};

/// Trait for reading and writing ACL entries.
pub trait AclProvider: Send + Sync {
    fn set_acl(&self, resource: &str, policy_ref: &str) -> StorageResult<()>;
    fn get_acl(&self, resource: &str) -> StorageResult<Option<AclEntry>>;
    fn list_acls(&self) -> StorageResult<Vec<AclEntry>>;
    #[allow(dead_code)] // API variant — may be used by external callers
    fn remove_acl(&self, resource: &str) -> StorageResult<()>;
}

/// In-memory [`AclProvider`] backed by a `Mutex<HashMap>`.
#[derive(Default)]
pub struct MemoryAclProvider {
    entries: Mutex<HashMap<String, AclEntry>>,
}

impl MemoryAclProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AclProvider for MemoryAclProvider {
    fn set_acl(&self, resource: &str, policy_ref: &str) -> StorageResult<()> {
        let mut map = self
            .entries
            .lock()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        map.insert(resource.to_string(), AclEntry::new(resource, policy_ref));
        Ok(())
    }

    fn get_acl(&self, resource: &str) -> StorageResult<Option<AclEntry>> {
        let map = self
            .entries
            .lock()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        Ok(map.get(resource).cloned())
    }

    fn list_acls(&self) -> StorageResult<Vec<AclEntry>> {
        let map = self
            .entries
            .lock()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        Ok(map.values().cloned().collect())
    }

    fn remove_acl(&self, resource: &str) -> StorageResult<()> {
        let mut map = self
            .entries
            .lock()
            .map_err(|e| StorageError::Other(e.to_string()))?;
        map.remove(resource);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> MemoryAclProvider {
        MemoryAclProvider::new()
    }

    #[test]
    fn set_and_get() {
        let p = provider();
        p.set_acl("peer/ChaincodeInvoke", "OrgPolicy").unwrap();
        let entry = p.get_acl("peer/ChaincodeInvoke").unwrap().expect("entry");
        assert_eq!(entry.resource, "peer/ChaincodeInvoke");
        assert_eq!(entry.policy_ref, "OrgPolicy");
    }

    #[test]
    fn get_not_found_returns_none() {
        let p = provider();
        let entry = p.get_acl("nonexistent").unwrap();
        assert!(entry.is_none());
    }

    #[test]
    fn list_returns_all_entries() {
        let p = provider();
        p.set_acl("peer/BlockEvents", "PolicyA").unwrap();
        p.set_acl("peer/ChaincodeInvoke", "PolicyB").unwrap();
        let mut list = p.list_acls().unwrap();
        list.sort_by(|a, b| a.resource.cmp(&b.resource));
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].resource, "peer/BlockEvents");
        assert_eq!(list[1].resource, "peer/ChaincodeInvoke");
    }

    #[test]
    fn remove_acl() {
        let p = provider();
        p.set_acl("peer/PrivateData", "PolicyA").unwrap();
        p.remove_acl("peer/PrivateData").unwrap();
        assert!(p.get_acl("peer/PrivateData").unwrap().is_none());
    }

    #[test]
    fn set_overwrites_existing() {
        let p = provider();
        p.set_acl("peer/BlockEvents", "PolicyA").unwrap();
        p.set_acl("peer/BlockEvents", "PolicyB").unwrap();
        let entry = p.get_acl("peer/BlockEvents").unwrap().expect("entry");
        assert_eq!(entry.policy_ref, "PolicyB");
    }
}
