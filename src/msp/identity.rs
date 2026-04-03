use serde::{Deserialize, Serialize};

use super::MspRole;

/// A principal identity within an MSP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MspIdentity {
    pub did: String,
    pub org_id: String,
    pub role: MspRole,
    pub public_key: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_identity() {
        let id = MspIdentity {
            did: "did:bc:org1:alice".to_string(),
            org_id: "Org1".to_string(),
            role: MspRole::Member,
            public_key: [7u8; 32],
        };
        assert_eq!(id.did, "did:bc:org1:alice");
        assert_eq!(id.org_id, "Org1");
        assert_eq!(id.role, MspRole::Member);
        assert_eq!(id.public_key, [7u8; 32]);
    }

    #[test]
    fn serde_roundtrip() {
        let id = MspIdentity {
            did: "did:bc:org2:bob".to_string(),
            org_id: "Org2".to_string(),
            role: MspRole::Admin,
            public_key: [255u8; 32],
        };
        let json = serde_json::to_string(&id).unwrap();
        let decoded: MspIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(id, decoded);
    }
}
