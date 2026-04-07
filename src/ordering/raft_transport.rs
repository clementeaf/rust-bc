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

/// Parse a `RAFT_PEERS` string into a `PeerMap`.
///
/// Format: `"1:orderer1:8087,2:orderer2:8087,3:orderer3:8087"`
/// Each entry is `{raft_id}:{host}:{port}`.
pub fn parse_raft_peers(s: &str) -> HashMap<u64, String> {
    let mut map = HashMap::new();
    for entry in s.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        // Split on first ':' only — the rest is the address.
        if let Some((id_str, addr)) = entry.split_once(':') {
            if let Ok(id) = id_str.parse::<u64>() {
                map.insert(id, addr.to_string());
            }
        }
    }
    map
}

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

/// Spawn a background loop that ticks the Raft node every `tick_ms` and sends
/// outbound consensus messages to peers via P2P.
///
/// The loop:
/// 1. Locks the `RaftNode` and calls `tick_and_collect()`.
/// 2. For each outbound message, looks up the destination in `peer_map`.
/// 3. Sends `Message::RaftMessage(bytes)` to the peer via `Node::send_and_wait`
///    (fire-and-forget — we ignore the response).
pub fn start_raft_tick_loop(
    raft_node: Arc<Mutex<RaftNode>>,
    peer_map: PeerMap,
    p2p_node: Arc<crate::network::Node>,
    tick_ms: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(tick_ms));
        loop {
            interval.tick().await;

            let outbound = {
                let mut node = raft_node.lock().unwrap_or_else(|e| e.into_inner());
                tick_and_collect(&mut node)
            };

            if outbound.is_empty() {
                continue;
            }

            let map = peer_map.lock().unwrap_or_else(|e| e.into_inner()).clone();
            for (to_id, data) in outbound {
                let Some(addr) = map.get(&to_id) else {
                    continue;
                };
                let msg = crate::network::Message::RaftMessage(data);
                let addr = addr.clone();
                let node = p2p_node.clone();
                // Fire-and-forget: send raft message, ignore response.
                tokio::spawn(async move {
                    let _ = node
                        .send_and_wait(&addr, msg, std::time::Duration::from_secs(2))
                        .await;
                });
            }
        }
    })
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
        msgs.into_iter()
            .map(|m| (m.to, encode_raft_msg(&m)))
            .collect()
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

        // Elect leader — Raft randomises the election timeout between
        // [election_tick, 2*election_tick] (i.e. [10, 20]).  With three nodes
        // and possible split votes, the worst case needs ~40+ ticks.
        route_bytes(&mut nodes, 50);
        let leader_id = nodes.iter().find(|n| n.is_leader()).expect("no leader").id;

        // Propose on leader.
        let leader = nodes.iter_mut().find(|n| n.id == leader_id).unwrap();
        leader.propose(b"transport-test".to_vec()).unwrap();

        // Route until committed on all.
        route_bytes(&mut nodes, 50);

        // All nodes should have the entry.
        for node in &nodes {
            let found = node
                .committed_entries
                .iter()
                .any(|e| e.data == b"transport-test");
            assert!(found, "node {} missing 'transport-test'", node.id);
        }
    }

    #[test]
    fn parse_raft_peers_three_entries() {
        let map = parse_raft_peers("1:orderer1:8087,2:orderer2:8087,3:orderer3:8087");
        assert_eq!(map.len(), 3);
        assert_eq!(map[&1], "orderer1:8087");
        assert_eq!(map[&2], "orderer2:8087");
        assert_eq!(map[&3], "orderer3:8087");
    }

    #[test]
    fn parse_raft_peers_empty_string() {
        let map = parse_raft_peers("");
        assert!(map.is_empty());
    }

    #[test]
    fn parse_raft_peers_with_spaces() {
        let map = parse_raft_peers(" 1:host1:9000 , 2:host2:9000 ");
        assert_eq!(map.len(), 2);
        assert_eq!(map[&1], "host1:9000");
    }

    #[test]
    fn parse_raft_peers_skips_invalid() {
        let map = parse_raft_peers("abc:host:1234,2:valid:5678");
        assert_eq!(map.len(), 1);
        assert_eq!(map[&2], "valid:5678");
    }
}
