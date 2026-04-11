//! Organization model for endorsement policies

/// Represents a member organization in the endorsement system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Organization {
    /// Unique organization identifier
    pub org_id: String,
    /// Membership Service Provider identifier (Fabric-style)
    pub msp_id: String,
    /// DIDs of admin members
    pub admin_dids: Vec<String>,
    /// DIDs of regular members
    pub member_dids: Vec<String>,
    /// Ed25519 root public keys ([u8; 32]) for this org
    pub root_public_keys: Vec<[u8; 32]>,
}

impl Organization {
    #[allow(dead_code)]
    /// Create a new organization. Returns `None` if `admin_dids` is empty.
    pub fn new(
        org_id: impl Into<String>,
        msp_id: impl Into<String>,
        admin_dids: Vec<String>,
        member_dids: Vec<String>,
        root_public_keys: Vec<[u8; 32]>,
    ) -> Option<Self> {
        if admin_dids.is_empty() {
            return None;
        }
        Some(Organization {
            org_id: org_id.into(),
            msp_id: msp_id.into(),
            admin_dids,
            member_dids,
            root_public_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_org_with_valid_data() {
        let org = Organization::new(
            "org1",
            "Org1MSP",
            vec!["did:bc:admin1".to_string()],
            vec!["did:bc:member1".to_string()],
            vec![[0u8; 32]],
        )
        .unwrap();

        assert_eq!(org.org_id, "org1");
        assert_eq!(org.msp_id, "Org1MSP");
        assert_eq!(org.admin_dids.len(), 1);
        assert_eq!(org.member_dids.len(), 1);
        assert_eq!(org.root_public_keys.len(), 1);
    }

    #[test]
    fn rejects_org_without_admin_dids() {
        let result = Organization::new(
            "org2",
            "Org2MSP",
            vec![],
            vec!["did:bc:member1".to_string()],
            vec![],
        );
        assert!(result.is_none());
    }
}
