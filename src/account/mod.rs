//! Account state model for cryptocurrency operations.
//!
//! Each account is identified by an address (derived from public key) and
//! holds a balance of NOTA tokens, a nonce for replay protection, and an
//! optional code hash for smart contract accounts.

pub mod address;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

// ── Types ──────────────────────────────────────────────────────────────────

/// A single account in the global state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountState {
    /// NOTA token balance (in smallest unit).
    pub balance: u64,
    /// Monotonically increasing nonce — each outgoing tx must match and increment.
    pub nonce: u64,
    /// Optional code hash for contract accounts (empty for EOAs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_hash: Option<String>,
}

impl AccountState {
    pub fn new(balance: u64) -> Self {
        Self {
            balance,
            nonce: 0,
            code_hash: None,
        }
    }

    /// Returns true if this is a contract account (has code).
    pub fn is_contract(&self) -> bool {
        self.code_hash.is_some()
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("account not found: {0}")]
    NotFound(String),
    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },
    #[error("nonce mismatch: expected {expected}, got {got}")]
    NonceMismatch { expected: u64, got: u64 },
    #[error("arithmetic overflow")]
    Overflow,
    #[error("internal storage error: {0}")]
    Internal(String),
}

// ── Trait ───────────────────────────────────────────────────────────────────

/// Trait for account state storage.
///
/// Implementations must be thread-safe (`Send + Sync`).
pub trait AccountStore: Send + Sync {
    /// Get account state, returning default (zero balance, zero nonce) if not found.
    fn get_account(&self, address: &str) -> Result<AccountState, AccountError>;

    /// Get account only if it exists, None otherwise.
    fn get_account_if_exists(&self, address: &str) -> Result<Option<AccountState>, AccountError>;

    /// Set account state (upsert).
    fn set_account(&self, address: &str, state: &AccountState) -> Result<(), AccountError>;

    /// Transfer `amount` from `from` to `to`, incrementing sender nonce.
    /// Returns updated (from, to) states.
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

        self.set_account(from, &sender)?;
        self.set_account(to, &recipient)?;

        Ok((sender, recipient))
    }

    /// Credit an account (block reward, mint). No nonce change.
    fn credit(&self, address: &str, amount: u64) -> Result<AccountState, AccountError> {
        let mut acc = self.get_account(address)?;
        acc.balance = acc
            .balance
            .checked_add(amount)
            .ok_or(AccountError::Overflow)?;
        self.set_account(address, &acc)?;
        Ok(acc)
    }

    /// Debit an account (fee burn, slash). No nonce change.
    fn debit(&self, address: &str, amount: u64) -> Result<AccountState, AccountError> {
        let mut acc = self.get_account(address)?;
        if acc.balance < amount {
            return Err(AccountError::InsufficientBalance {
                have: acc.balance,
                need: amount,
            });
        }
        acc.balance -= amount;
        self.set_account(address, &acc)?;
        Ok(acc)
    }

    /// Get all accounts (for genesis, snapshots).
    fn all_accounts(&self) -> Result<Vec<(String, AccountState)>, AccountError>;
}

// ── In-Memory Implementation ───────────────────────────────────────────────

/// Thread-safe in-memory account store.
pub struct MemoryAccountStore {
    accounts: Mutex<HashMap<String, AccountState>>,
}

impl MemoryAccountStore {
    pub fn new() -> Self {
        Self {
            accounts: Mutex::new(HashMap::new()),
        }
    }

    /// Create store with genesis allocations.
    pub fn with_genesis(allocations: &[(&str, u64)]) -> Self {
        let mut map = HashMap::new();
        for (addr, balance) in allocations {
            map.insert((*addr).to_string(), AccountState::new(*balance));
        }
        Self {
            accounts: Mutex::new(map),
        }
    }
}

impl Default for MemoryAccountStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountStore for MemoryAccountStore {
    fn get_account(&self, address: &str) -> Result<AccountState, AccountError> {
        let accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        Ok(accounts.get(address).cloned().unwrap_or_default())
    }

    fn get_account_if_exists(&self, address: &str) -> Result<Option<AccountState>, AccountError> {
        let accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        Ok(accounts.get(address).cloned())
    }

    fn set_account(&self, address: &str, state: &AccountState) -> Result<(), AccountError> {
        let mut accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        accounts.insert(address.to_string(), state.clone());
        Ok(())
    }

    fn all_accounts(&self) -> Result<Vec<(String, AccountState)>, AccountError> {
        let accounts = self.accounts.lock().unwrap_or_else(|e| e.into_inner());
        Ok(accounts
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_account_is_zero() {
        let acc = AccountState::default();
        assert_eq!(acc.balance, 0);
        assert_eq!(acc.nonce, 0);
        assert!(!acc.is_contract());
    }

    #[test]
    fn new_account_with_balance() {
        let acc = AccountState::new(1000);
        assert_eq!(acc.balance, 1000);
        assert_eq!(acc.nonce, 0);
    }

    #[test]
    fn genesis_allocations() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 5000), ("bob", 3000)]);
        let alice = store.get_account("alice").unwrap();
        let bob = store.get_account("bob").unwrap();
        assert_eq!(alice.balance, 5000);
        assert_eq!(bob.balance, 3000);
    }

    #[test]
    fn get_nonexistent_returns_default() {
        let store = MemoryAccountStore::new();
        let acc = store.get_account("unknown").unwrap();
        assert_eq!(acc.balance, 0);
        assert_eq!(acc.nonce, 0);
    }

    #[test]
    fn get_if_exists_returns_none() {
        let store = MemoryAccountStore::new();
        assert!(store.get_account_if_exists("unknown").unwrap().is_none());
    }

    #[test]
    fn set_and_get_account() {
        let store = MemoryAccountStore::new();
        let acc = AccountState::new(42);
        store.set_account("alice", &acc).unwrap();
        let retrieved = store.get_account("alice").unwrap();
        assert_eq!(retrieved.balance, 42);
    }

    #[test]
    fn transfer_success() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let (sender, recipient) = store.transfer("alice", "bob", 300, 0).unwrap();
        assert_eq!(sender.balance, 700);
        assert_eq!(sender.nonce, 1);
        assert_eq!(recipient.balance, 300);
    }

    #[test]
    fn transfer_insufficient_balance() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 100)]);
        let err = store.transfer("alice", "bob", 200, 0).unwrap_err();
        assert!(matches!(
            err,
            AccountError::InsufficientBalance {
                have: 100,
                need: 200
            }
        ));
    }

    #[test]
    fn transfer_nonce_mismatch() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let err = store.transfer("alice", "bob", 100, 5).unwrap_err();
        assert!(matches!(
            err,
            AccountError::NonceMismatch {
                expected: 0,
                got: 5
            }
        ));
    }

    #[test]
    fn transfer_increments_nonce_sequentially() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        store.transfer("alice", "bob", 100, 0).unwrap();
        store.transfer("alice", "bob", 100, 1).unwrap();
        store.transfer("alice", "bob", 100, 2).unwrap();
        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.nonce, 3);
        assert_eq!(alice.balance, 700);
    }

    #[test]
    fn credit_account() {
        let store = MemoryAccountStore::with_genesis(&[("miner", 0)]);
        let acc = store.credit("miner", 50).unwrap();
        assert_eq!(acc.balance, 50);
        // Credit again
        let acc = store.credit("miner", 25).unwrap();
        assert_eq!(acc.balance, 75);
    }

    #[test]
    fn credit_nonexistent_creates_account() {
        let store = MemoryAccountStore::new();
        let acc = store.credit("new_addr", 100).unwrap();
        assert_eq!(acc.balance, 100);
    }

    #[test]
    fn debit_account() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 500)]);
        let acc = store.debit("alice", 200).unwrap();
        assert_eq!(acc.balance, 300);
    }

    #[test]
    fn debit_insufficient_fails() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 100)]);
        let err = store.debit("alice", 200).unwrap_err();
        assert!(matches!(err, AccountError::InsufficientBalance { .. }));
    }

    #[test]
    fn all_accounts_returns_everything() {
        let store = MemoryAccountStore::with_genesis(&[("a", 1), ("b", 2), ("c", 3)]);
        let all = store.all_accounts().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn transfer_overflow_protection() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 100), ("bob", u64::MAX)]);
        let err = store.transfer("alice", "bob", 100, 0).unwrap_err();
        assert!(matches!(err, AccountError::Overflow));
    }

    #[test]
    fn contract_account() {
        let acc = AccountState {
            balance: 0,
            nonce: 0,
            code_hash: Some("abc123".to_string()),
        };
        assert!(acc.is_contract());
    }

    #[test]
    fn serde_roundtrip() {
        let acc = AccountState::new(12345);
        let json = serde_json::to_string(&acc).unwrap();
        let deserialized: AccountState = serde_json::from_str(&json).unwrap();
        assert_eq!(acc, deserialized);
    }
}
