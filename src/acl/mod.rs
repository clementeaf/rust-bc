//! Access Control List (ACL) framework.
//!
//! Each [`AclEntry`] maps a resource name to a named [`EndorsementPolicy`]
//! that governs who may access that resource.

pub mod provider;
pub mod checker;
pub mod resources;
pub use provider::{AclProvider, MemoryAclProvider};
pub use checker::{check_access, AclError};
pub use resources::AclResource;

use serde::{Deserialize, Serialize};

/// An ACL entry binding a resource to a named endorsement policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AclEntry {
    /// Canonical resource identifier (e.g. `"peer/ChaincodeToChaincode"`).
    pub resource: String,
    /// Name of the [`EndorsementPolicy`] that governs this resource.
    pub policy_ref: String,
}

impl AclEntry {
    /// Create a new [`AclEntry`].
    pub fn new(resource: impl Into<String>, policy_ref: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            policy_ref: policy_ref.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_acl_entry() {
        let entry = AclEntry::new("peer/ChaincodeToChaincode", "ChannelMemberPolicy");
        assert_eq!(entry.resource, "peer/ChaincodeToChaincode");
        assert_eq!(entry.policy_ref, "ChannelMemberPolicy");
    }

    #[test]
    fn serde_roundtrip() {
        let entry = AclEntry::new("peer/BlockEvents", "OrgAdminPolicy");
        let json = serde_json::to_string(&entry).expect("serialize");
        let restored: AclEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(entry, restored);
    }
}
