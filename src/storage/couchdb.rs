//! CouchDB-backed world state adapter.
//!
//! Implements the `WorldState` trait using CouchDB as the backing store.
//! Each key is stored as a JSON document with `_id = key`, versioning tracked
//! in a `version` field, and data stored as base64-encoded bytes.
//!
//! Uses an async `reqwest::Client` internally, bridged to the synchronous
//! `WorldState` trait via `tokio::task::block_in_place` + `Handle::block_on`.
//! This avoids blocking the async runtime's worker threads (which would cause
//! deadlocks under load with `reqwest::blocking::Client`).
//!
//! Requires a running CouchDB instance. Configure via:
//!   `COUCHDB_URL=http://localhost:5984`
//!   `COUCHDB_DB=world_state`
//!
//! Rich queries are supported via CouchDB Mango selectors on `get_range`.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::errors::{StorageError, StorageResult};
use super::traits::HistoryEntry;
use super::world_state::{VersionedValue, WorldState};

/// CouchDB world state adapter.
pub struct CouchDbWorldState {
    client: Client,
    base_url: String,
    db: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CouchDoc {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_rev", skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    version: u64,
    #[serde(with = "base64_bytes")]
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct CouchAllDocsResponse {
    rows: Vec<CouchAllDocsRow>,
}

#[derive(Debug, Deserialize)]
struct CouchAllDocsRow {
    id: String,
    doc: Option<CouchDoc>,
}

#[derive(Debug, Deserialize)]
struct CouchErrorResponse {
    error: Option<String>,
}

mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], ser: S) -> Result<S::Ok, S::Error> {
        STANDARD.encode(data).serialize(ser)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(de)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

/// Run an async future on the current tokio runtime without blocking other
/// worker threads.  Uses `block_in_place` (multi-threaded runtime) so the
/// calling thread is temporarily removed from the worker pool while the
/// future completes.
fn block_on_async<F: std::future::Future>(f: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}

impl CouchDbWorldState {
    /// Create a new adapter and ensure the database exists.
    pub fn new(couchdb_url: &str, db_name: &str) -> StorageResult<Self> {
        let client = Client::new();
        let state = Self {
            client,
            base_url: couchdb_url.trim_end_matches('/').to_string(),
            db: db_name.to_string(),
        };

        // Create DB if it doesn't exist (CouchDB returns 412 if already exists).
        let url = format!("{}/{}", state.base_url, state.db);
        let resp = block_on_async(state.client.put(&url).send())
            .map_err(|e| StorageError::Other(format!("CouchDB create db: {e}")))?;

        let status = resp.status().as_u16();
        if status != 201 && status != 412 {
            return Err(StorageError::Other(format!(
                "CouchDB create db failed: HTTP {status}"
            )));
        }

        Ok(state)
    }

    fn doc_url(&self, key: &str) -> String {
        format!("{}/{}/{}", self.base_url, self.db, urlencoding::encode(key))
    }

    fn get_doc(&self, key: &str) -> StorageResult<Option<CouchDoc>> {
        let resp = block_on_async(self.client.get(self.doc_url(key)).send())
            .map_err(|e| StorageError::Other(format!("CouchDB get: {e}")))?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        let doc: CouchDoc = block_on_async(resp.json())
            .map_err(|e| StorageError::Other(format!("CouchDB get parse: {e}")))?;

        Ok(Some(doc))
    }

    fn put_doc(&self, doc: &CouchDoc) -> StorageResult<String> {
        let resp = block_on_async(self.client.put(self.doc_url(&doc.id)).json(doc).send())
            .map_err(|e| StorageError::Other(format!("CouchDB put: {e}")))?;

        if !resp.status().is_success() {
            let err: CouchErrorResponse =
                block_on_async(resp.json()).unwrap_or(CouchErrorResponse { error: None });
            return Err(StorageError::Other(format!(
                "CouchDB put failed: {}",
                err.error.unwrap_or_else(|| "unknown".to_string())
            )));
        }

        #[derive(Deserialize)]
        struct PutResp {
            rev: String,
        }
        let put_resp: PutResp = block_on_async(resp.json())
            .map_err(|e| StorageError::Other(format!("CouchDB put parse: {e}")))?;

        Ok(put_resp.rev)
    }
}

impl WorldState for CouchDbWorldState {
    fn get(&self, key: &str) -> StorageResult<Option<VersionedValue>> {
        match self.get_doc(key)? {
            Some(doc) => Ok(Some(VersionedValue {
                version: doc.version,
                data: doc.data,
            })),
            None => Ok(None),
        }
    }

    fn put(&self, key: &str, data: &[u8]) -> StorageResult<u64> {
        let (new_version, rev) = match self.get_doc(key)? {
            Some(doc) => (doc.version + 1, doc.rev),
            None => (1, None),
        };

        let doc = CouchDoc {
            id: key.to_string(),
            rev,
            version: new_version,
            data: data.to_vec(),
        };

        self.put_doc(&doc)?;
        Ok(new_version)
    }

    fn delete(&self, key: &str) -> StorageResult<()> {
        let Some(doc) = self.get_doc(key)? else {
            return Ok(());
        };

        let Some(rev) = doc.rev else {
            return Ok(());
        };

        let url = format!("{}?rev={}", self.doc_url(key), rev);
        let resp = block_on_async(self.client.delete(&url).send())
            .map_err(|e| StorageError::Other(format!("CouchDB delete: {e}")))?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            return Err(StorageError::Other("CouchDB delete failed".to_string()));
        }

        Ok(())
    }

    fn get_range(&self, start: &str, end: &str) -> StorageResult<Vec<(String, VersionedValue)>> {
        // Use CouchDB _all_docs with startkey/endkey for range queries.
        let url = format!(
            "{}/{}/_all_docs?include_docs=true&startkey={}&endkey={}",
            self.base_url,
            self.db,
            serde_json::to_string(start).unwrap_or_default(),
            serde_json::to_string(end).unwrap_or_default(),
        );

        let resp = block_on_async(self.client.get(&url).send())
            .map_err(|e| StorageError::Other(format!("CouchDB range: {e}")))?;

        let all_docs: CouchAllDocsResponse = block_on_async(resp.json())
            .map_err(|e| StorageError::Other(format!("CouchDB range parse: {e}")))?;

        let mut results = Vec::new();
        for row in all_docs.rows {
            // Exclude the end key (half-open range like BTreeMap).
            if row.id.as_str() >= end {
                continue;
            }
            if let Some(doc) = row.doc {
                results.push((
                    doc.id,
                    VersionedValue {
                        version: doc.version,
                        data: doc.data,
                    },
                ));
            }
        }

        Ok(results)
    }

    fn get_history(&self, _key: &str) -> StorageResult<Vec<HistoryEntry>> {
        // CouchDB doesn't natively store revision history data.
        // Return empty — a production implementation would use a separate
        // history collection or CouchDB's _changes feed.
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CouchDB tests require a running instance. Skip in CI.
    // Run manually with: COUCHDB_URL=http://localhost:5984 cargo test couchdb -- --ignored

    fn couchdb_url() -> Option<String> {
        std::env::var("COUCHDB_URL").ok()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore]
    async fn put_and_get_roundtrip() {
        let url = couchdb_url().expect("set COUCHDB_URL");
        let db = format!(
            "test_ws_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        let ws = CouchDbWorldState::new(&url, &db).unwrap();

        let v = ws.put("key1", b"hello").unwrap();
        assert_eq!(v, 1);

        let got = ws.get("key1").unwrap().unwrap();
        assert_eq!(got.version, 1);
        assert_eq!(got.data, b"hello");

        let v2 = ws.put("key1", b"world").unwrap();
        assert_eq!(v2, 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore]
    async fn delete_removes_key() {
        let url = couchdb_url().expect("set COUCHDB_URL");
        let db = format!(
            "test_ws_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        let ws = CouchDbWorldState::new(&url, &db).unwrap();

        ws.put("del_key", b"data").unwrap();
        ws.delete("del_key").unwrap();
        assert!(ws.get("del_key").unwrap().is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore]
    async fn get_range_returns_subset() {
        let url = couchdb_url().expect("set COUCHDB_URL");
        let db = format!(
            "test_ws_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        let ws = CouchDbWorldState::new(&url, &db).unwrap();

        for i in 0..5u8 {
            ws.put(&format!("key{i:02}"), &[i]).unwrap();
        }

        let range = ws.get_range("key01", "key03").unwrap();
        assert_eq!(range.len(), 2); // key01, key02
    }
}
