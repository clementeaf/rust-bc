//! TCP server for the minimal testnet node.

use std::net::SocketAddr;
use std::sync::Arc;

use log::info;
use tokio::net::TcpListener;

use super::node::NodeHandle;
use super::peer::Peer;

/// Start listening for incoming peer connections.
/// Spawns a handler task for each accepted connection.
pub async fn start_server(addr: SocketAddr, node: Arc<NodeHandle>) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("[testnet] server listening on {addr}");

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        info!("[testnet] accepted connection from {peer_addr}");
        let node = Arc::clone(&node);

        tokio::spawn(async move {
            let mut peer = Peer::new(peer_addr, stream);
            loop {
                match peer.recv().await {
                    Ok(Some(msg)) => {
                        node.handle_message(msg, &mut peer).await;
                    }
                    Ok(None) => {
                        // Connection closed
                        info!("[testnet] peer {peer_addr} closed connection");
                        break;
                    }
                    Err(e) => {
                        info!("[testnet] peer {peer_addr} error: {e}");
                        break;
                    }
                }
            }
        });
    }
}
