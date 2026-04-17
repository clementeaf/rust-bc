//! Storage deposits — lock tokens when creating on-chain objects, refund on deletion.
//!
//! Every key-value entry in the world state requires a storage deposit proportional
//! to the data size. This prevents state bloat by making storage economically costly
//! and incentivizing cleanup of unused state.
//!
//! Deposit flow:
//! 1. On `put(key, value)`: deposit = `DEPOSIT_PER_BYTE * (key.len + value.len)`
//! 2. Deposit is locked (deducted from sender's balance, held by protocol)
//! 3. On `delete(key)`: deposit is refunded to the original depositor
//!
//! Deposits are tracked in a separate ledger to avoid polluting world state versions.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Cost per byte of storage (in smallest NOTA unit).
pub const DEPOSIT_PER_BYTE: u64 = 1;

/// Minimum deposit regardless of data size.
pub const MIN_DEPOSIT: u64 = 10;

/// Maximum key+value size for a single entry (64 KB).
pub const MAX_ENTRY_SIZE: usize = 65_536;

/// A single storage deposit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositRecord {
    /// The account that paid the deposit.
    pub depositor: String,
    /// The world state key this deposit covers.
    pub key: String,
    /// Amount locked.
    pub amount: u64,
    /// Block height when the deposit was made.
    pub created_at: u64,
}

/// Calculate the required deposit for a key-value entry.
pub fn calculate_deposit(key: &str, value: &[u8]) -> Result<u64, DepositError> {
    let total_bytes = key.len() + value.len();
    if total_bytes > MAX_ENTRY_SIZE {
        return Err(DepositError::EntryTooLarge {
            size: total_bytes,
            max: MAX_ENTRY_SIZE,
        });
    }
    let deposit = (total_bytes as u64) * DEPOSIT_PER_BYTE;
    Ok(deposit.max(MIN_DEPOSIT))
}

/// Errors from the deposit system.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DepositError {
    #[error("insufficient balance: need {required}, have {available}")]
    InsufficientBalance { required: u64, available: u64 },
    #[error("entry too large: {size} bytes exceeds max {max}")]
    EntryTooLarge { size: usize, max: usize },
    #[error("no deposit found for key '{0}'")]
    NotFound(String),
}

/// In-memory deposit ledger.
///
/// Tracks active deposits keyed by world state key.
/// Thread-safe via internal `Mutex`.
pub struct DepositLedger {
    /// Active deposits: world_state_key → DepositRecord.
    deposits: Mutex<HashMap<String, DepositRecord>>,
    /// Total tokens currently locked in deposits.
    total_locked: Mutex<u64>,
}

impl DepositLedger {
    pub fn new() -> Self {
        Self {
            deposits: Mutex::new(HashMap::new()),
            total_locked: Mutex::new(0),
        }
    }

    /// Lock a deposit for a new or updated key.
    ///
    /// If the key already has a deposit (update scenario), the old deposit
    /// is refunded and the new one is charged. Returns the net cost
    /// (new deposit - refunded old deposit, or 0 if net negative).
    pub fn lock(
        &self,
        depositor: &str,
        key: &str,
        value: &[u8],
        block_height: u64,
        balance: u64,
    ) -> Result<DepositResult, DepositError> {
        let new_deposit = calculate_deposit(key, value)?;

        let mut deposits = self.deposits.lock().unwrap();
        let mut total = self.total_locked.lock().unwrap();

        // Refund existing deposit if updating.
        let refund = deposits.get(key).map(|r| r.amount).unwrap_or(0);
        let net_cost = new_deposit.saturating_sub(refund);

        if net_cost > balance {
            return Err(DepositError::InsufficientBalance {
                required: net_cost,
                available: balance,
            });
        }

        let record = DepositRecord {
            depositor: depositor.to_string(),
            key: key.to_string(),
            amount: new_deposit,
            created_at: block_height,
        };

        deposits.insert(key.to_string(), record);
        *total = total.saturating_sub(refund).saturating_add(new_deposit);

        Ok(DepositResult {
            deposit_amount: new_deposit,
            refunded: refund,
            net_cost,
        })
    }

    /// Refund the deposit for a deleted key.
    ///
    /// Returns the refunded amount and the original depositor address.
    pub fn refund(&self, key: &str) -> Result<(String, u64), DepositError> {
        let mut deposits = self.deposits.lock().unwrap();
        let mut total = self.total_locked.lock().unwrap();

        let record = deposits
            .remove(key)
            .ok_or_else(|| DepositError::NotFound(key.to_string()))?;

        *total = total.saturating_sub(record.amount);

        Ok((record.depositor, record.amount))
    }

    /// Get the deposit for a key (if any).
    pub fn get(&self, key: &str) -> Option<DepositRecord> {
        self.deposits.lock().unwrap().get(key).cloned()
    }

    /// Total tokens locked across all deposits.
    pub fn total_locked(&self) -> u64 {
        *self.total_locked.lock().unwrap()
    }

    /// Number of active deposits.
    pub fn deposit_count(&self) -> usize {
        self.deposits.lock().unwrap().len()
    }
}

impl Default for DepositLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a lock operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositResult {
    /// Total deposit locked for this key.
    pub deposit_amount: u64,
    /// Amount refunded from a previous deposit on the same key.
    pub refunded: u64,
    /// Net cost to the depositor (deposit_amount - refunded).
    pub net_cost: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- calculate_deposit ---

    #[test]
    fn deposit_minimum_for_small_entry() {
        // key="a" (1 byte) + value=[1] (1 byte) = 2 bytes * 1 = 2, but min is 10.
        assert_eq!(calculate_deposit("a", &[1]).unwrap(), MIN_DEPOSIT);
    }

    #[test]
    fn deposit_scales_with_size() {
        // 100 bytes key + 100 bytes value = 200 * 1 = 200.
        let key = "k".repeat(100);
        let value = vec![0u8; 100];
        assert_eq!(calculate_deposit(&key, &value).unwrap(), 200);
    }

    #[test]
    fn deposit_rejects_oversized_entry() {
        let key = "k".repeat(MAX_ENTRY_SIZE + 1);
        let err = calculate_deposit(&key, &[]).unwrap_err();
        assert!(matches!(err, DepositError::EntryTooLarge { .. }));
    }

    // --- DepositLedger::lock ---

    #[test]
    fn lock_new_deposit() {
        let ledger = DepositLedger::new();
        let result = ledger.lock("alice", "k1", &[0u8; 100], 1, 1000).unwrap();
        assert_eq!(result.deposit_amount, 102); // "k1"(2) + 100 = 102 bytes * 1 = 102
        assert_eq!(result.refunded, 0);
        assert_eq!(result.net_cost, 102);
        assert_eq!(ledger.total_locked(), 102);
        assert_eq!(ledger.deposit_count(), 1);
    }

    #[test]
    fn lock_insufficient_balance() {
        let ledger = DepositLedger::new();
        let err = ledger.lock("alice", "k1", &[0u8; 100], 1, 5).unwrap_err();
        assert!(matches!(err, DepositError::InsufficientBalance { .. }));
    }

    #[test]
    fn lock_update_refunds_old_deposit() {
        let ledger = DepositLedger::new();
        // First lock: 50 bytes → deposit = 52 ("k1" = 2 + 50).
        ledger.lock("alice", "k1", &[0u8; 50], 1, 1000).unwrap();
        assert_eq!(ledger.total_locked(), 52);

        // Update: 100 bytes → deposit = 102. Old deposit 52 refunded.
        let result = ledger.lock("alice", "k1", &[0u8; 100], 2, 1000).unwrap();
        assert_eq!(result.deposit_amount, 102);
        assert_eq!(result.refunded, 52);
        assert_eq!(result.net_cost, 50); // 102 - 52
        assert_eq!(ledger.total_locked(), 102); // Only new deposit
    }

    #[test]
    fn lock_update_shrink_refunds_excess() {
        let ledger = DepositLedger::new();
        // First: 200 bytes.
        ledger.lock("alice", "k1", &[0u8; 200], 1, 1000).unwrap();
        assert_eq!(ledger.total_locked(), 202);

        // Shrink to 10 bytes — net cost 0, refund exceeds new deposit.
        let result = ledger.lock("alice", "k1", &[0u8; 10], 2, 0).unwrap();
        assert_eq!(result.net_cost, 0);
        assert_eq!(ledger.total_locked(), 12);
    }

    // --- DepositLedger::refund ---

    #[test]
    fn refund_returns_depositor_and_amount() {
        let ledger = DepositLedger::new();
        ledger.lock("alice", "k1", &[0u8; 50], 1, 1000).unwrap();

        let (depositor, amount) = ledger.refund("k1").unwrap();
        assert_eq!(depositor, "alice");
        assert_eq!(amount, 52);
        assert_eq!(ledger.total_locked(), 0);
        assert_eq!(ledger.deposit_count(), 0);
    }

    #[test]
    fn refund_nonexistent_key_fails() {
        let ledger = DepositLedger::new();
        let err = ledger.refund("nope").unwrap_err();
        assert!(matches!(err, DepositError::NotFound(_)));
    }

    // --- DepositLedger::get ---

    #[test]
    fn get_existing_deposit() {
        let ledger = DepositLedger::new();
        ledger.lock("alice", "k1", &[0u8; 50], 1, 1000).unwrap();

        let record = ledger.get("k1").unwrap();
        assert_eq!(record.depositor, "alice");
        assert_eq!(record.key, "k1");
        assert_eq!(record.created_at, 1);
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let ledger = DepositLedger::new();
        assert!(ledger.get("nope").is_none());
    }

    // --- lifecycle: lock → refund → lock ---

    #[test]
    fn full_lifecycle() {
        let ledger = DepositLedger::new();

        // Create.
        ledger
            .lock("alice", "asset:1", b"initial_value", 1, 10_000)
            .unwrap();
        assert_eq!(ledger.deposit_count(), 1);

        // Update (larger value).
        ledger
            .lock("alice", "asset:1", &[0u8; 500], 2, 10_000)
            .unwrap();
        assert_eq!(ledger.deposit_count(), 1); // Same key

        // Delete → full refund.
        let (depositor, _amount) = ledger.refund("asset:1").unwrap();
        assert_eq!(depositor, "alice");
        assert_eq!(ledger.deposit_count(), 0);
        assert_eq!(ledger.total_locked(), 0);

        // Re-create by different user.
        ledger
            .lock("bob", "asset:1", b"new_owner", 3, 10_000)
            .unwrap();
        assert_eq!(ledger.get("asset:1").unwrap().depositor, "bob");
    }

    // --- stress ---

    #[test]
    fn stress_1000_deposits() {
        let ledger = DepositLedger::new();
        for i in 0..1000 {
            let key = format!("key_{i}");
            ledger
                .lock("alice", &key, &[0u8; 32], i, 1_000_000)
                .unwrap();
        }
        assert_eq!(ledger.deposit_count(), 1000);
        assert!(ledger.total_locked() > 0);

        // Refund all.
        for i in 0..1000 {
            let key = format!("key_{i}");
            ledger.refund(&key).unwrap();
        }
        assert_eq!(ledger.deposit_count(), 0);
        assert_eq!(ledger.total_locked(), 0);
    }
}
