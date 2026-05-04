//! TCP client for connecting to testnet peers.

use std::net::SocketAddr;

use log::info;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::messages::NetworkMessage;
use super::peer::Peer;

/// Connect to a peer and return a Peer handle.
pub async fn connect_to_peer(addr: SocketAddr) -> std::io::Result<Peer> {
    let stream = TcpStream::connect(addr).await?;
    info!("[testnet] connected to peer {addr}");
    Ok(Peer::new(addr, stream))
}

/// Connect, send a single message (flushed), then return the peer.
pub async fn send_to_peer(addr: SocketAddr, msg: &NetworkMessage) -> std::io::Result<Peer> {
    let mut peer = connect_to_peer(addr).await?;
    peer.send(msg).await?;
    peer.stream.flush().await?;
    Ok(peer)
}

/// Broadcast a message to multiple peers. Errors are logged but not fatal.
/// Each peer connection is shut down gracefully after sending.
pub async fn broadcast(peers: &[SocketAddr], msg: &NetworkMessage) {
    for addr in peers {
        match send_to_peer(*addr, msg).await {
            Ok(mut peer) => {
                // Shut down the write half gracefully so the server gets the data
                let _ = peer.stream.shutdown().await;
                info!("[testnet] broadcast to {addr}");
            }
            Err(e) => info!("[testnet] broadcast to {addr} failed: {e}"),
        }
    }
}
