//! Audit trail — immutable log of all API requests and domain events for
//! regulatory compliance (ISO 27001).
//!
//! Two levels of audit:
//! - **Request-level**: every HTTP request (method, path, status, duration).
//! - **Action-level**: semantic domain events (block_mined, did_registered,
//!   chaincode_installed, etc.) emitted from business logic.
//!
//! Records are append-only. The `action` field distinguishes domain events
//! from raw HTTP request logs (`action = "http_request"`).

use crate::storage::errors::StorageResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Semantic action types for domain-level audit events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// Raw HTTP request (populated by middleware).
    HttpRequest,
    /// A new block was mined / committed.
    BlockMined,
    /// A wallet was created.
    WalletCreated,
    /// Tokens were transferred.
    TokenTransfer,
    /// Tokens were staked.
    TokenStaked,
    /// Unstake requested.
    TokenUnstaked,
    /// Chaincode (smart contract) was installed.
    ChaincodeInstalled,
    /// Chaincode was upgraded.
    ChaincodeUpgraded,
    /// A DID identity was registered.
    DidRegistered,
    /// A DID was revoked.
    DidRevoked,
    /// A verifiable credential was stored.
    CredentialStored,
    /// A credential was revoked.
    CredentialRevoked,
    /// A channel was created.
    ChannelCreated,
    /// A governance proposal was submitted.
    ProposalSubmitted,
    /// A vote was cast on a proposal.
    ProposalVoted,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{self:?}"));
        f.write_str(&s)
    }
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: AuditAction,
    pub method: String,
    pub path: String,
    pub org_id: String,
    pub source_ip: String,
    pub status_code: u16,
    pub trace_id: String,
    pub duration_ms: u64,
    /// Optional domain-specific metadata (e.g., block height, DID, chaincode ID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// Helper to emit a domain audit event without needing HTTP context.
pub fn emit_domain_event(
    store: &dyn AuditStore,
    action: AuditAction,
    org_id: &str,
    metadata: Option<String>,
) {
    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action,
        method: String::new(),
        path: String::new(),
        org_id: org_id.to_string(),
        source_ip: String::new(),
        status_code: 0,
        trace_id: uuid::Uuid::new_v4().to_string(),
        duration_ms: 0,
        metadata,
    };
    if let Err(e) = store.append(&entry) {
        log::error!("audit domain event failed: {e}");
    }
}

/// Convenience wrapper: emit if store is `Some`.
pub fn emit_if_present(
    store: &Option<Arc<dyn AuditStore>>,
    action: AuditAction,
    org_id: &str,
    metadata: Option<String>,
) {
    if let Some(s) = store {
        emit_domain_event(s.as_ref(), action, org_id, metadata);
    }
}

/// Trait for persisting and querying audit entries.
pub trait AuditStore: Send + Sync {
    /// Append an audit entry (append-only, never delete).
    fn append(&self, entry: &AuditEntry) -> StorageResult<()>;

    /// Query entries within a time range, optionally filtered by org_id and/or action.
    fn query(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        org_id: Option<&str>,
        action: Option<&AuditAction>,
        limit: usize,
    ) -> StorageResult<Vec<AuditEntry>>;

    /// Export entries as CSV string.
    fn export_csv(&self, from: Option<&str>, to: Option<&str>) -> StorageResult<String> {
        let entries = self.query(from, to, None, None, usize::MAX)?;
        let mut csv = String::from(
            "timestamp,action,method,path,org_id,source_ip,status_code,trace_id,duration_ms,metadata\n",
        );
        for e in &entries {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                e.timestamp,
                e.action,
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
        action: Option<&AuditAction>,
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
                if let Some(act) = action {
                    if &e.action != act {
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
            action: AuditAction::HttpRequest,
            method: "POST".to_string(),
            path: path.to_string(),
            org_id: org.to_string(),
            source_ip: "127.0.0.1".to_string(),
            status_code: 200,
            trace_id: "trace-1".to_string(),
            duration_ms: 5,
            metadata: None,
        }
    }

    fn make_domain_entry(ts: &str, org: &str, action: AuditAction, meta: &str) -> AuditEntry {
        AuditEntry {
            timestamp: ts.to_string(),
            action,
            method: String::new(),
            path: String::new(),
            org_id: org.to_string(),
            source_ip: String::new(),
            status_code: 0,
            trace_id: "trace-d".to_string(),
            duration_ms: 0,
            metadata: Some(meta.to_string()),
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

        let all = store.query(None, None, None, None, 100).unwrap();
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

        let org1 = store.query(None, None, Some("org1"), None, 100).unwrap();
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
                None,
                100,
            )
            .unwrap();
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].path, "/b");
    }

    #[test]
    fn query_by_action_type() {
        let store = MemoryAuditStore::new();
        store
            .append(&make_entry("2026-04-07T10:00:00Z", "org1", "/a"))
            .unwrap();
        store
            .append(&make_domain_entry(
                "2026-04-07T10:01:00Z",
                "org1",
                AuditAction::BlockMined,
                "height=42",
            ))
            .unwrap();
        store
            .append(&make_domain_entry(
                "2026-04-07T10:02:00Z",
                "org1",
                AuditAction::DidRegistered,
                "did:cerulean:abc",
            ))
            .unwrap();

        let blocks = store
            .query(None, None, None, Some(&AuditAction::BlockMined), 100)
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].metadata.as_deref(), Some("height=42"));

        let http = store
            .query(None, None, None, Some(&AuditAction::HttpRequest), 100)
            .unwrap();
        assert_eq!(http.len(), 1);
    }

    #[test]
    fn query_combined_filters() {
        let store = MemoryAuditStore::new();
        store
            .append(&make_domain_entry(
                "2026-04-07T10:00:00Z",
                "org1",
                AuditAction::BlockMined,
                "h=1",
            ))
            .unwrap();
        store
            .append(&make_domain_entry(
                "2026-04-07T10:01:00Z",
                "org2",
                AuditAction::BlockMined,
                "h=2",
            ))
            .unwrap();
        store
            .append(&make_domain_entry(
                "2026-04-07T10:02:00Z",
                "org1",
                AuditAction::DidRegistered,
                "did:x",
            ))
            .unwrap();

        let result = store
            .query(
                None,
                None,
                Some("org1"),
                Some(&AuditAction::BlockMined),
                100,
            )
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].metadata.as_deref(), Some("h=1"));
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
        assert!(csv.starts_with("timestamp,action,method,path,org_id"));
        assert!(csv.contains("/gateway/submit"));
        assert!(csv.contains("http_request"));
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
        let limited = store.query(None, None, None, None, 3).unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[test]
    fn emit_domain_event_appends_entry() {
        let store = MemoryAuditStore::new();
        emit_domain_event(
            &store,
            AuditAction::ChaincodeInstalled,
            "org1",
            Some("cc_id=mycc".to_string()),
        );
        let entries = store.query(None, None, None, None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, AuditAction::ChaincodeInstalled);
        assert_eq!(entries[0].metadata.as_deref(), Some("cc_id=mycc"));
    }

    #[test]
    fn emit_if_present_with_none_is_noop() {
        let store: Option<Arc<dyn AuditStore>> = None;
        emit_if_present(&store, AuditAction::BlockMined, "org1", None);
        // No panic — just a no-op.
    }

    #[test]
    fn emit_if_present_with_some_appends() {
        let store: Option<Arc<dyn AuditStore>> = Some(Arc::new(MemoryAuditStore::new()));
        emit_if_present(
            &store,
            AuditAction::DidRevoked,
            "org2",
            Some("did:x".to_string()),
        );
        let entries = store
            .as_ref()
            .unwrap()
            .query(None, None, None, Some(&AuditAction::DidRevoked), 100)
            .unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn audit_action_display() {
        assert_eq!(AuditAction::BlockMined.to_string(), "block_mined");
        assert_eq!(AuditAction::HttpRequest.to_string(), "http_request");
        assert_eq!(
            AuditAction::ChaincodeInstalled.to_string(),
            "chaincode_installed"
        );
    }
}
