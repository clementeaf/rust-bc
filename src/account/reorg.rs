//! Reorg safety — snapshot, rollback, and reapply for account state.
//!
//! Before applying a block, take a snapshot of affected accounts.
//! On reorg, revert to snapshot and replay the winning chain.

use std::collections::HashMap;
use std::sync::Mutex;

use super::{AccountError, AccountState, AccountStore};
use crate::tokenomics::economics::EconomicsState;
use crate::transaction::native::NativeTransaction;

/// A snapshot of account state at a given height, enabling rollback.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Height this snapshot was taken before.
    pub before_height: u64,
    /// Account states before the block was applied.
    pub accounts: HashMap<String, AccountState>,
    /// Economics state before the block.
    pub economics: EconomicsState,
}

/// Manages snapshots for rollback capability.
pub struct ReorgManager {
    /// Snapshots keyed by height. Keeps last N heights.
    snapshots: Mutex<HashMap<u64, StateSnapshot>>,
    /// Maximum number of snapshots to retain.
    max_snapshots: usize,
}

impl ReorgManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Mutex::new(HashMap::new()),
            max_snapshots,
        }
    }

    /// Take a snapshot of accounts that will be affected by a block's transactions.
    ///
    /// Call this BEFORE applying the block.
    pub fn snapshot_before_block(
        &self,
        height: u64,
        affected_addresses: &[&str],
        store: &dyn AccountStore,
        economics: &EconomicsState,
    ) -> Result<(), AccountError> {
        let mut accounts = HashMap::new();
        for addr in affected_addresses {
            accounts.insert(addr.to_string(), store.get_account(addr)?);
        }

        let snap = StateSnapshot {
            before_height: height,
            accounts,
            economics: economics.clone(),
        };

        let mut snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        snapshots.insert(height, snap);

        // Prune old snapshots
        if snapshots.len() > self.max_snapshots {
            if let Some(&min_key) = snapshots.keys().min() {
                snapshots.remove(&min_key);
            }
        }

        Ok(())
    }

    /// Rollback to the state before `height` was applied.
    ///
    /// Restores all snapshotted accounts and returns the pre-block economics state.
    pub fn rollback_to(
        &self,
        height: u64,
        store: &dyn AccountStore,
    ) -> Result<EconomicsState, AccountError> {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());

        let snap = snapshots
            .remove(&height)
            .ok_or_else(|| AccountError::Internal(format!("no snapshot for height {height}")))?;

        // Restore all accounts to pre-block state
        for (addr, state) in &snap.accounts {
            store.set_account(addr, state)?;
        }

        // Remove all snapshots at or above this height (they're invalid now)
        let to_remove: Vec<u64> = snapshots
            .keys()
            .filter(|&&h| h >= height)
            .copied()
            .collect();
        for h in to_remove {
            snapshots.remove(&h);
        }

        Ok(snap.economics)
    }

    /// Check if a rollback to `height` is possible.
    pub fn can_rollback_to(&self, height: u64) -> bool {
        let snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        snapshots.contains_key(&height)
    }

    /// Number of snapshots currently retained.
    pub fn snapshot_count(&self) -> usize {
        let snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        snapshots.len()
    }
}

/// Extract all addresses affected by a set of transactions (senders + recipients + proposer).
pub fn affected_addresses<'a>(txs: &'a [NativeTransaction], proposer: &'a str) -> Vec<&'a str> {
    let mut addrs: Vec<&str> = vec![proposer];
    for tx in txs {
        if let Some(sender) = tx.sender() {
            addrs.push(sender);
        }
        addrs.push(tx.recipient());
    }
    addrs.sort_unstable();
    addrs.dedup();
    addrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::MemoryAccountStore;
    use crate::tokenomics::economics::EconomicsState;
    use crate::transaction::native::execute_transfer;

    #[test]
    fn snapshot_and_rollback_restores_state() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000), ("bob", 500)]);
        let economics = EconomicsState::default();
        let mgr = ReorgManager::new(10);

        // Snapshot before block 1
        mgr.snapshot_before_block(1, &["alice", "bob"], &store, &economics)
            .unwrap();

        // Apply a transfer
        let tx = NativeTransaction::new_transfer("alice", "bob", 200, 0, 5);
        execute_transfer(&store, &tx, "miner").unwrap();

        let alice_after = store.get_account("alice").unwrap();
        assert_eq!(alice_after.balance, 795); // 1000 - 200 - 5

        // Rollback
        let restored_econ = mgr.rollback_to(1, &store).unwrap();
        assert_eq!(restored_econ.height, 0);

        let alice_restored = store.get_account("alice").unwrap();
        assert_eq!(alice_restored.balance, 1000);
        let bob_restored = store.get_account("bob").unwrap();
        assert_eq!(bob_restored.balance, 500);
    }

    #[test]
    fn rollback_nonexistent_height_fails() {
        let store = MemoryAccountStore::new();
        let mgr = ReorgManager::new(10);
        assert!(mgr.rollback_to(99, &store).is_err());
    }

    #[test]
    fn fork_at_height_longer_chain_wins() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
        let economics = EconomicsState::default();
        let mgr = ReorgManager::new(10);

        // Snapshot before block 1
        let addrs = vec!["alice", "bob", "charlie", "miner"];
        mgr.snapshot_before_block(1, &addrs, &store, &economics)
            .unwrap();

        // Chain A: alice → bob 500
        let tx_a = NativeTransaction::new_transfer("alice", "bob", 500, 0, 5);
        execute_transfer(&store, &tx_a, "miner").unwrap();

        let bob_chain_a = store.get_account("bob").unwrap();
        assert_eq!(bob_chain_a.balance, 500);

        // Fork detected! Chain B is longer. Rollback block 1.
        mgr.rollback_to(1, &store).unwrap();

        // Verify rollback
        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.balance, 10_000);
        assert_eq!(alice.nonce, 0);
        let bob = store.get_account("bob").unwrap();
        assert_eq!(bob.balance, 0);

        // Apply chain B: alice → charlie 300
        let tx_b = NativeTransaction::new_transfer("alice", "charlie", 300, 0, 5);
        execute_transfer(&store, &tx_b, "miner").unwrap();

        let charlie = store.get_account("charlie").unwrap();
        assert_eq!(charlie.balance, 300);
        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.balance, 10_000 - 300 - 5);
    }

    #[test]
    fn no_double_spend_after_reorg() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let economics = EconomicsState::default();
        let mgr = ReorgManager::new(10);

        // Snapshot, apply tx (alice sends 900)
        mgr.snapshot_before_block(1, &["alice", "bob", "miner"], &store, &economics)
            .unwrap();
        let tx = NativeTransaction::new_transfer("alice", "bob", 900, 0, 5);
        execute_transfer(&store, &tx, "miner").unwrap();

        // Rollback
        mgr.rollback_to(1, &store).unwrap();

        // Alice is back to 1000, nonce back to 0
        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.balance, 1000);
        assert_eq!(alice.nonce, 0);

        // Alice can send again with nonce 0 — no double spend from the reverted chain
        let tx2 = NativeTransaction::new_transfer("alice", "charlie", 800, 0, 5);
        execute_transfer(&store, &tx2, "miner").unwrap();

        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.balance, 195); // 1000 - 800 - 5
                                        // Bob got nothing (reverted)
        let bob = store.get_account("bob").unwrap();
        assert_eq!(bob.balance, 0);
    }

    #[test]
    fn snapshot_pruning() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let economics = EconomicsState::default();
        let mgr = ReorgManager::new(3); // keep only 3

        for h in 1..=5 {
            mgr.snapshot_before_block(h, &["alice"], &store, &economics)
                .unwrap();
        }

        assert_eq!(mgr.snapshot_count(), 3);
        // Oldest (1, 2) pruned; 3, 4, 5 remain
        assert!(!mgr.can_rollback_to(1));
        assert!(!mgr.can_rollback_to(2));
        assert!(mgr.can_rollback_to(3));
        assert!(mgr.can_rollback_to(5));
    }

    #[test]
    fn affected_addresses_extracts_all() {
        let txs = vec![
            NativeTransaction::new_transfer("alice", "bob", 10, 0, 1),
            NativeTransaction::new_transfer("charlie", "bob", 20, 0, 1),
        ];
        let addrs = affected_addresses(&txs, "miner");
        assert!(addrs.contains(&"alice"));
        assert!(addrs.contains(&"bob"));
        assert!(addrs.contains(&"charlie"));
        assert!(addrs.contains(&"miner"));
    }
}
