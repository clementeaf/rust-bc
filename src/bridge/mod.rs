//! Cross-chain bridge — abstractions for trustless token and message transfers
//! between rust-bc and external blockchain networks.
//!
//! Architecture:
//! - `types.rs` — chain registry, message envelope, transfer records
//! - `protocol.rs` — `BridgeProtocol` trait and bridge engine
//! - `escrow.rs` — lock/mint/burn/release token lifecycle
//! - `verifier.rs` — light client proof verification (Merkle + header)

pub mod escrow;
pub mod protocol;
pub mod types;
pub mod verifier;
