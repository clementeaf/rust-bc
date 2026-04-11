//! Organization registry trait and in-memory implementation

use std::collections::HashMap;
use std::sync::Mutex;

use crate::storage::errors::{StorageError, StorageResult};

use super::org::Organization;

/// Trait for registering and querying organizations
pub trait OrgRegistry: Send + Sync {
    fn register_org(&self, org: &Organization) -> StorageResult<()>;
    fn get_org(&self, org_id: &str) -> StorageResult<Organization>;
    fn list_orgs(&self) -> StorageResult<Vec<Organization>>;
    fn remove_org(&self, org_id: &str) -> StorageResult<()>;
}

/// In-memory organization registry backed by a `HashMap`
pub struct MemoryOrgRegistry {
    inner: Mutex<HashMap<String, Organization>>,
}

impl MemoryOrgRegistry {
    pub fn new() -> Self {
        MemoryOrgRegistry {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryOrgRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OrgRegistry for MemoryOrgRegistry {
    fn register_org(&self, org: &Organization) -> StorageResult<()> {
        self.inner
            .lock()
            .unwrap()
            .insert(org.org_id.clone(), org.clone());
        Ok(())
    }

    fn get_org(&self, org_id: &str) -> StorageResult<Organization> {
        self.inner
            .lock()
            .unwrap()
            .get(org_id)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(org_id.to_string()))
    }

    fn list_orgs(&self) -> StorageResult<Vec<Organization>> {
        Ok(self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect())
    }

    fn remove_org(&self, org_id: &str) -> StorageResult<()> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if map.remove(org_id).is_none() {
            return Err(StorageError::KeyNotFound(org_id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_org(id: &str) -> Organization {
        Organization::new(
            id,
            format!("{id}MSP"),
            vec![format!("did:bc:{id}:admin")],
            vec![],
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn register_and_get() {
        let reg = MemoryOrgRegistry::new();
        let org = make_org("org1");
        reg.register_org(&org).unwrap();
        let retrieved = reg.get_org("org1").unwrap();
        assert_eq!(retrieved.org_id, "org1");
    }

    #[test]
    fn list_orgs() {
        let reg = MemoryOrgRegistry::new();
        reg.register_org(&make_org("org1")).unwrap();
        reg.register_org(&make_org("org2")).unwrap();
        let orgs = reg.list_orgs().unwrap();
        assert_eq!(orgs.len(), 2);
    }

    #[test]
    fn remove_org() {
        let reg = MemoryOrgRegistry::new();
        reg.register_org(&make_org("org1")).unwrap();
        reg.remove_org("org1").unwrap();
        assert!(reg.get_org("org1").is_err());
    }

    #[test]
    fn get_not_found() {
        let reg = MemoryOrgRegistry::new();
        let result = reg.get_org("nonexistent");
        assert!(matches!(result, Err(StorageError::KeyNotFound(_))));
    }
}
