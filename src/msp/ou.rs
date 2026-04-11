//! Organizational Units (OUs) for subdividing organizations.
//!
//! OUs form a hierarchy within an org: each OU can have an optional parent OU,
//! enabling fine-grained endorsement policies (e.g. "2 from manufacturing OU").

use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::storage::errors::StorageResult;

/// An organizational unit within an MSP organization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationalUnit {
    pub ou_id: String,
    pub org_id: String,
    pub description: String,
    pub parent_ou: Option<String>,
}

/// Registry for organizational unit definitions.
pub trait OuRegistry: Send + Sync {
    #[allow(dead_code)]
    fn register_ou(&self, ou: &OrganizationalUnit) -> StorageResult<()>;
    #[allow(dead_code)]
    fn get_ou(&self, ou_id: &str) -> StorageResult<Option<OrganizationalUnit>>;
    #[allow(dead_code)]
    fn list_ous(&self, org_id: &str) -> StorageResult<Vec<OrganizationalUnit>>;
    #[allow(dead_code)]
    /// Return the hierarchy chain from `ou_id` up to the root OU.
    fn get_hierarchy(&self, ou_id: &str) -> StorageResult<Vec<OrganizationalUnit>>;
}

/// In-memory OU registry for testing and single-node dev.
pub struct MemoryOuRegistry {
    inner: RwLock<HashMap<String, OrganizationalUnit>>,
}

impl MemoryOuRegistry {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryOuRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OuRegistry for MemoryOuRegistry {
    fn register_ou(&self, ou: &OrganizationalUnit) -> StorageResult<()> {
        self.inner
            .write()
            .unwrap()
            .insert(ou.ou_id.clone(), ou.clone());
        Ok(())
    }

    fn get_ou(&self, ou_id: &str) -> StorageResult<Option<OrganizationalUnit>> {
        Ok(self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(ou_id)
            .cloned())
    }

    fn list_ous(&self, org_id: &str) -> StorageResult<Vec<OrganizationalUnit>> {
        let map = self.inner.read().unwrap_or_else(|e| e.into_inner());
        Ok(map
            .values()
            .filter(|ou| ou.org_id == org_id)
            .cloned()
            .collect())
    }

    fn get_hierarchy(&self, ou_id: &str) -> StorageResult<Vec<OrganizationalUnit>> {
        let map = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let mut chain = Vec::new();
        let mut current = ou_id.to_string();
        // Walk up the parent chain (limit to 100 to prevent cycles).
        for _ in 0..100 {
            match map.get(&current) {
                Some(ou) => {
                    chain.push(ou.clone());
                    match &ou.parent_ou {
                        Some(parent) => current = parent.clone(),
                        None => break,
                    }
                }
                None => break,
            }
        }
        Ok(chain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ou(ou_id: &str, org_id: &str, parent: Option<&str>) -> OrganizationalUnit {
        OrganizationalUnit {
            ou_id: ou_id.into(),
            org_id: org_id.into(),
            description: format!("{ou_id} unit"),
            parent_ou: parent.map(String::from),
        }
    }

    #[test]
    fn create_ou_and_serde_roundtrip() {
        let ou = make_ou("mfg", "org1", None);
        let json = serde_json::to_string(&ou).unwrap();
        let decoded: OrganizationalUnit = serde_json::from_str(&json).unwrap();
        assert_eq!(ou, decoded);
    }

    #[test]
    fn ou_with_parent() {
        let ou = make_ou("mfg-line1", "org1", Some("mfg"));
        assert_eq!(ou.parent_ou, Some("mfg".into()));
    }

    #[test]
    fn register_and_get() {
        let reg = MemoryOuRegistry::new();
        let ou = make_ou("eng", "org1", None);
        reg.register_ou(&ou).unwrap();

        let got = reg.get_ou("eng").unwrap().unwrap();
        assert_eq!(got.ou_id, "eng");
    }

    #[test]
    fn list_by_org() {
        let reg = MemoryOuRegistry::new();
        reg.register_ou(&make_ou("eng", "org1", None)).unwrap();
        reg.register_ou(&make_ou("mfg", "org1", None)).unwrap();
        reg.register_ou(&make_ou("sales", "org2", None)).unwrap();

        let org1_ous = reg.list_ous("org1").unwrap();
        assert_eq!(org1_ous.len(), 2);

        let org2_ous = reg.list_ous("org2").unwrap();
        assert_eq!(org2_ous.len(), 1);
    }

    #[test]
    fn hierarchy_traversal() {
        let reg = MemoryOuRegistry::new();
        reg.register_ou(&make_ou("root", "org1", None)).unwrap();
        reg.register_ou(&make_ou("mfg", "org1", Some("root")))
            .unwrap();
        reg.register_ou(&make_ou("line1", "org1", Some("mfg")))
            .unwrap();

        let chain = reg.get_hierarchy("line1").unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].ou_id, "line1");
        assert_eq!(chain[1].ou_id, "mfg");
        assert_eq!(chain[2].ou_id, "root");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let reg = MemoryOuRegistry::new();
        assert!(reg.get_ou("nope").unwrap().is_none());
    }

    #[test]
    fn hierarchy_stops_at_root() {
        let reg = MemoryOuRegistry::new();
        reg.register_ou(&make_ou("root", "org1", None)).unwrap();
        let chain = reg.get_hierarchy("root").unwrap();
        assert_eq!(chain.len(), 1);
    }
}
