//! World state trait and versioned value type.
//!
//! Provides an abstraction over a key-value store with MVCC versioning:
//! every `put` increments a monotonic version counter for that key so that
//! read-write set conflict detection can compare read versions against the
//! current committed version.

use super::errors::StorageResult;
use super::traits::HistoryEntry;

/// A versioned value stored in the world state.
///
/// Each successful `put` increments `version` by 1.  The first write sets
/// `version = 1`.  A `delete` removes the entry entirely.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VersionedValue {
    pub version: u64,
    pub data: Vec<u8>,
}

/// Versioned key-value world state.
///
/// Implementations must be `Send + Sync` so they can live behind `Arc<dyn WorldState>`.
pub trait WorldState: Send + Sync {
    /// Return the current versioned value for `key`, or `None` if absent.
    fn get(&self, key: &str) -> StorageResult<Option<VersionedValue>>;

    /// Write `data` under `key`, auto-incrementing the version.
    ///
    /// Returns the new version number (1 on first write, n+1 on update).
    fn put(&self, key: &str, data: &[u8]) -> StorageResult<u64>;

    /// Remove `key` from the world state.  No-op if the key does not exist.
    fn delete(&self, key: &str) -> StorageResult<()>;

    /// Return all entries whose key satisfies `start <= key < end`, ordered
    /// lexicographically by key.
    fn get_range(&self, start: &str, end: &str) -> StorageResult<Vec<(String, VersionedValue)>>;

    /// Return the full change history for `key`, ordered by version.
    fn get_history(&self, key: &str) -> StorageResult<Vec<HistoryEntry>>;
}

// ── MemoryStore implementation ────────────────────────────────────────────────

use std::collections::BTreeMap;
use std::sync::Mutex;

/// In-memory world state backed by a `BTreeMap`, which gives free
/// lexicographic ordering for `get_range`.
pub struct MemoryWorldState {
    inner: Mutex<BTreeMap<String, VersionedValue>>,
    history: Mutex<std::collections::HashMap<String, Vec<HistoryEntry>>>,
}

impl MemoryWorldState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(BTreeMap::new()),
            history: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryWorldState {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldState for MemoryWorldState {
    fn get(&self, key: &str) -> StorageResult<Option<VersionedValue>> {
        Ok(self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned())
    }

    fn put(&self, key: &str, data: &[u8]) -> StorageResult<u64> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let new_version = map.get(key).map(|v| v.version + 1).unwrap_or(1);
        map.insert(
            key.to_string(),
            VersionedValue {
                version: new_version,
                data: data.to_vec(),
            },
        );

        // Append history entry.
        let mut hist = self.history.lock().unwrap_or_else(|e| e.into_inner());
        hist.entry(key.to_string()).or_default().push(HistoryEntry {
            version: new_version,
            data: data.to_vec(),
            tx_id: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_delete: false,
        });

        Ok(new_version)
    }

    fn delete(&self, key: &str) -> StorageResult<()> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // Record the delete version before removing.
        let del_version = map.get(key).map(|v| v.version + 1).unwrap_or(1);
        map.remove(key);

        let mut hist = self.history.lock().unwrap_or_else(|e| e.into_inner());
        hist.entry(key.to_string()).or_default().push(HistoryEntry {
            version: del_version,
            data: vec![],
            tx_id: String::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_delete: true,
        });

        Ok(())
    }

    fn get_range(&self, start: &str, end: &str) -> StorageResult<Vec<(String, VersionedValue)>> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let result = map
            .range(start.to_string()..end.to_string())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(result)
    }

    fn get_history(&self, key: &str) -> StorageResult<Vec<HistoryEntry>> {
        let hist = self.history.lock().unwrap_or_else(|e| e.into_inner());
        Ok(hist.get(key).cloned().unwrap_or_default())
    }
}

// ── Composite key helpers ─────────────────────────────────────────────────────

/// Separator byte used in composite keys (matches Hyperledger Fabric convention).
const COMPOSITE_SEP: char = '\x00';

/// Build a composite key from an object type and zero or more attribute values.
///
/// Format: `\x00{object_type}\x00{attr1}\x00{attr2}\x00…`
///
/// The leading `\x00` isolates composite keys from simple string keys in the
/// same namespace.  Every component is separated and terminated by `\x00` so
/// that a prefix scan on `\x00{type}\x00` never matches keys of other types.
pub fn composite_key(object_type: &str, attrs: &[&str]) -> String {
    let mut key = String::new();
    key.push(COMPOSITE_SEP);
    key.push_str(object_type);
    key.push(COMPOSITE_SEP);
    for attr in attrs {
        key.push_str(attr);
        key.push(COMPOSITE_SEP);
    }
    key
}

/// Parse a composite key back into its `(object_type, attributes)` components.
///
/// Returns `None` if the key does not start with `\x00` or has fewer than two
/// `\x00`-separated segments (i.e. it is not a composite key).
pub fn parse_composite_key(key: &str) -> Option<(String, Vec<String>)> {
    // Must start with the separator
    let rest = key.strip_prefix(COMPOSITE_SEP)?;
    // Split on the separator; the last element will be empty due to the
    // trailing separator, so filter it out.
    let mut parts: Vec<&str> = rest.split(COMPOSITE_SEP).collect();
    // Last part is always "" (trailing sep) — remove it.
    if parts.last() == Some(&"") {
        parts.pop();
    }
    if parts.is_empty() {
        return None;
    }
    let object_type = parts[0].to_string();
    let attrs = parts[1..].iter().map(|s| s.to_string()).collect();
    Some((object_type, attrs))
}

/// Return all world-state entries whose composite key starts with the given
/// `object_type` and `partial` attribute prefix.
///
/// Uses `WorldState::get_range` so it works with both `MemoryWorldState` and
/// `RocksDbBlockStore` without scanning the entire namespace.
pub fn get_by_partial_key(
    state: &dyn WorldState,
    object_type: &str,
    partial: &[&str],
) -> StorageResult<Vec<(String, VersionedValue)>> {
    let start = composite_key(object_type, partial);
    // End key: increment the last byte of `start` so the range covers exactly
    // the prefix.  Because `\x00` is the minimum printable byte, the next
    // character after a trailing `\x00` prefix is `\x01`.
    let mut end = start.clone();
    // Replace the trailing `\x00` with `\x01` to form an exclusive upper bound.
    end.pop(); // remove trailing \x00
    end.push('\x01');

    state.get_range(&start, &end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> MemoryWorldState {
        MemoryWorldState::new()
    }

    #[test]
    fn put_new_key_starts_at_version_1() {
        let s = ws();
        assert_eq!(s.put("k", b"v1").unwrap(), 1);
        let vv = s.get("k").unwrap().unwrap();
        assert_eq!(vv.version, 1);
        assert_eq!(vv.data, b"v1");
    }

    #[test]
    fn put_existing_key_increments_version() {
        let s = ws();
        s.put("k", b"v1").unwrap();
        assert_eq!(s.put("k", b"v2").unwrap(), 2);
        let vv = s.get("k").unwrap().unwrap();
        assert_eq!(vv.version, 2);
        assert_eq!(vv.data, b"v2");
    }

    #[test]
    fn get_absent_key_returns_none() {
        let s = ws();
        assert!(s.get("missing").unwrap().is_none());
    }

    #[test]
    fn delete_removes_key() {
        let s = ws();
        s.put("k", b"v").unwrap();
        s.delete("k").unwrap();
        assert!(s.get("k").unwrap().is_none());
    }

    #[test]
    fn delete_absent_key_is_noop() {
        let s = ws();
        assert!(s.delete("ghost").is_ok());
    }

    #[test]
    fn get_range_returns_keys_in_order() {
        let s = ws();
        // Insert 10 keys: "key00".."key09"
        for i in 0..10u8 {
            s.put(&format!("key{:02}", i), &[i]).unwrap();
        }
        // Range ["key02", "key07") → key02,03,04,05,06
        let result = s.get_range("key02", "key07").unwrap();
        assert_eq!(result.len(), 5);
        let keys: Vec<&str> = result.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["key02", "key03", "key04", "key05", "key06"]);
    }

    #[test]
    fn get_range_empty_when_no_match() {
        let s = ws();
        s.put("zzz", b"v").unwrap();
        let result = s.get_range("aaa", "bbb").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn multiple_keys_are_independent() {
        let s = ws();
        s.put("a", b"1").unwrap();
        s.put("a", b"2").unwrap();
        s.put("b", b"x").unwrap();
        assert_eq!(s.get("a").unwrap().unwrap().version, 2);
        assert_eq!(s.get("b").unwrap().unwrap().version, 1);
    }

    // ── composite_key / parse_composite_key ───────────────────────────────────

    #[test]
    fn composite_key_format_has_leading_null() {
        let k = composite_key("Color", &["Blue"]);
        assert!(k.starts_with('\x00'));
    }

    #[test]
    fn composite_key_roundtrip() {
        let k = composite_key("Asset", &["owner1", "id42"]);
        let (typ, attrs) = parse_composite_key(&k).unwrap();
        assert_eq!(typ, "Asset");
        assert_eq!(attrs, ["owner1", "id42"]);
    }

    #[test]
    fn composite_key_no_attrs_roundtrip() {
        let k = composite_key("Asset", &[]);
        let (typ, attrs) = parse_composite_key(&k).unwrap();
        assert_eq!(typ, "Asset");
        assert!(attrs.is_empty());
    }

    #[test]
    fn parse_composite_key_rejects_plain_key() {
        assert!(parse_composite_key("plain-key").is_none());
    }

    #[test]
    fn composite_key_type_prefix_matches_only_that_type() {
        // Keys of different types must not share a prefix
        let car = composite_key("Car", &["red"]);
        let cargo = composite_key("Cargo", &["red"]);
        // "Car\x00red\x00" must not be a prefix of "Cargo\x00red\x00"
        assert!(!cargo.starts_with(&car));
    }

    // ── get_by_partial_key ───────────────────────────────────────────────────

    #[test]
    fn get_by_partial_key_returns_all_for_type() {
        let state = ws();
        for i in 0..5u8 {
            let k = composite_key("Asset", &[&format!("owner{i}"), &format!("id{i}")]);
            state.put(&k, &[i]).unwrap();
        }
        // Unrelated type — must not appear in results
        state.put(&composite_key("Token", &["x"]), b"t").unwrap();

        let results = get_by_partial_key(&state, "Asset", &[]).unwrap();
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|(k, _)| k.contains("Asset")));
    }

    #[test]
    fn get_by_partial_key_filters_by_first_attr() {
        let state = ws();
        // owner0 has 3 assets, owner1 has 2
        for i in 0..3u8 {
            state
                .put(
                    &composite_key("Asset", &["owner0", &format!("id{i}")]),
                    &[i],
                )
                .unwrap();
        }
        for i in 0..2u8 {
            state
                .put(
                    &composite_key("Asset", &["owner1", &format!("id{i}")]),
                    &[i],
                )
                .unwrap();
        }

        let owner0 = get_by_partial_key(&state, "Asset", &["owner0"]).unwrap();
        assert_eq!(owner0.len(), 3);

        let owner1 = get_by_partial_key(&state, "Asset", &["owner1"]).unwrap();
        assert_eq!(owner1.len(), 2);
    }

    #[test]
    fn get_by_partial_key_empty_when_no_match() {
        let state = ws();
        state
            .put(&composite_key("Asset", &["ownerA"]), b"v")
            .unwrap();
        let results = get_by_partial_key(&state, "Token", &[]).unwrap();
        assert!(results.is_empty());
    }

    // ── get_history ─────────────────────────────────────────────────────────

    #[test]
    fn put_five_times_yields_five_history_entries() {
        let s = ws();
        for i in 1..=5u8 {
            s.put("k", &[i]).unwrap();
        }
        let hist = s.get_history("k").unwrap();
        assert_eq!(hist.len(), 5);
        for (i, entry) in hist.iter().enumerate() {
            assert_eq!(entry.version, (i + 1) as u64);
            assert!(!entry.is_delete);
        }
    }

    #[test]
    fn delete_appends_history_entry_with_is_delete() {
        let s = ws();
        s.put("k", b"a").unwrap();
        s.delete("k").unwrap();
        let hist = s.get_history("k").unwrap();
        assert_eq!(hist.len(), 2);
        assert!(!hist[0].is_delete);
        assert!(hist[1].is_delete);
        assert_eq!(hist[1].version, 2);
        assert!(hist[1].data.is_empty());
    }

    #[test]
    fn put_put_delete_history_sequence() {
        let s = ws();
        s.put("x", b"a").unwrap();
        s.put("x", b"b").unwrap();
        s.delete("x").unwrap();
        let hist = s.get_history("x").unwrap();
        assert_eq!(hist.len(), 3);
        assert_eq!(hist[0].data, b"a");
        assert_eq!(hist[0].version, 1);
        assert_eq!(hist[1].data, b"b");
        assert_eq!(hist[1].version, 2);
        assert!(hist[2].is_delete);
        assert_eq!(hist[2].version, 3);
    }

    #[test]
    fn get_history_absent_key_returns_empty() {
        let s = ws();
        assert!(s.get_history("missing").unwrap().is_empty());
    }
}
