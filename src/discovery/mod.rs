pub mod service;

use serde::{Deserialize, Serialize};

use crate::ordering::NodeRole;

/// Describes a peer registered in the discovery service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerDescriptor {
    /// Network address of the peer (e.g. `"localhost:7051"`).
    pub peer_address: String,
    /// MSP / organization this peer belongs to.
    pub org_id: String,
    /// Whether this node acts as peer, orderer, or both.
    pub role: NodeRole,
    /// Chaincode IDs installed on this peer.
    pub chaincodes: Vec<String>,
    /// Channel IDs this peer participates in.
    pub channels: Vec<String>,
    /// Unix timestamp (seconds) of the last heartbeat from this peer.
    pub last_heartbeat: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PeerDescriptor {
        PeerDescriptor {
            peer_address: "localhost:7051".to_string(),
            org_id: "Org1MSP".to_string(),
            role: NodeRole::Peer,
            chaincodes: vec!["basic".to_string(), "asset-transfer".to_string()],
            channels: vec!["mychannel".to_string()],
            last_heartbeat: 1_700_000_000,
        }
    }

    #[test]
    fn create_descriptor_fields_are_accessible() {
        let d = sample();
        assert_eq!(d.peer_address, "localhost:7051");
        assert_eq!(d.org_id, "Org1MSP");
        assert_eq!(d.role, NodeRole::Peer);
        assert_eq!(d.chaincodes, vec!["basic", "asset-transfer"]);
        assert_eq!(d.channels, vec!["mychannel"]);
        assert_eq!(d.last_heartbeat, 1_700_000_000);
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let original = sample();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: PeerDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn serialize_contains_expected_fields() {
        let d = sample();
        let json = serde_json::to_string(&d).expect("serialize");
        assert!(json.contains("localhost:7051"));
        assert!(json.contains("Org1MSP"));
        assert!(json.contains("basic"));
        assert!(json.contains("mychannel"));
    }
}
