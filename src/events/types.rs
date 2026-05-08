//! `BlockEvent` enum — events emitted by the node's commit path.

use serde::{Deserialize, Serialize};

/// Events that can be published on the [`super::EventBus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockEvent {
    /// A block was committed to the store.
    BlockCommitted {
        channel_id: String,
        height: u64,
        tx_count: usize,
    },
    /// A single transaction was committed inside a block.
    TransactionCommitted {
        channel_id: String,
        tx_id: String,
        block_height: u64,
        valid: bool,
    },
    /// A chaincode emitted a named event during execution.
    ChaincodeEvent {
        channel_id: String,
        chaincode_id: String,
        event_name: String,
        payload: Vec<u8>,
    },

    // ── Security events (for CSIRT/SIEM integration) ─────────────────────────
    /// An ACL check denied a request.
    AclDenied {
        resource: String,
        identity: String,
        reason: String,
    },
    /// A validator produced conflicting proposals (equivocation).
    EquivocationDetected {
        proposer: String,
        height: u64,
        slot: u64,
    },
    /// Rate limiter rejected a request.
    RateLimitExceeded { source_ip: String, endpoint: String },
    /// A block or transaction signature failed verification.
    InvalidSignature {
        entity: String,
        algorithm: String,
        reason: String,
    },
    /// A validator was penalized (slashing).
    ValidatorSlashed {
        validator: String,
        reason: String,
        penalty_height: u64,
    },
}

impl BlockEvent {
    /// Returns `true` if this is a security-relevant event suitable for
    /// CSIRT/SIEM forwarding.
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            BlockEvent::AclDenied { .. }
                | BlockEvent::EquivocationDetected { .. }
                | BlockEvent::RateLimitExceeded { .. }
                | BlockEvent::InvalidSignature { .. }
                | BlockEvent::ValidatorSlashed { .. }
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(event: &BlockEvent) -> BlockEvent {
        let json = serde_json::to_string(event).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn block_committed_roundtrip() {
        let event = BlockEvent::BlockCommitted {
            channel_id: "ch1".to_string(),
            height: 7,
            tx_count: 3,
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn transaction_committed_roundtrip() {
        let event = BlockEvent::TransactionCommitted {
            channel_id: "ch1".to_string(),
            tx_id: "tx-abc".to_string(),
            block_height: 7,
            valid: true,
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn chaincode_event_roundtrip() {
        let event = BlockEvent::ChaincodeEvent {
            channel_id: "ch1".to_string(),
            chaincode_id: "basic".to_string(),
            event_name: "Transfer".to_string(),
            payload: b"hello world".to_vec(),
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn block_committed_json_contains_type_tag() {
        let event = BlockEvent::BlockCommitted {
            channel_id: "".to_string(),
            height: 1,
            tx_count: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"block_committed\""));
    }

    #[test]
    fn transaction_committed_json_contains_type_tag() {
        let event = BlockEvent::TransactionCommitted {
            channel_id: "".to_string(),
            tx_id: "t1".to_string(),
            block_height: 1,
            valid: false,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"transaction_committed\""));
    }

    #[test]
    fn chaincode_event_json_contains_type_tag() {
        let event = BlockEvent::ChaincodeEvent {
            channel_id: "".to_string(),
            chaincode_id: "cc".to_string(),
            event_name: "evt".to_string(),
            payload: vec![],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"chaincode_event\""));
    }

    // ── Security event tests ─────────────────────────────────────────────────

    #[test]
    fn acl_denied_roundtrip() {
        let event = BlockEvent::AclDenied {
            resource: "/api/v1/blocks".into(),
            identity: "did:cerulean:abc".into(),
            reason: "missing X-Org-Id".into(),
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn equivocation_detected_roundtrip() {
        let event = BlockEvent::EquivocationDetected {
            proposer: "validator-1".into(),
            height: 100,
            slot: 5,
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn rate_limit_exceeded_roundtrip() {
        let event = BlockEvent::RateLimitExceeded {
            source_ip: "192.168.1.100".into(),
            endpoint: "/api/v1/transactions".into(),
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn invalid_signature_roundtrip() {
        let event = BlockEvent::InvalidSignature {
            entity: "block-42".into(),
            algorithm: "ml-dsa-65".into(),
            reason: "size mismatch".into(),
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn validator_slashed_roundtrip() {
        let event = BlockEvent::ValidatorSlashed {
            validator: "validator-3".into(),
            reason: "equivocation".into(),
            penalty_height: 200,
        };
        assert_eq!(roundtrip(&event), event);
    }

    #[test]
    fn is_security_event_returns_true_for_security_variants() {
        assert!(BlockEvent::AclDenied {
            resource: "".into(),
            identity: "".into(),
            reason: "".into(),
        }
        .is_security_event());

        assert!(BlockEvent::EquivocationDetected {
            proposer: "".into(),
            height: 0,
            slot: 0,
        }
        .is_security_event());

        assert!(BlockEvent::RateLimitExceeded {
            source_ip: "".into(),
            endpoint: "".into(),
        }
        .is_security_event());

        assert!(BlockEvent::InvalidSignature {
            entity: "".into(),
            algorithm: "".into(),
            reason: "".into(),
        }
        .is_security_event());

        assert!(BlockEvent::ValidatorSlashed {
            validator: "".into(),
            reason: "".into(),
            penalty_height: 0,
        }
        .is_security_event());
    }

    #[test]
    fn is_security_event_returns_false_for_block_events() {
        assert!(!BlockEvent::BlockCommitted {
            channel_id: "".into(),
            height: 0,
            tx_count: 0,
        }
        .is_security_event());

        assert!(!BlockEvent::TransactionCommitted {
            channel_id: "".into(),
            tx_id: "".into(),
            block_height: 0,
            valid: true,
        }
        .is_security_event());
    }
}
