//! Core types for the cross-chain bridge: chain identifiers, message envelopes,
//! transfer records, and proof structures.

use serde::{Deserialize, Serialize};

// ── Chain Identity ──────────────────────────────────────────────────────────

/// Unique identifier for a blockchain network.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl ChainId {
    /// The native rust-bc chain.
    pub fn native() -> Self {
        Self("rust-bc".to_string())
    }
}

impl std::fmt::Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Registered external chain with connection metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: ChainId,
    /// Human-readable name (e.g. "Ethereum Mainnet").
    pub name: String,
    /// Bridge protocol type used for this chain.
    pub protocol: BridgeType,
    /// Whether the bridge to this chain is currently active.
    pub active: bool,
    /// Minimum confirmations required on the source chain before accepting.
    pub min_confirmations: u64,
    /// Maximum transfer amount per transaction (0 = unlimited).
    pub max_transfer: u64,
}

/// Supported bridge protocol types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BridgeType {
    /// IBC-style light client verification.
    LightClient,
    /// Relay-based with multisig committee.
    Relay,
    /// Hash time-locked contracts (atomic swaps).
    Htlc,
}

// ── Cross-Chain Message ─────────────────────────────────────────────────────

/// Unique identifier for a cross-chain message.
pub type MessageId = [u8; 32];

/// A cross-chain message envelope.
///
/// This is the canonical format for all bridge communication.
/// It wraps a payload with routing, sequencing, and proof metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeMessage {
    /// Unique message ID (SHA-256 of contents).
    pub id: MessageId,
    /// Source chain.
    pub source_chain: ChainId,
    /// Destination chain.
    pub dest_chain: ChainId,
    /// Monotonic sequence number per (source, dest) pair.
    pub sequence: u64,
    /// Message payload.
    pub payload: MessagePayload,
    /// Block height on the source chain where this message was emitted.
    pub source_height: u64,
    /// Timestamp of the source block.
    pub source_timestamp: u64,
    /// Proof that the message was included in the source chain.
    pub proof: Option<InclusionProof>,
}

/// Payload types for cross-chain messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Token transfer: lock on source, mint on destination.
    TokenTransfer {
        sender: String,
        recipient: String,
        amount: u64,
        /// Token denomination (e.g. "NOTA", "ETH").
        denom: String,
    },
    /// Arbitrary data relay (for cross-chain contract calls).
    DataRelay {
        contract_id: String,
        function: String,
        data: Vec<u8>,
    },
    /// Acknowledgement of a previously received message.
    Ack {
        /// The message ID being acknowledged.
        original_id: MessageId,
        success: bool,
        /// Optional error message if !success.
        error: Option<String>,
    },
}

// ── Transfer Record ─────────────────────────────────────────────────────────

/// State of a cross-chain transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Tokens locked on source chain, waiting for proof verification.
    Pending,
    /// Proof verified, tokens minted/released on destination.
    Completed,
    /// Transfer failed or timed out, tokens refunded on source.
    Refunded,
    /// Transfer expired without completion.
    Expired,
}

/// Record of a cross-chain token transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRecord {
    pub message_id: MessageId,
    pub source_chain: ChainId,
    pub dest_chain: ChainId,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub denom: String,
    pub status: TransferStatus,
    /// Block height when the transfer was initiated.
    pub created_at: u64,
    /// Block height when the transfer was finalized (completed/refunded).
    pub finalized_at: Option<u64>,
}

// ── Inclusion Proof ─────────────────────────────────────────────────────────

/// Proof that a message was included in a source chain block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InclusionProof {
    /// Merkle proof path (list of sibling hashes from leaf to root).
    pub merkle_path: Vec<[u8; 32]>,
    /// Index of the leaf in the Merkle tree.
    pub leaf_index: u64,
    /// The Merkle root this proof verifies against.
    pub root: [u8; 32],
    /// Block header hash from the source chain.
    pub block_hash: [u8; 32],
    /// Block height on the source chain.
    pub block_height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_display() {
        let id = ChainId("ethereum".into());
        assert_eq!(format!("{id}"), "ethereum");
    }

    #[test]
    fn native_chain_id() {
        assert_eq!(ChainId::native().0, "rust-bc");
    }

    #[test]
    fn message_payload_serde_roundtrip() {
        let payload = MessagePayload::TokenTransfer {
            sender: "alice".into(),
            recipient: "bob".into(),
            amount: 1000,
            denom: "NOTA".into(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: MessagePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, back);
    }

    #[test]
    fn transfer_record_serde_roundtrip() {
        let record = TransferRecord {
            message_id: [1u8; 32],
            source_chain: ChainId::native(),
            dest_chain: ChainId("ethereum".into()),
            sender: "alice".into(),
            recipient: "0xBob".into(),
            amount: 500,
            denom: "NOTA".into(),
            status: TransferStatus::Pending,
            created_at: 100,
            finalized_at: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: TransferRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);
    }

    #[test]
    fn bridge_type_variants() {
        let types = [BridgeType::LightClient, BridgeType::Relay, BridgeType::Htlc];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: BridgeType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }

    #[test]
    fn chain_config_serde() {
        let config = ChainConfig {
            chain_id: ChainId("cosmos".into()),
            name: "Cosmos Hub".into(),
            protocol: BridgeType::LightClient,
            active: true,
            min_confirmations: 10,
            max_transfer: 1_000_000,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: ChainConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, back);
    }
}
