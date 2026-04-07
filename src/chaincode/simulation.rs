//! Simulation world state — executes chaincode without committing to the real ledger.
//!
//! `SimulationWorldState` wraps a read-only `Arc<dyn WorldState>` and buffers
//! all writes locally.  Reads are satisfied from the write buffer first, then
//! from the base state.  Every read is recorded in a `read_set` so that an
//! MVCC conflict check can be performed later.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::storage::errors::StorageResult;
use crate::storage::world_state::{VersionedValue, WorldState};
use crate::transaction::rwset::{KVRead, KVWrite};

/// A sandboxed world state for chaincode simulation.
///
/// Writes go into `write_buffer` only — the underlying `base_state` is never
/// mutated.  `delete_set` tracks keys that the simulation has deleted.
pub struct SimulationWorldState {
    /// The committed ledger state — read-only from simulation's perspective.
    base_state: Arc<dyn WorldState>,
    /// Local write buffer: key → raw bytes.
    write_buffer: Mutex<HashMap<String, Vec<u8>>>,
    /// Keys read during simulation, with the version observed at read time.
    read_set: Mutex<Vec<KVRead>>,
    /// Keys logically deleted during simulation.
    delete_set: Mutex<Vec<String>>,
}

impl SimulationWorldState {
    pub fn new(base_state: Arc<dyn WorldState>) -> Self {
        Self {
            base_state,
            write_buffer: Mutex::new(HashMap::new()),
            read_set: Mutex::new(Vec::new()),
            delete_set: Mutex::new(Vec::new()),
        }
    }

    /// Drain the accumulated read/write sets into a `ReadWriteSet`.
    ///
    /// Writes in `write_buffer` become `KVWrite` entries; reads in `read_set`
    /// are included as-is.
    pub fn to_rwset(&self) -> crate::transaction::rwset::ReadWriteSet {
        use crate::transaction::rwset::ReadWriteSet;

        let reads = self
            .read_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let writes: Vec<KVWrite> = self
            .write_buffer
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| KVWrite {
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        ReadWriteSet { reads, writes }
    }
}

impl WorldState for SimulationWorldState {
    /// Return the value for `key` and record the read.
    ///
    /// Resolution order:
    /// 1. If `key` is in `delete_set` → return `None` (version 0 recorded).
    /// 2. If `key` is in `write_buffer` → return that value (version 0, local write).
    /// 3. Delegate to `base_state`; record the observed version.
    fn get(&self, key: &str) -> StorageResult<Option<VersionedValue>> {
        // Check delete set first.
        if self
            .delete_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|k| k == key)
        {
            self.read_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(KVRead {
                    key: key.to_string(),
                    version: 0,
                });
            return Ok(None);
        }

        // Check local write buffer.
        if let Some(data) = self
            .write_buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
        {
            // Local writes have no committed version — record version 0.
            self.read_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(KVRead {
                    key: key.to_string(),
                    version: 0,
                });
            return Ok(Some(VersionedValue { version: 0, data }));
        }

        // Fall through to base state.
        let result = self.base_state.get(key)?;
        let version = result.as_ref().map(|vv| vv.version).unwrap_or(0);
        self.read_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(KVRead {
                key: key.to_string(),
                version,
            });
        Ok(result)
    }

    /// Buffer the write locally; never touch `base_state`.
    ///
    /// Returns version `0` because no real version has been assigned yet.
    fn put(&self, key: &str, data: &[u8]) -> StorageResult<u64> {
        // If the key was previously deleted in this simulation, un-delete it.
        self.delete_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|k| k != key);
        self.write_buffer
            .lock()
            .unwrap()
            .insert(key.to_string(), data.to_vec());
        Ok(0)
    }

    /// Mark the key as deleted in the simulation without touching `base_state`.
    fn delete(&self, key: &str) -> StorageResult<()> {
        self.write_buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
        let mut ds = self.delete_set.lock().unwrap_or_else(|e| e.into_inner());
        if !ds.iter().any(|k| k == key) {
            ds.push(key.to_string());
        }
        Ok(())
    }

    /// Range scan: merge base state results with local writes/deletes.
    fn get_range(&self, start: &str, end: &str) -> StorageResult<Vec<(String, VersionedValue)>> {
        let mut base = self.base_state.get_range(start, end)?;

        let write_buf = self.write_buffer.lock().unwrap_or_else(|e| e.into_inner());
        let delete_set = self.delete_set.lock().unwrap_or_else(|e| e.into_inner());

        // Remove base entries that were locally deleted or overwritten.
        base.retain(|(k, _)| !delete_set.contains(k) && !write_buf.contains_key(k));

        // Add local writes that fall in [start, end).
        for (k, v) in write_buf.iter() {
            if k.as_str() >= start && k.as_str() < end {
                base.push((
                    k.clone(),
                    VersionedValue {
                        version: 0,
                        data: v.clone(),
                    },
                ));
            }
        }

        // Keep lexicographic order.
        base.sort_by(|(a, _), (b, _)| a.cmp(b));
        Ok(base)
    }

    fn get_history(&self, key: &str) -> StorageResult<Vec<crate::storage::traits::HistoryEntry>> {
        // Simulation doesn't track its own history — delegate to base.
        self.base_state.get_history(key)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::world_state::MemoryWorldState;

    fn base_with(entries: &[(&str, &[u8])]) -> Arc<dyn WorldState> {
        let ws = MemoryWorldState::new();
        for (k, v) in entries {
            ws.put(k, v).unwrap();
        }
        Arc::new(ws)
    }

    // ── put does not modify base_state ─────────────────────────────────────────

    #[test]
    fn simulate_put_does_not_change_base_state() {
        let base = base_with(&[]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.put("x", b"hello").unwrap();

        // base_state must remain untouched
        assert!(base.get("x").unwrap().is_none());
    }

    // ── get from write_buffer returns local value ──────────────────────────────

    #[test]
    fn simulate_put_then_get_returns_local_value() {
        let base = base_with(&[]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.put("x", b"local").unwrap();
        let vv = sim.get("x").unwrap().unwrap();
        assert_eq!(vv.data, b"local");
    }

    // ── get falls through to base_state ───────────────────────────────────────

    #[test]
    fn simulate_get_reads_from_base_when_not_in_buffer() {
        let base = base_with(&[("y", b"base_value")]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        let vv = sim.get("y").unwrap().unwrap();
        assert_eq!(vv.data, b"base_value");
    }

    // ── get records KVRead ─────────────────────────────────────────────────────

    #[test]
    fn simulate_get_records_read_in_read_set() {
        let base = base_with(&[("z", b"v")]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.get("z").unwrap();

        let rs = sim.read_set.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].key, "z");
        assert_eq!(rs[0].version, 1); // first write in base → version 1
    }

    #[test]
    fn simulate_get_missing_key_records_version_zero() {
        let base = base_with(&[]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.get("ghost").unwrap();

        let rs = sim.read_set.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].version, 0);
    }

    // ── delete marks key without touching base_state ──────────────────────────

    #[test]
    fn simulate_delete_hides_base_key() {
        let base = base_with(&[("d", b"data")]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.delete("d").unwrap();

        assert!(sim.get("d").unwrap().is_none());
        // base is intact
        assert!(base.get("d").unwrap().is_some());
    }

    // ── to_rwset aggregates reads and writes ──────────────────────────────────

    #[test]
    fn to_rwset_three_reads_two_writes() {
        let base = base_with(&[("r1", b"v1"), ("r2", b"v2"), ("r3", b"v3")]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.get("r1").unwrap();
        sim.get("r2").unwrap();
        sim.get("r3").unwrap();
        sim.put("w1", b"new1").unwrap();
        sim.put("w2", b"new2").unwrap();

        let rwset = sim.to_rwset();
        assert_eq!(rwset.reads.len(), 3);
        assert_eq!(rwset.writes.len(), 2);

        let read_keys: Vec<&str> = rwset.reads.iter().map(|r| r.key.as_str()).collect();
        assert!(read_keys.contains(&"r1"));
        assert!(read_keys.contains(&"r2"));
        assert!(read_keys.contains(&"r3"));
    }

    #[test]
    fn to_rwset_contains_recorded_reads_and_writes() {
        let base = base_with(&[("r1", b"v1"), ("r2", b"v2")]);
        let sim = SimulationWorldState::new(Arc::clone(&base));

        sim.get("r1").unwrap();
        sim.get("r2").unwrap();
        sim.put("w1", b"new1").unwrap();
        sim.put("w2", b"new2").unwrap();

        let rwset = sim.to_rwset();
        assert_eq!(rwset.reads.len(), 2);
        assert_eq!(rwset.writes.len(), 2);

        let write_keys: Vec<&str> = rwset.writes.iter().map(|w| w.key.as_str()).collect();
        assert!(write_keys.contains(&"w1"));
        assert!(write_keys.contains(&"w2"));
    }
}
