//! Storage Tier (Tier 1): RocksDB-backed persistence layer
//!
//! Responsibilities:
//! - Block storage and retrieval
//! - Merkle proof generation
//! - Index maintenance (UTXO, timestamp, account)
//! - Ledger state management
//! - Error handling with exponential backoff

use std::sync::Arc;
use thiserror::Error;

pub mod adapters;
pub mod index;
pub mod ledger;
pub mod errors;
pub mod traits;

pub use adapters::RocksDbBlockStore;
pub use errors::{StorageError, StorageResult};
pub use traits::BlockStore;

/// Storage configuration
#[derive(Clone, Debug)]
pub struct StorageConfig {
    pub path: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: "./data/blockchain".to_string(),
            max_retries: 3,
            retry_delay_ms: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = StorageConfig::default();
        assert_eq!(cfg.max_retries, 3);
    }
}
