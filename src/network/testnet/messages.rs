//! Network message types for the minimal testnet.

use serde::{Deserialize, Serialize};

use crate::transaction::compact_block::{
    CompactBlock, MissingCompactRequest, MissingCompactResponse, SegWitBlock,
};
use crate::transaction::segwit::{TxCore, TxWitness};

/// Messages exchanged between testnet nodes over TCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Broadcast a new SegWit transaction (core + witness).
    NewTransaction(TxCore, TxWitness),
    /// Broadcast a full SegWit block (fallback, used for sync).
    NewBlock(SegWitBlock),
    /// Broadcast a compact block (primary propagation).
    CompactBlock(CompactBlock),
    /// Request missing txs/witnesses from a compact block.
    RequestMissing(MissingCompactRequest),
    /// Response with missing txs/witnesses.
    ResponseMissing(MissingCompactResponse),
    /// Request full chain sync from genesis.
    SyncRequest,
    /// Response with all blocks.
    SyncResponse(Vec<SegWitBlock>),

    // ── CLI control messages ───────────────────────────────────────────
    /// CLI: trigger block mining on this node.
    MineBlock,
    /// CLI: query account balance.
    QueryBalance { address: String },
    /// Response to QueryBalance.
    BalanceResponse {
        address: String,
        balance: u64,
        nonce: u64,
    },
    /// Response to MineBlock.
    MineBlockResponse { height: u64, tx_count: usize },
}

/// Encode a message as length-prefixed JSON for TCP transport.
pub fn encode_message(msg: &NetworkMessage) -> Vec<u8> {
    let json = serde_json::to_vec(msg).expect("message serialization must not fail");
    let len = (json.len() as u32).to_be_bytes();
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len);
    buf.extend_from_slice(&json);
    buf
}

/// Decode a length-prefixed JSON message from a buffer.
/// Returns the message and the number of bytes consumed.
pub fn decode_message(buf: &[u8]) -> Option<(NetworkMessage, usize)> {
    if buf.len() < 4 {
        return None;
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if buf.len() < 4 + len {
        return None;
    }
    let msg: NetworkMessage = serde_json::from_slice(&buf[4..4 + len]).ok()?;
    Some((msg, 4 + len))
}
