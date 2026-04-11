//! Persistent Raft storage backed by RocksDB.
//!
//! Implements the `raft::Storage` trait so that the Raft log, hard state,
//! conf state, and snapshots survive process restarts.
//!
//! Key schema (single CF `raft`):
//!   - `hardstate`  → protobuf-encoded HardState
//!   - `confstate`  → protobuf-encoded ConfState
//!   - `snapshot`   → protobuf-encoded Snapshot
//!   - `entry:{index:020}` → protobuf-encoded Entry (zero-padded for lex order)

use prost::Message as ProstMessage;
use raft::prelude::*;
use raft::{Error as RaftError, GetEntriesContext, RaftState, StorageError};
use rocksdb::{ColumnFamilyDescriptor, DBWithThreadMode, MultiThreaded, Options};
use std::path::Path;
use std::sync::Mutex;

const CF_RAFT: &str = "raft";
const KEY_HARD_STATE: &[u8] = b"hardstate";
const KEY_CONF_STATE: &[u8] = b"confstate";
const KEY_SNAPSHOT: &[u8] = b"snapshot";

fn entry_key(index: u64) -> Vec<u8> {
    format!("entry:{index:020}").into_bytes()
}

type RocksDB = DBWithThreadMode<MultiThreaded>;

/// Persistent Raft storage using RocksDB.
pub struct RocksDbRaftStorage {
    db: RocksDB,
    /// Cached first index (inclusive) — entries before this have been compacted.
    first_index: Mutex<u64>,
}

impl RocksDbRaftStorage {
    /// Open or create a RocksDB instance for Raft state at `path`.
    pub fn new(path: &Path) -> Result<Self, String> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf = ColumnFamilyDescriptor::new(CF_RAFT, Options::default());
        let db = RocksDB::open_cf_descriptors(&opts, path, vec![cf])
            .map_err(|e| format!("failed to open raft DB: {e}"))?;

        let first_index = {
            let cf_handle = db.cf_handle(CF_RAFT).ok_or("missing raft CF")?;
            // Find the first entry key by iterating from the start.
            let mut first = 1u64;
            for (key, _) in db
                .iterator_cf(&cf_handle, rocksdb::IteratorMode::Start)
                .flatten()
            {
                if let Ok(key_str) = std::str::from_utf8(&key) {
                    if let Some(idx_str) = key_str.strip_prefix("entry:") {
                        if let Ok(idx) = idx_str.parse::<u64>() {
                            first = idx;
                            break;
                        }
                    }
                }
            }
            first
        };

        Ok(Self {
            db,
            first_index: Mutex::new(first_index),
        })
    }

    /// Initialize with a conf state (for new clusters).
    pub fn initialize(&self, cs: &ConfState) -> Result<(), String> {
        let cf = self.cf_handle()?;
        // Only initialize if no hard state exists yet.
        if self
            .db
            .get_cf(&cf, KEY_HARD_STATE)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Ok(()); // Already initialized.
        }
        let hs = HardState::default();
        self.put_hard_state(&hs)?;
        self.put_conf_state(cs)?;
        // Write a dummy entry at index 0 so the log is never completely empty.
        Ok(())
    }

    fn cf_handle(&self) -> Result<std::sync::Arc<rocksdb::BoundColumnFamily>, String> {
        self.db
            .cf_handle(CF_RAFT)
            .ok_or_else(|| "missing raft CF".to_string())
    }

    fn put_hard_state(&self, hs: &HardState) -> Result<(), String> {
        let cf = self.cf_handle()?;
        self.db
            .put_cf(&cf, KEY_HARD_STATE, hs.encode_to_vec())
            .map_err(|e| e.to_string())
    }

    fn put_conf_state(&self, cs: &ConfState) -> Result<(), String> {
        let cf = self.cf_handle()?;
        self.db
            .put_cf(&cf, KEY_CONF_STATE, cs.encode_to_vec())
            .map_err(|e| e.to_string())
    }

    fn get_hard_state(&self) -> Result<HardState, String> {
        let cf = self.cf_handle()?;
        match self
            .db
            .get_cf(&cf, KEY_HARD_STATE)
            .map_err(|e| e.to_string())?
        {
            Some(bytes) => HardState::decode(bytes.as_slice()).map_err(|e| e.to_string()),
            None => Ok(HardState::default()),
        }
    }

    fn get_conf_state(&self) -> Result<ConfState, String> {
        let cf = self.cf_handle()?;
        match self
            .db
            .get_cf(&cf, KEY_CONF_STATE)
            .map_err(|e| e.to_string())?
        {
            Some(bytes) => ConfState::decode(bytes.as_slice()).map_err(|e| e.to_string()),
            None => Ok(ConfState::default()),
        }
    }

    /// Append entries to persistent storage.
    pub fn append_entries(&self, entries: &[Entry]) -> Result<(), String> {
        let cf = self.cf_handle()?;
        let mut batch = rocksdb::WriteBatch::default();
        for entry in entries {
            batch.put_cf(&cf, entry_key(entry.index), entry.encode_to_vec());
        }
        self.db.write(batch).map_err(|e| e.to_string())
    }

    /// Set the hard state (term, vote, commit).
    pub fn set_hardstate(&self, hs: &HardState) -> Result<(), String> {
        self.put_hard_state(hs)
    }

    #[allow(dead_code)]
    /// Apply a snapshot — replace all state.
    pub fn apply_snapshot_data(&self, snap: &Snapshot) -> Result<(), String> {
        let cf = self.cf_handle()?;
        // Store the snapshot.
        self.db
            .put_cf(&cf, KEY_SNAPSHOT, snap.encode_to_vec())
            .map_err(|e| e.to_string())?;
        // Update conf state from snapshot metadata.
        self.put_conf_state(snap.get_metadata().get_conf_state())?;
        // Update first_index to snapshot index + 1.
        let snap_idx = snap.get_metadata().index;
        *self.first_index.lock().unwrap_or_else(|e| e.into_inner()) = snap_idx + 1;
        Ok(())
    }

    fn get_entry(&self, index: u64) -> Result<Option<Entry>, String> {
        let cf = self.cf_handle()?;
        match self
            .db
            .get_cf(&cf, entry_key(index))
            .map_err(|e| e.to_string())?
        {
            Some(bytes) => {
                let entry = Entry::decode(bytes.as_slice()).map_err(|e| e.to_string())?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    fn last_index_inner(&self) -> u64 {
        let cf = match self.cf_handle() {
            Ok(cf) => cf,
            Err(_) => return 0,
        };
        // Reverse iterate to find the last entry key.
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::End);
        for (key, _) in iter.flatten() {
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if let Some(idx_str) = key_str.strip_prefix("entry:") {
                    if let Ok(idx) = idx_str.parse::<u64>() {
                        return idx;
                    }
                }
            }
        }
        0
    }
}

impl raft::Storage for RocksDbRaftStorage {
    fn initial_state(&self) -> raft::Result<RaftState> {
        let hs = self
            .get_hard_state()
            .map_err(|e| RaftError::Store(StorageError::Other(e.into())))?;
        let cs = self
            .get_conf_state()
            .map_err(|e| RaftError::Store(StorageError::Other(e.into())))?;
        Ok(RaftState::new(hs, cs))
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        _context: GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        let max = max_size.into().unwrap_or(u64::MAX) as usize;
        let first = *self.first_index.lock().unwrap_or_else(|e| e.into_inner());
        if low < first {
            return Err(RaftError::Store(StorageError::Compacted));
        }

        let mut result = Vec::new();
        let mut size = 0usize;
        for idx in low..high {
            match self.get_entry(idx) {
                Ok(Some(entry)) => {
                    size += entry.encoded_len();
                    result.push(entry);
                    if size >= max {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(RaftError::Store(StorageError::Other(e.into())));
                }
            }
        }
        Ok(result)
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        let first = *self.first_index.lock().unwrap_or_else(|e| e.into_inner());
        if idx < first && idx > 0 {
            return Err(RaftError::Store(StorageError::Compacted));
        }
        if idx == 0 {
            return Ok(0);
        }
        match self.get_entry(idx) {
            Ok(Some(entry)) => Ok(entry.term),
            Ok(None) => {
                // Check snapshot
                let cf = self
                    .cf_handle()
                    .map_err(|e| RaftError::Store(StorageError::Other(e.into())))?;
                if let Some(bytes) = self
                    .db
                    .get_cf(&cf, KEY_SNAPSHOT)
                    .map_err(|e| RaftError::Store(StorageError::Other(e.to_string().into())))?
                {
                    let snap = Snapshot::decode(bytes.as_slice())
                        .map_err(|e| RaftError::Store(StorageError::Other(e.to_string().into())))?;
                    if snap.get_metadata().index == idx {
                        return Ok(snap.get_metadata().term);
                    }
                }
                Err(RaftError::Store(StorageError::Unavailable))
            }
            Err(e) => Err(RaftError::Store(StorageError::Other(e.into()))),
        }
    }

    fn first_index(&self) -> raft::Result<u64> {
        Ok(*self.first_index.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn last_index(&self) -> raft::Result<u64> {
        let last = self.last_index_inner();
        if last == 0 {
            // Empty log — return first_index - 1 (raft convention).
            let first = *self.first_index.lock().unwrap_or_else(|e| e.into_inner());
            return Ok(if first > 0 { first - 1 } else { 0 });
        }
        Ok(last)
    }

    fn snapshot(&self, _request_index: u64, _to: u64) -> raft::Result<Snapshot> {
        let cf = self
            .cf_handle()
            .map_err(|e| RaftError::Store(StorageError::Other(e.into())))?;
        match self
            .db
            .get_cf(&cf, KEY_SNAPSHOT)
            .map_err(|e| RaftError::Store(StorageError::Other(e.to_string().into())))?
        {
            Some(bytes) => Snapshot::decode(bytes.as_slice())
                .map_err(|e| RaftError::Store(StorageError::Other(e.to_string().into()))),
            None => Ok(Snapshot::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_storage_initializes_and_returns_state() {
        let dir = tempfile::TempDir::new().unwrap();
        let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
        let cs = ConfState {
            voters: vec![1, 2, 3],
            ..Default::default()
        };
        storage.initialize(&cs).unwrap();

        let state = storage.initial_state().unwrap();
        assert_eq!(state.conf_state.voters, vec![1, 2, 3]);
    }

    #[test]
    fn append_and_read_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
        let cs = ConfState {
            voters: vec![1],
            ..Default::default()
        };
        storage.initialize(&cs).unwrap();

        let mut entries = Vec::new();
        for i in 1..=5u64 {
            let e = Entry {
                index: i,
                term: 1,
                data: format!("data-{i}").into_bytes(),
                ..Default::default()
            };
            entries.push(e);
        }
        storage.append_entries(&entries).unwrap();

        let read = storage
            .entries(1, 6, None, GetEntriesContext::empty(false))
            .unwrap();
        assert_eq!(read.len(), 5);
        assert_eq!(read[0].data, b"data-1");
        assert_eq!(read[4].data, b"data-5");
    }

    #[test]
    fn last_index_reflects_appended_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
        let cs = ConfState {
            voters: vec![1],
            ..Default::default()
        };
        storage.initialize(&cs).unwrap();

        let e = Entry {
            index: 10,
            term: 2,
            ..Default::default()
        };
        storage.append_entries(&[e]).unwrap();

        assert_eq!(storage.last_index().unwrap(), 10);
    }

    #[test]
    fn hardstate_persists_across_reopen() {
        let dir = tempfile::TempDir::new().unwrap();
        {
            let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
            let cs = ConfState {
                voters: vec![1],
                ..Default::default()
            };
            storage.initialize(&cs).unwrap();
            let hs = HardState {
                term: 5,
                vote: 1,
                commit: 3,
            };
            storage.set_hardstate(&hs).unwrap();
        }
        // Reopen
        let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
        let state = storage.initial_state().unwrap();
        assert_eq!(state.hard_state.term, 5);
        assert_eq!(state.hard_state.vote, 1);
        assert_eq!(state.hard_state.commit, 3);
    }

    #[test]
    fn entries_persist_across_reopen() {
        let dir = tempfile::TempDir::new().unwrap();
        {
            let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
            let cs = ConfState {
                voters: vec![1],
                ..Default::default()
            };
            storage.initialize(&cs).unwrap();
            let e = Entry {
                index: 1,
                term: 1,
                data: b"persistent".to_vec(),
                ..Default::default()
            };
            storage.append_entries(&[e]).unwrap();
        }
        let storage = RocksDbRaftStorage::new(dir.path()).unwrap();
        let entries = storage
            .entries(1, 2, None, GetEntriesContext::empty(false))
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].data, b"persistent");
    }
}
