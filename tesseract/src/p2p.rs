//! P2P networking: real TCP gossip of attestations between tesseract nodes.
//!
//! Protocol: length-prefixed JSON messages over TCP.
//!
//! Message flow:
//!   1. Node attests event locally (field.attest())
//!   2. Node gossips `AttestEvent` to connected peers (push gossip)
//!   3. Peers apply attestation, decrement TTL, re-propagate if TTL > 0
//!   4. Periodically, nodes run anti-entropy: exchange seen-set digests,
//!      pull missing attestations from peers
//!   5. Ping/Pong for liveness detection; reconnect on failure

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::{Cell, Coord, Dimension};

/// Default TTL for gossip messages.
pub const DEFAULT_TTL: u8 = 3;
/// Ping interval in seconds.
pub const PING_INTERVAL_SECS: u64 = 5;
/// Reconnection backoff base in seconds.
pub const RECONNECT_BASE_SECS: u64 = 2;
/// Anti-entropy interval in seconds.
pub const ANTI_ENTROPY_INTERVAL_SECS: u64 = 10;

/// Messages exchanged between peers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Message {
    /// Gossip an attestation to peers (new model — dimension-bound).
    AttestEvent {
        coord: Coord,
        event_id: String,
        dimension: Dimension,
        validator_id: String,
        /// Time-to-live: decremented on each hop. 0 = don't propagate further.
        ttl: u8,
    },
    /// Legacy seed event (backwards compat with old nodes).
    SeedEvent {
        coord: Coord,
        event_id: String,
        ttl: u8,
    },
    /// Share boundary cells for field synchronization.
    BoundarySync { cells: Vec<(Coord, Cell)> },
    /// Anti-entropy request: "here are my seen keys, send me what I'm missing".
    AntiEntropyRequest { seen_keys: Vec<String> },
    /// Anti-entropy response: attestations the requester is missing.
    AntiEntropyResponse {
        missing: Vec<AttestRecord>,
    },
    /// Ping for liveness.
    Ping { node_id: String },
    /// Pong response.
    Pong { node_id: String },
}

/// Compact attestation record for anti-entropy exchange.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AttestRecord {
    pub coord: Coord,
    pub event_id: String,
    pub dimension: Dimension,
    pub validator_id: String,
}

impl AttestRecord {
    pub fn dedup_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.event_id, self.dimension, self.validator_id, self.coord
        )
    }
}

/// Encode a message as length-prefixed JSON (4-byte big-endian length + JSON bytes).
pub fn encode_message(msg: &Message) -> Vec<u8> {
    let json = serde_json::to_vec(msg).expect("message serialization failed");
    let len = (json.len() as u32).to_be_bytes();
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len);
    buf.extend_from_slice(&json);
    buf
}

/// Decode a message from a TCP stream. Returns None on EOF/error.
pub async fn decode_message(stream: &mut TcpStream) -> Option<Message> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await.ok()?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > 10_000_000 {
        return None; // reject messages > 10MB
    }

    let mut json_buf = vec![0u8; len];
    stream.read_exact(&mut json_buf).await.ok()?;
    serde_json::from_slice(&json_buf).ok()
}

/// Send a message to a TCP stream.
pub async fn send_message(stream: &mut TcpStream, msg: &Message) -> std::io::Result<()> {
    let data = encode_message(msg);
    stream.write_all(&data).await
}

/// Shared peer state.
struct PeerConnection {
    addr: String,
    stream: Option<TcpStream>,
    failures: u32,
}

/// Shared state for the P2P node.
pub struct P2pNode {
    pub node_id: String,
    /// Dedup: attestation keys already seen.
    seen: Arc<RwLock<HashSet<String>>>,
    /// All attestations received (for anti-entropy).
    records: Arc<RwLock<Vec<AttestRecord>>>,
    /// Connected peers.
    peers: Arc<Mutex<Vec<PeerConnection>>>,
    /// Known peer addresses (for reconnection).
    known_addrs: Arc<RwLock<Vec<String>>>,
    /// Channel: inbound attestations to apply to local field.
    pub attest_rx: mpsc::Receiver<AttestRecord>,
    /// Channel: inbound boundary cells to merge.
    pub boundary_rx: mpsc::Receiver<Vec<(Coord, Cell)>>,
}

/// Handle for sending events into the P2P network.
#[derive(Clone)]
pub struct P2pHandle {
    gossip_tx: mpsc::Sender<Message>,
    seen: Arc<RwLock<HashSet<String>>>,
    records: Arc<RwLock<Vec<AttestRecord>>>,
}

impl P2pHandle {
    /// Gossip an attestation to the network.
    pub async fn gossip_attest(
        &self,
        coord: Coord,
        event_id: &str,
        dimension: Dimension,
        validator_id: &str,
    ) {
        let record = AttestRecord {
            coord,
            event_id: event_id.to_string(),
            dimension,
            validator_id: validator_id.to_string(),
        };
        let key = record.dedup_key();

        // Mark as seen locally
        self.seen.write().await.insert(key);
        self.records.write().await.push(record);

        let msg = Message::AttestEvent {
            coord,
            event_id: event_id.to_string(),
            dimension,
            validator_id: validator_id.to_string(),
            ttl: DEFAULT_TTL,
        };
        let _ = self.gossip_tx.send(msg).await;
    }

    /// Send boundary cells to the network.
    pub async fn gossip_boundary(&self, cells: Vec<(Coord, Cell)>) {
        let msg = Message::BoundarySync { cells };
        let _ = self.gossip_tx.send(msg).await;
    }
}

/// Start a P2P node. Returns handle (for sending) and node (for receiving).
pub async fn start(
    node_id: &str,
    listen_addr: &str,
    peer_addrs: &[String],
) -> std::io::Result<(P2pHandle, P2pNode)> {
    let (attest_tx, attest_rx) = mpsc::channel::<AttestRecord>(512);
    let (boundary_tx, boundary_rx) = mpsc::channel::<Vec<(Coord, Cell)>>(64);
    let (gossip_tx, mut gossip_rx) = mpsc::channel::<Message>(512);

    let seen: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));
    let records: Arc<RwLock<Vec<AttestRecord>>> = Arc::new(RwLock::new(Vec::new()));
    let peers: Arc<Mutex<Vec<PeerConnection>>> = Arc::new(Mutex::new(Vec::new()));
    let known_addrs: Arc<RwLock<Vec<String>>> =
        Arc::new(RwLock::new(peer_addrs.to_vec()));

    // ── Listener: accept incoming connections ─────────────────────────────
    let listener = TcpListener::bind(listen_addr).await?;
    let node_id_owned = node_id.to_string();
    {
        let seen = seen.clone();
        let records = records.clone();
        let attest_tx = attest_tx.clone();
        let boundary_tx = boundary_tx.clone();
        let gossip_tx = gossip_tx.clone();
        let node_id = node_id_owned.clone();

        tokio::spawn(async move {
            loop {
                if let Ok((stream, _addr)) = listener.accept().await {
                    let ctx = ConnectionCtx {
                        node_id: node_id.clone(),
                        seen: seen.clone(),
                        records: records.clone(),
                        attest_tx: attest_tx.clone(),
                        boundary_tx: boundary_tx.clone(),
                        gossip_tx: gossip_tx.clone(),
                    };
                    tokio::spawn(handle_connection(stream, ctx));
                }
            }
        });
    }

    // ── Connect to known peers ────────────────��───────────────────────────
    for addr in peer_addrs {
        let stream = TcpStream::connect(addr).await.ok();
        peers.lock().await.push(PeerConnection {
            addr: addr.clone(),
            stream,
            failures: 0,
        });
    }

    // ── Gossip outbound: broadcast messages to all connected peers ─────────
    {
        let peers = peers.clone();
        tokio::spawn(async move {
            while let Some(msg) = gossip_rx.recv().await {
                let data = encode_message(&msg);
                let mut peers = peers.lock().await;
                for peer in peers.iter_mut() {
                    if let Some(ref mut stream) = peer.stream {
                        if stream.write_all(&data).await.is_err() {
                            peer.stream = None;
                            peer.failures += 1;
                        }
                    }
                }
            }
        });
    }

    // ── Liveness: periodic ping + reconnect dead peers ────────────────────
    {
        let peers = peers.clone();
        let node_id = node_id_owned.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(PING_INTERVAL_SECS)).await;

                let mut peers = peers.lock().await;
                for peer in peers.iter_mut() {
                    if let Some(ref mut stream) = peer.stream {
                        // Try ping
                        let ping = Message::Ping {
                            node_id: node_id.clone(),
                        };
                        if send_message(stream, &ping).await.is_err() {
                            peer.stream = None;
                            peer.failures += 1;
                        }
                    } else {
                        // Reconnect with backoff
                        let backoff = RECONNECT_BASE_SECS * 2u64.pow(peer.failures.min(5));
                        if peer.failures > 0 {
                            // Skip this round (backoff timer managed by failure count)
                            // Real impl would track last_attempt timestamp
                        }
                        if let Ok(stream) = TcpStream::connect(&peer.addr).await {
                            peer.stream = Some(stream);
                            peer.failures = 0;
                        }
                    }
                }
            }
        });
    }

    // ── Anti-entropy: periodic pull reconciliation ────────────────────────
    {
        let peers = peers.clone();
        let seen = seen.clone();
        let records = records.clone();
        let attest_tx = attest_tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(ANTI_ENTROPY_INTERVAL_SECS)).await;

                let seen_keys: Vec<String> = seen.read().await.iter().cloned().collect();
                let req = Message::AntiEntropyRequest { seen_keys };
                let data = encode_message(&req);

                // Send to first connected peer
                let mut peers = peers.lock().await;
                for peer in peers.iter_mut() {
                    if let Some(ref mut stream) = peer.stream {
                        if stream.write_all(&data).await.is_ok() {
                            // Try to read response
                            if let Some(Message::AntiEntropyResponse { missing }) =
                                decode_message(stream).await
                            {
                                let mut seen_w = seen.write().await;
                                let mut records_w = records.write().await;
                                for record in missing {
                                    let key = record.dedup_key();
                                    if seen_w.insert(key) {
                                        let _ = attest_tx.send(record.clone()).await;
                                        records_w.push(record);
                                    }
                                }
                            }
                        }
                        break; // one peer per round
                    }
                }
            }
        });
    }

    let handle = P2pHandle {
        gossip_tx,
        seen: seen.clone(),
        records: records.clone(),
    };

    let node = P2pNode {
        node_id: node_id_owned,
        seen,
        records,
        peers,
        known_addrs,
        attest_rx,
        boundary_rx,
    };

    Ok((handle, node))
}

/// Context passed to each connection handler.
struct ConnectionCtx {
    node_id: String,
    seen: Arc<RwLock<HashSet<String>>>,
    records: Arc<RwLock<Vec<AttestRecord>>>,
    attest_tx: mpsc::Sender<AttestRecord>,
    boundary_tx: mpsc::Sender<Vec<(Coord, Cell)>>,
    gossip_tx: mpsc::Sender<Message>,
}

/// Handle an incoming peer connection.
async fn handle_connection(mut stream: TcpStream, ctx: ConnectionCtx) {
    while let Some(msg) = decode_message(&mut stream).await {
        match msg {
            Message::AttestEvent {
                coord,
                event_id,
                dimension,
                validator_id,
                ttl,
            } => {
                let record = AttestRecord {
                    coord,
                    event_id: event_id.clone(),
                    dimension,
                    validator_id: validator_id.clone(),
                };
                let key = record.dedup_key();

                let is_new = ctx.seen.write().await.insert(key);
                if !is_new {
                    continue; // already seen — dedup
                }

                // Apply locally
                ctx.records.write().await.push(record.clone());
                let _ = ctx.attest_tx.send(record).await;

                // Re-propagate with decremented TTL
                if ttl > 0 {
                    let forward = Message::AttestEvent {
                        coord,
                        event_id,
                        dimension,
                        validator_id,
                        ttl: ttl - 1,
                    };
                    let _ = ctx.gossip_tx.send(forward).await;
                }
            }

            Message::SeedEvent {
                coord,
                event_id,
                ttl,
            } => {
                // Legacy compat: treat as all-dimension attestation from "legacy" validator
                let record = AttestRecord {
                    coord,
                    event_id: event_id.clone(),
                    dimension: Dimension::Temporal,
                    validator_id: "legacy".to_string(),
                };
                let key = format!("legacy:{}", event_id);
                let is_new = ctx.seen.write().await.insert(key);
                if !is_new {
                    continue;
                }
                ctx.records.write().await.push(record.clone());
                let _ = ctx.attest_tx.send(record).await;

                if ttl > 0 {
                    let forward = Message::SeedEvent {
                        coord,
                        event_id,
                        ttl: ttl - 1,
                    };
                    let _ = ctx.gossip_tx.send(forward).await;
                }
            }

            Message::BoundarySync { cells } => {
                let _ = ctx.boundary_tx.send(cells).await;
            }

            Message::AntiEntropyRequest { seen_keys } => {
                // Respond with records the requester doesn't have
                let peer_seen: HashSet<String> = seen_keys.into_iter().collect();
                let records = ctx.records.read().await;
                let missing: Vec<AttestRecord> = records
                    .iter()
                    .filter(|r| !peer_seen.contains(&r.dedup_key()))
                    .cloned()
                    .collect();
                let resp = Message::AntiEntropyResponse { missing };
                let _ = send_message(&mut stream, &resp).await;
            }

            Message::AntiEntropyResponse { missing } => {
                // Apply missing attestations from peer
                let mut seen = ctx.seen.write().await;
                let mut records = ctx.records.write().await;
                for record in missing {
                    let key = record.dedup_key();
                    if seen.insert(key) {
                        let _ = ctx.attest_tx.send(record.clone()).await;
                        records.push(record);
                    }
                }
            }

            Message::Ping { .. } => {
                let pong = Message::Pong {
                    node_id: ctx.node_id.clone(),
                };
                let _ = send_message(&mut stream, &pong).await;
            }

            Message::Pong { .. } => {
                // Peer is alive — nothing to do
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_attest_event() {
        let msg = Message::AttestEvent {
            coord: Coord {
                t: 1,
                c: 2,
                o: 3,
                v: 4,
            },
            event_id: "ev1".into(),
            dimension: Dimension::Temporal,
            validator_id: "val-T".into(),
            ttl: 3,
        };
        let encoded = encode_message(&msg);
        assert!(encoded.len() > 4);

        // Verify length prefix
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        assert_eq!(len, encoded.len() - 4);

        // Verify JSON deserialization
        let decoded: Message = serde_json::from_slice(&encoded[4..]).unwrap();
        match decoded {
            Message::AttestEvent {
                coord,
                event_id,
                dimension,
                validator_id,
                ttl,
            } => {
                assert_eq!(coord.t, 1);
                assert_eq!(event_id, "ev1");
                assert_eq!(dimension, Dimension::Temporal);
                assert_eq!(validator_id, "val-T");
                assert_eq!(ttl, 3);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn encode_decode_anti_entropy() {
        let msg = Message::AntiEntropyRequest {
            seen_keys: vec!["a:T:v1:(1,2,3,4)".into(), "b:C:v2:(5,6,7,8)".into()],
        };
        let encoded = encode_message(&msg);
        let decoded: Message = serde_json::from_slice(&encoded[4..]).unwrap();
        match decoded {
            Message::AntiEntropyRequest { seen_keys } => {
                assert_eq!(seen_keys.len(), 2);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn attest_record_dedup_key_deterministic() {
        let r = AttestRecord {
            coord: Coord {
                t: 1,
                c: 2,
                o: 3,
                v: 4,
            },
            event_id: "ev1".into(),
            dimension: Dimension::Origin,
            validator_id: "val-O".into(),
        };
        let k1 = r.dedup_key();
        let k2 = r.dedup_key();
        assert_eq!(k1, k2);
        assert!(k1.contains("ev1"));
        assert!(k1.contains("O"));
        assert!(k1.contains("val-O"));
    }

    #[test]
    fn message_size_within_bounds() {
        // A typical attestation message should be well under 10MB
        let msg = Message::AttestEvent {
            coord: Coord {
                t: 100,
                c: 200,
                o: 300,
                v: 400,
            },
            event_id: "transfer:alice:bob:1000:nonce42".into(),
            dimension: Dimension::Verification,
            validator_id: "validator-verification-node-3".into(),
            ttl: DEFAULT_TTL,
        };
        let encoded = encode_message(&msg);
        assert!(encoded.len() < 1024); // should be ~200 bytes
    }
}
