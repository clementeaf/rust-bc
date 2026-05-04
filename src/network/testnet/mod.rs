//! Minimal testnet network layer — TCP transport, SegWit blocks, no TLS.
//!
//! Separate from the production P2P layer in `network/mod.rs`.
//! Uses `SegWitBlock`, `CompactBlock`, `SegWitMempool`, and `AccountStore`.

pub mod client;
pub mod messages;
pub mod node;
pub mod peer;
pub mod server;
