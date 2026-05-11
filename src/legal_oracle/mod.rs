//! Legal oracle — off-chain service that queries external legal APIs and
//! publishes signed results on-chain.
//!
//! The oracle informs, it does not decide. Results are stored as
//! `OracleRecord` with the response hash on-chain and full data off-chain.

pub mod legal;

use pqc_crypto_module::legacy::sha256::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// A single oracle query result stored on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRecord {
    /// Unique record identifier.
    pub id: String,
    /// Source identifier (e.g., "bcn", "registro-civil").
    pub source: String,
    /// The query that was executed.
    pub query: String,
    /// SHA-256 hash of the full response body.
    pub response_hash: String,
    /// Unix timestamp when the query was executed.
    pub timestamp: u64,
    /// Hex-encoded signature over `response_hash` by the oracle operator.
    pub signature: String,
    /// Optional summary extracted from the response (human-readable).
    pub summary: Option<String>,
}

/// Persistence trait for oracle records.
pub trait OracleRecordStore: Send + Sync {
    fn store(&self, record: &OracleRecord) -> Result<(), OracleError>;
    fn get(&self, id: &str) -> Result<Option<OracleRecord>, OracleError>;
    fn list(&self, source: Option<&str>, limit: usize) -> Result<Vec<OracleRecord>, OracleError>;
}

/// In-memory implementation.
pub struct MemoryOracleRecordStore {
    records: Mutex<HashMap<String, OracleRecord>>,
}

impl MemoryOracleRecordStore {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryOracleRecordStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OracleRecordStore for MemoryOracleRecordStore {
    fn store(&self, record: &OracleRecord) -> Result<(), OracleError> {
        self.records
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(record.id.clone(), record.clone());
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<OracleRecord>, OracleError> {
        Ok(self
            .records
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned())
    }

    fn list(&self, source: Option<&str>, limit: usize) -> Result<Vec<OracleRecord>, OracleError> {
        let records = self.records.lock().unwrap_or_else(|e| e.into_inner());
        let filtered: Vec<OracleRecord> = records
            .values()
            .filter(|r| source.is_none_or(|s| r.source == s))
            .take(limit)
            .cloned()
            .collect();
        Ok(filtered)
    }
}

/// Compute SHA-256 hash of response bytes, returned as hex string.
pub fn hash_response(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("source not configured: {0}")]
    SourceNotConfigured(String),
    #[error("query failed: {0}")]
    QueryFailed(String),
    #[error("cache hit")]
    CacheHit,
    #[error("storage error: {0}")]
    Storage(String),
    #[error("verification failed: {0}")]
    VerificationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(id: &str, source: &str) -> OracleRecord {
        OracleRecord {
            id: id.to_string(),
            source: source.to_string(),
            query: "test query".to_string(),
            response_hash: hash_response(b"test response"),
            timestamp: 1700000000,
            signature: "deadbeef".to_string(),
            summary: Some("test summary".to_string()),
        }
    }

    #[test]
    fn store_and_get_roundtrip() {
        let store = MemoryOracleRecordStore::new();
        let record = sample_record("r1", "bcn");
        store.store(&record).unwrap();
        let got = store.get("r1").unwrap().unwrap();
        assert_eq!(got.source, "bcn");
        assert_eq!(got.query, "test query");
    }

    #[test]
    fn get_returns_none_for_missing() {
        let store = MemoryOracleRecordStore::new();
        assert!(store.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn list_filters_by_source() {
        let store = MemoryOracleRecordStore::new();
        store.store(&sample_record("r1", "bcn")).unwrap();
        store.store(&sample_record("r2", "registro")).unwrap();
        store.store(&sample_record("r3", "bcn")).unwrap();

        let bcn = store.list(Some("bcn"), 100).unwrap();
        assert_eq!(bcn.len(), 2);

        let all = store.list(None, 100).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn list_respects_limit() {
        let store = MemoryOracleRecordStore::new();
        for i in 0..10 {
            store
                .store(&sample_record(&format!("r{i}"), "bcn"))
                .unwrap();
        }
        let limited = store.list(None, 3).unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[test]
    fn hash_response_is_deterministic() {
        let h1 = hash_response(b"hello world");
        let h2 = hash_response(b"hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn hash_response_differs_for_different_inputs() {
        let h1 = hash_response(b"hello");
        let h2 = hash_response(b"world");
        assert_ne!(h1, h2);
    }
}
