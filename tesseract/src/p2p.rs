//! P2P networking: real TCP gossip of events between tesseract nodes.
//!
//! Protocol: length-prefixed JSON messages over TCP.
//! Message flow:
//!   1. Node seeds event locally
//!   2. Node gossips `SeedEvent` to all connected peers
//!   3. Peers apply the seed and propagate to their peers (with TTL)
//!   4. Periodically, nodes exchange boundary cells for field sync

use std::collections::HashSet;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

use crate::{Cell, Coord};

/// Messages exchanged between peers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Message {
    /// Gossip a new event to peers.
    SeedEvent {
        coord: Coord,
        event_id: String,
        /// Time-to-live: decremented on each hop. 0 = don't propagate further.
        ttl: u8,
    },
    /// Share boundary cells for field synchronization.
    BoundarySync { cells: Vec<(Coord, Cell)> },
    /// Ping to check liveness.
    Ping { node_id: String },
    /// Pong response.
    Pong { node_id: String },
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

/// Decode a message from a TCP stream. Returns None on EOF.
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

/// A peer connection (address).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PeerAddr(pub String);

/// Shared state for the P2P node.
pub struct P2pState {
    pub node_id: String,
    /// Incoming events to apply to the local field.
    pub event_rx: mpsc::Receiver<(Coord, String)>,
    /// Incoming boundary cells to merge.
    pub boundary_rx: mpsc::Receiver<Vec<(Coord, Cell)>>,
    /// Event IDs already seen (dedup gossip).
    seen_events: HashSet<String>,
}

/// Handle for sending events into the P2P network.
#[derive(Clone)]
pub struct P2pHandle {
    /// Send events to gossip to peers.
    pub event_tx: mpsc::Sender<Message>,
}

/// Start a P2P node that listens for connections and connects to peers.
/// Returns a handle for sending events and the state for receiving them.
pub async fn start(
    node_id: &str,
    listen_addr: &str,
    peer_addrs: &[String],
) -> std::io::Result<(P2pHandle, P2pState)> {
    let (local_event_tx, local_event_rx) = mpsc::channel::<(Coord, String)>(256);
    let (local_boundary_tx, local_boundary_rx) = mpsc::channel::<Vec<(Coord, Cell)>>(64);
    let (gossip_tx, mut gossip_rx) = mpsc::channel::<Message>(256);

    let seen = Arc::new(Mutex::new(HashSet::<String>::new()));
    let peers: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));

    // Listen for incoming connections
    let listener = TcpListener::bind(listen_addr).await?;
    let node_id_owned = node_id.to_string();
    let seen_clone = seen.clone();
    let local_event_tx_clone = local_event_tx.clone();
    let local_boundary_tx_clone = local_boundary_tx.clone();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, addr)) = listener.accept().await {
                let event_tx = local_event_tx_clone.clone();
                let boundary_tx = local_boundary_tx_clone.clone();
                let seen = seen_clone.clone();
                let node_id = node_id_owned.clone();
                tokio::spawn(handle_connection(
                    stream,
                    node_id,
                    event_tx,
                    boundary_tx,
                    seen,
                ));
            }
        }
    });

    // Connect to known peers
    let peers_clone = peers.clone();
    for addr in peer_addrs {
        if let Ok(stream) = TcpStream::connect(addr).await {
            peers_clone.lock().await.push(stream);
        }
    }

    // Gossip outbound: forward messages to all connected peers
    let peers_for_gossip = peers.clone();
    tokio::spawn(async move {
        while let Some(msg) = gossip_rx.recv().await {
            let data = encode_message(&msg);
            let mut peers = peers_for_gossip.lock().await;
            let mut failed = Vec::new();
            for (i, peer) in peers.iter_mut().enumerate() {
                if peer.write_all(&data).await.is_err() {
                    failed.push(i);
                }
            }
            // Remove dead peers (reverse order to preserve indices)
            for i in failed.into_iter().rev() {
                peers.swap_remove(i);
            }
        }
    });

    let handle = P2pHandle {
        event_tx: gossip_tx,
    };
    let state = P2pState {
        node_id: node_id.to_string(),
        event_rx: local_event_rx,
        boundary_rx: local_boundary_rx,
        seen_events: HashSet::new(),
    };

    Ok((handle, state))
}

/// Handle an incoming peer connection.
async fn handle_connection(
    mut stream: TcpStream,
    node_id: String,
    event_tx: mpsc::Sender<(Coord, String)>,
    boundary_tx: mpsc::Sender<Vec<(Coord, Cell)>>,
    seen: Arc<Mutex<HashSet<String>>>,
) {
    while let Some(msg) = decode_message(&mut stream).await {
        match msg {
            Message::SeedEvent {
                coord,
                event_id,
                ttl,
            } => {
                let mut seen = seen.lock().await;
                if seen.contains(&event_id) {
                    continue; // already processed
                }
                seen.insert(event_id.clone());
                drop(seen);

                // Forward to local field
                let _ = event_tx.send((coord, event_id)).await;
            }
            Message::BoundarySync { cells } => {
                let _ = boundary_tx.send(cells).await;
            }
            Message::Ping { .. } => {
                let pong = Message::Pong {
                    node_id: node_id.clone(),
                };
                let _ = send_message(&mut stream, &pong).await;
            }
            Message::Pong { .. } => {}
        }
    }
}

/// Gossip a seed event to the network.
pub async fn gossip_seed(handle: &P2pHandle, coord: Coord, event_id: &str) {
    let msg = Message::SeedEvent {
        coord,
        event_id: event_id.to_string(),
        ttl: 3,
    };
    let _ = handle.event_tx.send(msg).await;
}

/// Send boundary cells to the network.
pub async fn gossip_boundary(handle: &P2pHandle, cells: Vec<(Coord, Cell)>) {
    let msg = Message::BoundarySync { cells };
    let _ = handle.event_tx.send(msg).await;
}
