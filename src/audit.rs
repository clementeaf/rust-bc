//! Audit trail — immutable log of all API requests for regulatory compliance.
//!
//! Each HTTP request is recorded with: timestamp, method, path, caller identity
//! (from TlsIdentity or X-Org-Id), source IP, response status, trace_id, and
//! duration. Records are append-only in RocksDB (CF `audit_log`).

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::storage::errors::{StorageError, StorageResult};

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub org_id: String,
    pub source_ip: String,
    pub status_code: u16,
    pub trace_id: String,
    pub duration_ms: u64,
}

/// Trait for persisting and querying audit entries.
pub trait AuditStore: Send + Sync {
    /// Append an audit entry (append-only, never delete).
    fn append(&self, entry: &AuditEntry) -> StorageResult<()>;

    /// Query entries within a time range, optionally filtered by org_id.
    fn query(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        org_id: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<AuditEntry>>;

    /// Export entries as CSV string.
    fn export_csv(&self, from: Option<&str>, to: Option<&str>) -> StorageResult<String> {
        let entries = self.query(from, to, None, usize::MAX)?;
        let mut csv = String::from(
            "timestamp,method,path,org_id,source_ip,status_code,trace_id,duration_ms\n",
        );
        for e in &entries {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                e.timestamp,
                e.method,
                e.path,
                e.org_id,
                e.source_ip,
                e.status_code,
                e.trace_id,
                e.duration_ms,
            ));
        }
        Ok(csv)
    }
}

/// In-memory audit store (for testing or when RocksDB is not configured).
pub struct MemoryAuditStore {
    entries: std::sync::Mutex<Vec<AuditEntry>>,
}

impl MemoryAuditStore {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for MemoryAuditStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditStore for MemoryAuditStore {
    fn append(&self, entry: &AuditEntry) -> StorageResult<()> {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry.clone());
        Ok(())
    }

    fn query(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        org_id: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<AuditEntry>> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let filtered = entries
            .iter()
            .filter(|e| {
                if let Some(f) = from {
                    if e.timestamp.as_str() < f {
                        return false;
                    }
                }
                if let Some(t) = to {
                    if e.timestamp.as_str() > t {
                        return false;
                    }
                }
                if let Some(org) = org_id {
                    if e.org_id != org {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .cloned()
            .collect();
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(ts: &str, org: &str, path: &str) -> AuditEntry {
        AuditEntry {
            timestamp: ts.to_string(),
            method: "POST".to_string(),
            path: path.to_string(),
            org_id: org.to_string(),
            source_ip: "127.0.0.1".to_string(),
            status_code: 200,
            trace_id: "trace-1".to_string(),
            duration_ms: 5,
        }
    }

    #[test]
    fn append_and_query_all() {
        let store = MemoryAuditStore::new();
        store
            .append(&make_entry(
                "2026-04-07T10:00:00Z",
                "org1",
                "/gateway/submit",
            ))
            .unwrap();
        store
            .append(&make_entry("2026-04-07T11:00:00Z", "org2", "/channels"))
            .unwrap();

        let all = store.query(None, None, None, 100).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn query_by_org_id() {
        let store = MemoryAuditStore::new();
        store
            .append(&make_entry("2026-04-07T10:00:00Z", "org1", "/a"))
            .unwrap();
        store
            .append(&make_entry("2026-04-07T10:01:00Z", "org2", "/b"))
            .unwrap();
        store
            .append(&make_entry("2026-04-07T10:02:00Z", "org1", "/c"))
            .unwrap();

        let org1 = store.query(None, None, Some("org1"), 100).unwrap();
        assert_eq!(org1.len(), 2);
    }

    #[test]
    fn query_by_time_range() {
        let store = MemoryAuditStore::new();
        store
            .append(&make_entry("2026-04-07T10:00:00Z", "org1", "/a"))
            .unwrap();
        store
            .append(&make_entry("2026-04-07T12:00:00Z", "org1", "/b"))
            .unwrap();
        store
            .append(&make_entry("2026-04-07T14:00:00Z", "org1", "/c"))
            .unwrap();

        let range = store
            .query(
                Some("2026-04-07T11:00:00Z"),
                Some("2026-04-07T13:00:00Z"),
                None,
                100,
            )
            .unwrap();
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].path, "/b");
    }

    #[test]
    fn export_csv_format() {
        let store = MemoryAuditStore::new();
        store
            .append(&make_entry(
                "2026-04-07T10:00:00Z",
                "org1",
                "/gateway/submit",
            ))
            .unwrap();

        let csv = store.export_csv(None, None).unwrap();
        assert!(csv.starts_with("timestamp,method,path,org_id"));
        assert!(csv.contains("/gateway/submit"));
    }

    #[test]
    fn query_respects_limit() {
        let store = MemoryAuditStore::new();
        for i in 0..10 {
            store
                .append(&make_entry(
                    &format!("2026-04-07T10:{i:02}:00Z"),
                    "org1",
                    "/a",
                ))
                .unwrap();
        }
        let limited = store.query(None, None, None, 3).unwrap();
        assert_eq!(limited.len(), 3);
    }
}
