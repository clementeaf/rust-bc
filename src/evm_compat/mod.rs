//! EVM compatibility layer — full EVM execution via revm, plus ABI
//! encoding/decoding and precompile interface for interoperability
//! with Ethereum tooling and Solidity contracts.

pub mod abi;
pub mod executor;
pub mod precompile;
