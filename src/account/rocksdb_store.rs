//! RocksDB-backed persistent account store.
//!
//! Uses a dedicated Column Family `accounts` with key = address, value = JSON AccountState.
//! Atomic writes via WriteBatch for transfer operations.

use rocksdb::{ColumnFamilyDescriptor, DBWithThreadMode, MultiThreaded, Options, WriteBatch};
use std::path::Path;

use super::{AccountError, AccountState, AccountStore};

type RocksDB = DBWithThreadMode<MultiThreaded>;

const CF_ACCOUNTS: &str = "accounts";

/// Persistent account store backed by RocksDB.
pub struct RocksDbAccountStore {
    db: RocksDB,
}

impl RocksDbAccountStore {
    /// Open or create a RocksDB-backed account store at `path`.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, AccountError> {
        let path = path.as_ref();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Merge with existing CFs on disk
        let mut cf_names = vec![CF_ACCOUNTS.to_string()];
        if let Ok(existing) = RocksDB::list_cf(&opts, path) {
            for name in existing {
                if !cf_names.contains(&name) {
                    cf_names.push(name);
                }
            }
        }

        let cf_descriptors: Vec<ColumnFamilyDescriptor> = cf_names
            .into_iter()
            .map(|name| ColumnFamilyDescriptor::new(name, Options::default()))
            .collect();

        let db = RocksDB::open_cf_descriptors(&opts, path, cf_descriptors)
            .map_err(|e| AccountError::Internal(format!("RocksDB open failed: {e}")))?;

        Ok(Self { db })
    }

    fn cf(&self) -> std::sync::Arc<rocksdb::BoundColumnFamily<'_>> {
        self.db
            .cf_handle(CF_ACCOUNTS)
            .expect("accounts CF must exist")
    }
}

impl AccountStore for RocksDbAccountStore {
    fn get_account(&self, address: &str) -> Result<AccountState, AccountError> {
        let cf = self.cf();
        match self.db.get_cf(&cf, address.as_bytes()) {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes)
                .map_err(|e| AccountError::Internal(format!("deserialize: {e}"))),
            Ok(None) => Ok(AccountState::default()),
            Err(e) => Err(AccountError::Internal(format!("RocksDB get: {e}"))),
        }
    }

    fn get_account_if_exists(&self, address: &str) -> Result<Option<AccountState>, AccountError> {
        let cf = self.cf();
        match self.db.get_cf(&cf, address.as_bytes()) {
            Ok(Some(bytes)) => {
                let acc: AccountState = serde_json::from_slice(&bytes)
                    .map_err(|e| AccountError::Internal(format!("deserialize: {e}")))?;
                Ok(Some(acc))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(AccountError::Internal(format!("RocksDB get: {e}"))),
        }
    }

    fn set_account(&self, address: &str, state: &AccountState) -> Result<(), AccountError> {
        let cf = self.cf();
        let json = serde_json::to_vec(state)
            .map_err(|e| AccountError::Internal(format!("serialize: {e}")))?;
        self.db
            .put_cf(&cf, address.as_bytes(), &json)
            .map_err(|e| AccountError::Internal(format!("RocksDB put: {e}")))
    }

    /// Atomic transfer using WriteBatch — both accounts updated in one write.
    fn transfer(
        &self,
        from: &str,
        to: &str,
        amount: u64,
        expected_nonce: u64,
    ) -> Result<(AccountState, AccountState), AccountError> {
        let mut sender = self.get_account(from)?;

        if sender.nonce != expected_nonce {
            return Err(AccountError::NonceMismatch {
                expected: sender.nonce,
                got: expected_nonce,
            });
        }
        if sender.balance < amount {
            return Err(AccountError::InsufficientBalance {
                have: sender.balance,
                need: amount,
            });
        }

        let mut recipient = self.get_account(to)?;

        sender.balance -= amount;
        sender.nonce += 1;
        recipient.balance = recipient
            .balance
            .checked_add(amount)
            .ok_or(AccountError::Overflow)?;

        // Atomic write
        let cf = self.cf();
        let mut batch = WriteBatch::default();
        let sender_json = serde_json::to_vec(&sender)
            .map_err(|e| AccountError::Internal(format!("serialize: {e}")))?;
        let recipient_json = serde_json::to_vec(&recipient)
            .map_err(|e| AccountError::Internal(format!("serialize: {e}")))?;
        batch.put_cf(&cf, from.as_bytes(), &sender_json);
        batch.put_cf(&cf, to.as_bytes(), &recipient_json);
        self.db
            .write(batch)
            .map_err(|e| AccountError::Internal(format!("RocksDB batch write: {e}")))?;

        Ok((sender, recipient))
    }

    fn all_accounts(&self) -> Result<Vec<(String, AccountState)>, AccountError> {
        let cf = self.cf();
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
        let mut result = Vec::new();
        for item in iter {
            let (key, value) =
                item.map_err(|e| AccountError::Internal(format!("RocksDB iter: {e}")))?;
            let address = String::from_utf8(key.to_vec())
                .map_err(|e| AccountError::Internal(format!("key decode: {e}")))?;
            let state: AccountState = serde_json::from_slice(&value)
                .map_err(|e| AccountError::Internal(format!("deserialize: {e}")))?;
            result.push((address, state));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_store() -> (RocksDbAccountStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = RocksDbAccountStore::new(dir.path()).unwrap();
        (store, dir)
    }

    #[test]
    fn get_nonexistent_returns_default() {
        let (store, _dir) = open_store();
        let acc = store.get_account("unknown").unwrap();
        assert_eq!(acc.balance, 0);
        assert_eq!(acc.nonce, 0);
    }

    #[test]
    fn set_and_get_roundtrip() {
        let (store, _dir) = open_store();
        let acc = AccountState::new(42);
        store.set_account("alice", &acc).unwrap();
        let retrieved = store.get_account("alice").unwrap();
        assert_eq!(retrieved.balance, 42);
    }

    #[test]
    fn transfer_atomic() {
        let (store, _dir) = open_store();
        store
            .set_account("alice", &AccountState::new(1000))
            .unwrap();
        let (sender, recipient) = store.transfer("alice", "bob", 300, 0).unwrap();
        assert_eq!(sender.balance, 700);
        assert_eq!(sender.nonce, 1);
        assert_eq!(recipient.balance, 300);
    }

    #[test]
    fn persist_across_reopen() {
        let dir = TempDir::new().unwrap();
        // First open: write data
        {
            let store = RocksDbAccountStore::new(dir.path()).unwrap();
            store
                .set_account("alice", &AccountState::new(5000))
                .unwrap();
            store.transfer("alice", "bob", 1000, 0).unwrap();
        }
        // Second open: verify data survived
        {
            let store = RocksDbAccountStore::new(dir.path()).unwrap();
            let alice = store.get_account("alice").unwrap();
            assert_eq!(alice.balance, 4000);
            assert_eq!(alice.nonce, 1);
            let bob = store.get_account("bob").unwrap();
            assert_eq!(bob.balance, 1000);
        }
    }

    #[test]
    fn multiple_txs_persist_across_restart() {
        let dir = TempDir::new().unwrap();
        {
            let store = RocksDbAccountStore::new(dir.path()).unwrap();
            store
                .set_account("alice", &AccountState::new(10_000))
                .unwrap();
            for i in 0..10 {
                store.transfer("alice", "bob", 100, i).unwrap();
            }
        }
        {
            let store = RocksDbAccountStore::new(dir.path()).unwrap();
            let alice = store.get_account("alice").unwrap();
            assert_eq!(alice.balance, 9000); // 10000 - 10*100
            assert_eq!(alice.nonce, 10);
            let bob = store.get_account("bob").unwrap();
            assert_eq!(bob.balance, 1000);
        }
    }

    #[test]
    fn credit_persists() {
        let dir = TempDir::new().unwrap();
        {
            let store = RocksDbAccountStore::new(dir.path()).unwrap();
            store.credit("miner", 50).unwrap();
            store.credit("miner", 25).unwrap();
        }
        {
            let store = RocksDbAccountStore::new(dir.path()).unwrap();
            let miner = store.get_account("miner").unwrap();
            assert_eq!(miner.balance, 75);
        }
    }

    #[test]
    fn all_accounts_returns_all() {
        let (store, _dir) = open_store();
        store.set_account("a", &AccountState::new(1)).unwrap();
        store.set_account("b", &AccountState::new(2)).unwrap();
        store.set_account("c", &AccountState::new(3)).unwrap();
        let all = store.all_accounts().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn concurrent_reads_safe() {
        let (store, _dir) = open_store();
        store
            .set_account("alice", &AccountState::new(1000))
            .unwrap();

        // Concurrent reads from multiple threads
        let store_ref = &store;
        std::thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    let acc = store_ref.get_account("alice").unwrap();
                    assert_eq!(acc.balance, 1000);
                });
            }
        });
    }
}
