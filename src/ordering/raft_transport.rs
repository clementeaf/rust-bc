//! Raft network transport — bridges `RaftNode` messages with the P2P layer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use prost::Message as ProstMessage;
use raft::prelude::Message as RaftMsg;

use crate::ordering::raft_node::RaftNode;

/// Serialize a raft `Message` to bytes (protobuf via prost).
pub fn encode_raft_msg(msg: &RaftMsg) -> Vec<u8> {
    msg.encode_to_vec()
}

/// Deserialize bytes back into a raft `Message`.
pub fn decode_raft_msg(data: &[u8]) -> Result<RaftMsg, prost::DecodeError> {
    RaftMsg::decode(data)
}

/// Peer address map: raft node ID → network address string.
pub type PeerMap = Arc<Mutex<HashMap<u64, String>>>;

/// Tick the raft node, advance it, and return serialized outbound messages
/// tagged with their destination node ID.
///
/// Callers are responsible for sending the bytes over the network
/// (e.g. via `Message::RaftMessage`).
pub fn tick_and_collect(node: &mut RaftNode) -> Vec<(u64, Vec<u8>)> {
    node.tick();
    let (msgs, _committed) = node.advance();
    msgs.into_iter()
        .map(|m| {
            let to = m.to;
            (to, encode_raft_msg(&m))
        })
        .collect()
}

/// Deliver a raw raft message (from the network) into the node.
pub fn deliver_raw(node: &mut RaftNode, data: &[u8]) -> Result<(), String> {
    let msg = decode_raft_msg(data).map_err(|e| format!("decode error: {e}"))?;
    node.step(msg).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let mut msg = RaftMsg::default();
        msg.to = 2;
        msg.from = 1;
        let bytes = encode_raft_msg(&msg);
        let decoded = decode_raft_msg(&bytes).unwrap();
        assert_eq!(decoded.to, 2);
        assert_eq!(decoded.from, 1);
    }

    /// Collect advance output as serialized bytes.
    fn collect_bytes(node: &mut RaftNode) -> Vec<(u64, Vec<u8>)> {
        let (msgs, _) = node.advance();
        msgs.into_iter().map(|m| (m.to, encode_raft_msg(&m))).collect()
    }

    /// Deliver serialized messages, recursively delivering responses.
    fn deliver_bytes(nodes: &mut [RaftNode], msgs: Vec<(u64, Vec<u8>)>) {
        if msgs.is_empty() {
            return;
        }
        let mut next: Vec<(u64, Vec<u8>)> = Vec::new();
        for (to, data) in msgs {
            if let Some(target) = nodes.iter_mut().find(|n| n.id == to) {
                let _ = deliver_raw(target, &data);
                next.extend(collect_bytes(target));
            }
        }
        deliver_bytes(nodes, next);
    }

    /// Tick + advance all nodes, then deliver all messages via serialized bytes.
    fn route_bytes(nodes: &mut [RaftNode], rounds: usize) {
        for _ in 0..rounds {
            let mut pending: Vec<(u64, Vec<u8>)> = Vec::new();
            for node in nodes.iter_mut() {
                node.tick();
                pending.extend(collect_bytes(node));
            }
            deliver_bytes(nodes, pending);
        }
    }

    #[test]
    fn three_nodes_in_process_propose_committed_on_all() {
        let peers = vec![1, 2, 3];
        let mut nodes: Vec<RaftNode> = peers
            .iter()
            .map(|&id| RaftNode::new(id, peers.clone()).unwrap())
            .collect();

        // Elect leader.
        route_bytes(&mut nodes, 30);
        let leader_id = nodes.iter().find(|n| n.is_leader()).expect("no leader").id;

        // Propose on leader.
        let leader = nodes.iter_mut().find(|n| n.id == leader_id).unwrap();
        leader.propose(b"transport-test".to_vec()).unwrap();

        // Route until committed on all.
        route_bytes(&mut nodes, 30);

        // All nodes should have the entry.
        for node in &nodes {
            let found = node
                .committed_entries
                .iter()
                .any(|e| e.data == b"transport-test");
            assert!(found, "node {} missing 'transport-test'", node.id);
        }
    }
}
