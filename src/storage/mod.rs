//! Storage Tier (Tier 1): RocksDB-backed persistence layer
//!
//! Responsibilities:
//! - Block storage and retrieval
//! - Merkle proof generation
//! - Index maintenance (UTXO, timestamp, account)
//! - Ledger state management
//! - Error handling with exponential backoff

pub mod adapters;
pub mod comprehensive_tests;
pub mod couchdb;
pub mod errors;
pub mod memory;
pub mod snapshot;
pub mod traits;
pub mod world_state;

pub use adapters::RocksDbBlockStore;
pub use errors::{StorageError, StorageResult};
pub use memory::MemoryStore;
pub use traits::BlockStore;
pub use world_state::{
    composite_key, get_by_partial_key, parse_composite_key, MemoryWorldState, VersionedValue,
    WorldState,
};

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
