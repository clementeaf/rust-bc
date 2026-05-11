//! Legal oracle service — queries configurable external legal APIs,
//! caches results, and publishes signed records on-chain.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{hash_response, OracleError, OracleRecord, OracleRecordStore};

/// Configuration for a legal data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalSourceConfig {
    /// Human-readable source identifier (e.g., "bcn").
    pub id: String,
    /// Base URL for the API.
    pub base_url: String,
    /// Optional API key header value.
    pub api_key: Option<String>,
}

/// Cache entry with TTL.
struct CacheEntry {
    record: OracleRecord,
    expires_at: u64,
}

/// Legal oracle service.
pub struct LegalOracle {
    sources: HashMap<String, LegalSourceConfig>,
    cache: Mutex<HashMap<String, CacheEntry>>,
    /// Cache TTL in seconds.
    cache_ttl_secs: u64,
}

impl LegalOracle {
    pub fn new(cache_ttl_secs: u64) -> Self {
        Self {
            sources: HashMap::new(),
            cache: Mutex::new(HashMap::new()),
            cache_ttl_secs,
        }
    }

    /// Register a legal data source.
    pub fn register_source(&mut self, config: LegalSourceConfig) {
        self.sources.insert(config.id.clone(), config);
    }

    /// List registered source IDs.
    pub fn source_ids(&self) -> Vec<&str> {
        self.sources.keys().map(|s| s.as_str()).collect()
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn cache_key(source: &str, query: &str) -> String {
        format!("{source}:{query}")
    }

    /// Query a legal source. Returns cached result if available.
    ///
    /// In production, `fetch_fn` would be an HTTP client call. Here we accept
    /// a closure so tests can inject mock responses without network I/O.
    pub fn query<F>(
        &self,
        source_id: &str,
        query_text: &str,
        store: &dyn OracleRecordStore,
        fetch_fn: F,
    ) -> Result<OracleRecord, OracleError>
    where
        F: FnOnce(&LegalSourceConfig, &str) -> Result<Vec<u8>, OracleError>,
    {
        // Check source exists
        let source_config = self
            .sources
            .get(source_id)
            .ok_or_else(|| OracleError::SourceNotConfigured(source_id.to_string()))?;

        // Check cache
        let ck = Self::cache_key(source_id, query_text);
        {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = cache.get(&ck) {
                if entry.expires_at > Self::now_secs() {
                    return Ok(entry.record.clone());
                }
            }
        }

        // Fetch from source
        let response_bytes = fetch_fn(source_config, query_text)?;
        let response_hash = hash_response(&response_bytes);

        let record = OracleRecord {
            id: uuid::Uuid::new_v4().to_string(),
            source: source_id.to_string(),
            query: query_text.to_string(),
            response_hash,
            timestamp: Self::now_secs(),
            signature: String::new(), // Signing deferred to caller with their key
            summary: extract_summary(&response_bytes),
        };

        // Store on-chain
        store
            .store(&record)
            .map_err(|e| OracleError::Storage(e.to_string()))?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.insert(
                ck,
                CacheEntry {
                    record: record.clone(),
                    expires_at: Self::now_secs() + self.cache_ttl_secs,
                },
            );
        }

        Ok(record)
    }

    /// Verify that a record's response_hash matches the given data.
    pub fn verify(record: &OracleRecord, response_data: &[u8]) -> bool {
        hash_response(response_data) == record.response_hash
    }
}

/// Try to extract a human-readable summary from response bytes.
/// If JSON with a "title" or "nombre" field, use that. Otherwise first 200 chars.
fn extract_summary(data: &[u8]) -> Option<String> {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            for key in &["title", "titulo", "nombre", "name", "summary", "resumen"] {
                if let Some(val) = json.get(key).and_then(|v| v.as_str()) {
                    return Some(val.to_string());
                }
            }
        }
        let truncated: String = text.chars().take(200).collect();
        return Some(truncated);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legal_oracle::MemoryOracleRecordStore;

    fn make_oracle() -> LegalOracle {
        let mut oracle = LegalOracle::new(300);
        oracle.register_source(LegalSourceConfig {
            id: "bcn".to_string(),
            base_url: "https://api.bcn.cl".to_string(),
            api_key: None,
        });
        oracle
    }

    fn mock_fetch(_config: &LegalSourceConfig, _query: &str) -> Result<Vec<u8>, OracleError> {
        Ok(br#"{"titulo": "Ley 21.663", "contenido": "Ley marco de ciberseguridad"}"#.to_vec())
    }

    fn mock_fetch_fail(_config: &LegalSourceConfig, _query: &str) -> Result<Vec<u8>, OracleError> {
        Err(OracleError::QueryFailed("connection refused".to_string()))
    }

    #[test]
    fn query_stores_and_returns_record() {
        let oracle = make_oracle();
        let store = MemoryOracleRecordStore::new();

        let record = oracle
            .query("bcn", "ley 21663", &store, mock_fetch)
            .unwrap();

        assert_eq!(record.source, "bcn");
        assert_eq!(record.query, "ley 21663");
        assert!(!record.response_hash.is_empty());
        assert_eq!(record.summary.as_deref(), Some("Ley 21.663"));

        // Verify stored
        let stored = store.get(&record.id).unwrap().unwrap();
        assert_eq!(stored.response_hash, record.response_hash);
    }

    #[test]
    fn query_returns_cached_on_second_call() {
        let oracle = make_oracle();
        let store = MemoryOracleRecordStore::new();

        let r1 = oracle
            .query("bcn", "ley 21663", &store, mock_fetch)
            .unwrap();

        // Second call with a fetch that would fail — should return cache
        let r2 = oracle
            .query("bcn", "ley 21663", &store, mock_fetch_fail)
            .unwrap();

        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn query_unknown_source_fails() {
        let oracle = make_oracle();
        let store = MemoryOracleRecordStore::new();

        let err = oracle
            .query("unknown", "test", &store, mock_fetch)
            .unwrap_err();
        assert!(matches!(err, OracleError::SourceNotConfigured(_)));
    }

    #[test]
    fn query_propagates_fetch_error() {
        let oracle = make_oracle();
        let store = MemoryOracleRecordStore::new();

        let err = oracle
            .query("bcn", "test", &store, mock_fetch_fail)
            .unwrap_err();
        assert!(matches!(err, OracleError::QueryFailed(_)));
    }

    #[test]
    fn verify_matches_correct_data() {
        let oracle = make_oracle();
        let store = MemoryOracleRecordStore::new();

        let record = oracle.query("bcn", "test", &store, mock_fetch).unwrap();
        let data = br#"{"titulo": "Ley 21.663", "contenido": "Ley marco de ciberseguridad"}"#;
        assert!(LegalOracle::verify(&record, data));
    }

    #[test]
    fn verify_rejects_tampered_data() {
        let oracle = make_oracle();
        let store = MemoryOracleRecordStore::new();

        let record = oracle.query("bcn", "test", &store, mock_fetch).unwrap();
        assert!(!LegalOracle::verify(&record, b"tampered data"));
    }

    #[test]
    fn extract_summary_from_json_titulo() {
        let data = br#"{"titulo": "Ley 21.663"}"#;
        assert_eq!(extract_summary(data).unwrap(), "Ley 21.663");
    }

    #[test]
    fn extract_summary_from_plain_text() {
        let data = b"This is a plain text response";
        assert_eq!(
            extract_summary(data).unwrap(),
            "This is a plain text response"
        );
    }

    #[test]
    fn source_ids_lists_registered() {
        let oracle = make_oracle();
        assert_eq!(oracle.source_ids(), vec!["bcn"]);
    }
}
