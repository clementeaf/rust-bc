//! Peer connection management for the minimal testnet.

use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::messages::{decode_message, encode_message, NetworkMessage};

/// A connected peer with its TCP stream.
pub struct Peer {
    pub addr: SocketAddr,
    pub stream: TcpStream,
    buf: Vec<u8>,
}

impl Peer {
    pub fn new(addr: SocketAddr, stream: TcpStream) -> Self {
        Self {
            addr,
            stream,
            buf: Vec::with_capacity(1024 * 64),
        }
    }

    /// Send a message to this peer.
    pub async fn send(&mut self, msg: &NetworkMessage) -> std::io::Result<()> {
        let data = encode_message(msg);
        self.stream.write_all(&data).await
    }

    /// Read the next message from this peer. Blocks until a full message arrives
    /// or the connection is closed.
    pub async fn recv(&mut self) -> std::io::Result<Option<NetworkMessage>> {
        loop {
            // Try to decode from buffer
            if let Some((msg, consumed)) = decode_message(&self.buf) {
                self.buf.drain(..consumed);
                return Ok(Some(msg));
            }

            // Read more data
            let mut tmp = vec![0u8; 1024 * 64];
            let n = self.stream.read(&mut tmp).await?;
            if n == 0 {
                return Ok(None); // connection closed
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }
}
