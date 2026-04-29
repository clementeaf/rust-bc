//! API endpoint correctness tests for the native cryptocurrency layer.
//!
//! Tests JSON schema, status codes, invalid input handling, and error messages.
//! Uses library-level constructs rather than full HTTP to avoid massive AppState setup.

use rust_bc::account::{AccountStore, MemoryAccountStore};
use rust_bc::tokenomics::economics::EconomicsState;
use rust_bc::transaction::mempool::{Mempool, MempoolConfig};
use rust_bc::transaction::native::NativeTransaction;

fn make_mempool() -> Mempool {
    Mempool::new(MempoolConfig {
        max_size: 100,
        max_per_sender: 10,
        min_fee: 1,
    })
}

// ── GET /accounts/{address} equivalents ────────────────────────────────────

#[test]
fn get_account_returns_default_for_unknown() {
    let store = MemoryAccountStore::new();
    let acc = store.get_account("nonexistent").unwrap();
    assert_eq!(acc.balance, 0);
    assert_eq!(acc.nonce, 0);
    assert!(!acc.is_contract());
}

#[test]
fn get_account_returns_correct_balance() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 5000)]);
    let acc = store.get_account("alice").unwrap();
    assert_eq!(acc.balance, 5000);
    assert_eq!(acc.nonce, 0);
}

#[test]
fn get_account_after_transfer_reflects_changes() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
    let tx = NativeTransaction::new_transfer("alice", "bob", 300, 0, 5);
    rust_bc::transaction::native::execute_transfer(&store, &tx, "miner").unwrap();

    let alice = store.get_account("alice").unwrap();
    assert_eq!(alice.balance, 695);
    assert_eq!(alice.nonce, 1);

    let bob = store.get_account("bob").unwrap();
    assert_eq!(bob.balance, 300);
}

#[test]
fn account_response_serializes_correctly() {
    let acc = rust_bc::account::AccountState::new(42);
    let json = serde_json::to_value(&acc).unwrap();
    assert_eq!(json["balance"], 42);
    assert_eq!(json["nonce"], 0);
    assert!(json.get("code_hash").is_none() || json["code_hash"].is_null());
}

// ── POST /transfer equivalents ─────────────────────────────────────────────

#[test]
fn transfer_valid_accepted_by_mempool() {
    let pool = make_mempool();
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    assert!(pool.add(tx).unwrap());
    assert_eq!(pool.len(), 1);
}

#[test]
fn transfer_zero_amount_rejected_at_execution() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
    let tx = NativeTransaction::new_transfer("alice", "bob", 0, 0, 5);
    let err = rust_bc::transaction::native::execute_transfer(&store, &tx, "v").unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::native::NativeTxError::ZeroAmount
    ));
}

#[test]
fn transfer_self_send_rejected() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
    let tx = NativeTransaction::new_transfer("alice", "alice", 100, 0, 5);
    let err = rust_bc::transaction::native::execute_transfer(&store, &tx, "v").unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::native::NativeTxError::SelfTransfer
    ));
}

#[test]
fn transfer_insufficient_balance_rejected() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10)]);
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let err = rust_bc::transaction::native::execute_transfer(&store, &tx, "v").unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::native::NativeTxError::Account(
            rust_bc::account::AccountError::InsufficientBalance { .. }
        )
    ));
}

#[test]
fn transfer_fee_below_minimum_rejected_by_mempool() {
    let pool = make_mempool();
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 0);
    let err = pool.add(tx).unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::mempool::MempoolError::FeeTooLow { .. }
    ));
}

#[test]
fn transfer_duplicate_rejected_by_mempool() {
    let pool = make_mempool();
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    assert!(pool.add(tx.clone()).unwrap());
    assert!(!pool.add(tx).unwrap());
}

#[test]
fn transfer_response_fields_present() {
    let tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 500, 3, 10, 9999);
    let json = serde_json::to_value(&tx).unwrap();

    assert!(json.get("id").is_some());
    assert_eq!(json["chain_id"], 9999);
    assert_eq!(json["nonce"], 3);
    assert_eq!(json["fee"], 10);
    assert!(json.get("kind").is_some());
}

// ── GET /mempool/stats equivalents ─────────────────────────────────────────

#[test]
fn mempool_stats_empty() {
    let pool = make_mempool();
    assert_eq!(pool.len(), 0);
    assert!(pool.is_empty());
}

#[test]
fn mempool_stats_after_adds() {
    let pool = make_mempool();
    for i in 0..5 {
        pool.add(NativeTransaction::new_transfer("a", "b", 1, i, 5))
            .unwrap();
    }
    assert_eq!(pool.len(), 5);
}

#[test]
fn mempool_base_fee_from_economics() {
    let economics = EconomicsState::default();
    assert_eq!(economics.base_fee, 1); // MIN_BASE_FEE
}

// ── Error handling (no panics) ─────────────────────────────────────────────

#[test]
fn empty_address_graceful_error() {
    let store = MemoryAccountStore::new();
    // Empty string is a valid key (returns default), no panic
    let acc = store.get_account("").unwrap();
    assert_eq!(acc.balance, 0);
}

#[test]
fn very_long_address_no_panic() {
    let store = MemoryAccountStore::new();
    let long_addr = "a".repeat(10_000);
    let acc = store.get_account(&long_addr).unwrap();
    assert_eq!(acc.balance, 0);
}

#[test]
fn concurrent_mempool_access_no_panic() {
    let pool = std::sync::Arc::new(make_mempool());
    std::thread::scope(|s| {
        for t in 0..10 {
            let pool = pool.clone();
            s.spawn(move || {
                for i in 0..10 {
                    let tx =
                        NativeTransaction::new_transfer(&format!("sender_{t}"), "bob", 1, i, 5);
                    let _ = pool.add(tx);
                }
            });
        }
    });
    // No panic, pool has some txs
    assert!(pool.len() > 0);
}
