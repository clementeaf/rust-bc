//! EVM compatibility layer — ABI encoding/decoding and precompile interface
//! for interoperability with Ethereum tooling and Solidity contracts.
//!
//! This is NOT a full EVM implementation. Instead, it provides:
//! - Solidity ABI encoding/decoding for function calls and return values
//! - Precompile address mapping for common operations
//! - Ethereum-compatible address derivation from rust-bc DIDs

pub mod abi;
pub mod precompile;
