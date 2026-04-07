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
}
