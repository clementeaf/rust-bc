//! Adversarial tests for the native cryptocurrency transaction layer.
//!
//! Each test targets a specific attack vector and must fail deterministically.

use rust_bc::account::{AccountStore, MemoryAccountStore};
use rust_bc::tokenomics::economics::EconomicsState;
use rust_bc::transaction::block_producer::produce_block;
use rust_bc::transaction::mempool::{Mempool, MempoolConfig};
use rust_bc::transaction::native::{
    execute_transfer, verify_tx_signature, NativeTransaction, NativeTxError,
};

fn default_mempool() -> Mempool {
    Mempool::new(MempoolConfig {
        max_size: 1_000,
        max_per_sender: 64,
        min_fee: 1,
    })
}

// ── Double Spend ───────────────────────────────────────────────────────────

#[test]
fn double_spend_same_nonce_rejected() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 1000)]);

    let tx1 = NativeTransaction::new_transfer("alice", "bob", 500, 0, 5);
    let tx2 = NativeTransaction::new_transfer("alice", "charlie", 500, 0, 5);

    // First transfer succeeds
    execute_transfer(&store, &tx1, "v").unwrap();

    // Second with same nonce fails — nonce already incremented to 1
    let err = execute_transfer(&store, &tx2, "v").unwrap_err();
    assert!(
        matches!(
            err,
            NativeTxError::Account(rust_bc::account::AccountError::NonceMismatch { .. })
        ),
        "expected NonceMismatch, got: {err:?}"
    );
}

#[test]
fn double_spend_via_mempool_dedup() {
    let pool = default_mempool();
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);

    assert!(pool.add(tx.clone()).unwrap()); // first → ok
    assert!(!pool.add(tx).unwrap()); // duplicate → rejected (false)
}

#[test]
fn double_spend_balance_exhaustion() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 100)]);

    // First tx: 90 + 5 fee = 95
    let tx1 = NativeTransaction::new_transfer("alice", "bob", 90, 0, 5);
    execute_transfer(&store, &tx1, "v").unwrap();

    // Second tx: tries 10 + 5 fee = 15, but alice only has 5 left
    let tx2 = NativeTransaction::new_transfer("alice", "charlie", 10, 1, 5);
    let err = execute_transfer(&store, &tx2, "v").unwrap_err();
    assert!(matches!(
        err,
        NativeTxError::Account(rust_bc::account::AccountError::InsufficientBalance { .. })
    ));
}

// ── Replay Protection ──────────────────────────────────────────────────────

#[test]
fn replay_old_nonce_rejected() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10000)]);

    // Execute nonce 0, 1, 2
    for i in 0..3 {
        let tx = NativeTransaction::new_transfer("alice", "bob", 10, i, 5);
        execute_transfer(&store, &tx, "v").unwrap();
    }

    // Replay nonce 1 → rejected
    let replay = NativeTransaction::new_transfer("alice", "bob", 10, 1, 5);
    let err = execute_transfer(&store, &replay, "v").unwrap_err();
    assert!(matches!(
        err,
        NativeTxError::Account(rust_bc::account::AccountError::NonceMismatch {
            expected: 3,
            got: 1
        })
    ));
}

#[test]
fn cross_network_replay_via_separate_stores() {
    // Two independent account stores = two networks
    let store_a = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
    let store_b = MemoryAccountStore::with_genesis(&[("alice", 1000)]);

    // Execute on network A
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    execute_transfer(&store_a, &tx, "v").unwrap();

    // Same tx on network B also succeeds (nonce 0 valid there) —
    // this is expected, cross-chain replay needs chain_id in signing payload.
    // For now, verify both networks are independent.
    execute_transfer(&store_b, &tx, "v").unwrap();

    // But replaying on A again fails
    let replay = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let err = execute_transfer(&store_a, &replay, "v").unwrap_err();
    assert!(matches!(
        err,
        NativeTxError::Account(rust_bc::account::AccountError::NonceMismatch { .. })
    ));
}

// ── Nonce Gaps ─────────────────────────────────────────────────────────────

#[test]
fn nonce_gap_rejected() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10000)]);

    // Skip nonce 0, try nonce 1
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 1, 5);
    let err = execute_transfer(&store, &tx, "v").unwrap_err();
    assert!(matches!(
        err,
        NativeTxError::Account(rust_bc::account::AccountError::NonceMismatch {
            expected: 0,
            got: 1
        })
    ));
}

#[test]
fn nonce_must_be_sequential_in_block() {
    let mempool = default_mempool();
    let store = MemoryAccountStore::with_genesis(&[("alice", 10000)]);
    let economics = EconomicsState::default();

    // Submit nonces 0, 2, 4 (gaps at 1 and 3)
    mempool
        .add(NativeTransaction::new_transfer("alice", "bob", 10, 0, 10))
        .unwrap();
    mempool
        .add(NativeTransaction::new_transfer("alice", "bob", 10, 2, 8))
        .unwrap();
    mempool
        .add(NativeTransaction::new_transfer("alice", "bob", 10, 4, 6))
        .unwrap();

    let block = produce_block(&mempool, &store, &economics, "miner", 100).unwrap();

    // Only nonce 0 should succeed; nonces 2 and 4 fail (gap)
    assert_eq!(block.tx_success_count, 1);
    let failures: Vec<_> = block.tx_results.iter().filter(|r| !r.success).collect();
    assert_eq!(failures.len(), 2);
}

// ── Mempool Spam ───────────────────────────────────────────────────────────

#[test]
fn mempool_per_sender_limit_blocks_spam() {
    let pool = Mempool::new(MempoolConfig {
        max_size: 10_000,
        max_per_sender: 5,
        min_fee: 1,
    });

    // Fill 5 txs from attacker
    for i in 0..5 {
        pool.add(NativeTransaction::new_transfer("attacker", "bob", 1, i, 5))
            .unwrap();
    }

    // 6th rejected
    let err = pool
        .add(NativeTransaction::new_transfer("attacker", "bob", 1, 5, 5))
        .unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::mempool::MempoolError::SenderFull { .. }
    ));

    // Other senders unaffected
    assert!(pool
        .add(NativeTransaction::new_transfer("legit", "bob", 1, 0, 5))
        .unwrap());
}

#[test]
fn mempool_rejects_zero_fee() {
    let pool = default_mempool();
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 0);
    let err = pool.add(tx).unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::mempool::MempoolError::FeeTooLow { .. }
    ));
}

#[test]
fn mempool_pool_full_eviction() {
    let pool = Mempool::new(MempoolConfig {
        max_size: 3,
        max_per_sender: 100,
        min_fee: 1,
    });

    // Fill with fee 5, 5, 5
    for i in 0..3 {
        pool.add(NativeTransaction::new_transfer(
            &format!("s{i}"),
            "bob",
            1,
            0,
            5,
        ))
        .unwrap();
    }

    // Fee 3 rejected (lower than all existing)
    let err = pool
        .add(NativeTransaction::new_transfer("new", "bob", 1, 0, 3))
        .unwrap_err();
    assert!(matches!(
        err,
        rust_bc::transaction::mempool::MempoolError::PoolFull
    ));

    // Fee 10 accepted (evicts one fee-5 tx)
    assert!(pool
        .add(NativeTransaction::new_transfer("rich", "bob", 1, 0, 10))
        .unwrap());
    assert_eq!(pool.len(), 3);
}

// ── Fee Sniping ────────────────────────────────────────────────────────────

#[test]
fn fee_below_base_fee_rejected_at_api_level() {
    // Simulate base fee = 10
    let base_fee = 10u64;
    let offered = 5u64;

    let result = rust_bc::tokenomics::policy::validate_fee(offered, base_fee);
    assert!(result.is_err());
}

#[test]
fn block_producer_fee_ordering_prevents_sniping() {
    let mempool = default_mempool();
    let store = MemoryAccountStore::with_genesis(&[("low_fee", 10000), ("high_fee", 10000)]);
    let economics = EconomicsState::default();

    // High fee tx submitted after low fee tx
    mempool
        .add(NativeTransaction::new_transfer("low_fee", "bob", 10, 0, 2))
        .unwrap();
    mempool
        .add(NativeTransaction::new_transfer(
            "high_fee", "bob", 10, 0, 100,
        ))
        .unwrap();

    // Produce block with limit of 1 — highest fee wins
    let block = produce_block(&mempool, &store, &economics, "miner", 1).unwrap();
    assert_eq!(block.tx_results.len(), 1);
    assert!(block.tx_results[0].success);

    // high_fee sender should have been debited
    let high = store.get_account("high_fee").unwrap();
    assert_eq!(high.balance, 10000 - 10 - 100);
    // low_fee untouched
    let low = store.get_account("low_fee").unwrap();
    assert_eq!(low.balance, 10000);
}

// ── Invalid Signature Flood ────────────────────────────────────────────────

#[test]
fn invalid_signature_flood_all_rejected() {
    for i in 0..100 {
        let mut tx = NativeTransaction::new_transfer("attacker", "victim", 1, i, 5);
        tx.signature = vec![0xDE, 0xAD, 0xBE, 0xEF]; // garbage sig

        let result = verify_tx_signature(&tx, &[0u8; 32]);
        assert!(
            result.is_err(),
            "tx {i} with garbage sig should be rejected"
        );
    }
}

#[test]
fn empty_signature_rejected() {
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let err = verify_tx_signature(&tx, &[0u8; 32]).unwrap_err();
    assert!(matches!(err, NativeTxError::MissingSignature));
}

#[test]
fn wrong_pubkey_signature_rejected() {
    use rust_bc::identity::signing::{SigningProvider, SoftwareSigningProvider};

    let signer = SoftwareSigningProvider::generate();
    let wrong_key = SoftwareSigningProvider::generate();

    let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let payload = tx.signing_payload();
    tx.signature = signer.sign(&payload).unwrap();

    // Verify with wrong pubkey → false
    let valid = verify_tx_signature(&tx, &wrong_key.public_key()).unwrap();
    assert!(!valid, "wrong pubkey should not validate");
}

// ── Economic Invariants in Block Production ────────────────────────────────

#[test]
fn supply_invariant_holds_over_100_blocks() {
    let mempool = default_mempool();
    let store = MemoryAccountStore::with_genesis(&[("alice", 1_000_000)]);
    let mut economics = EconomicsState::default();

    for i in 0..100 {
        // Add some txs every 5 blocks
        if i % 5 == 0 {
            let nonce = store.get_account("alice").unwrap().nonce;
            let tx = NativeTransaction::new_transfer("alice", "bob", 10, nonce, 5);
            let _ = mempool.add(tx);
        }

        let block = produce_block(&mempool, &store, &economics, "miner", 50).unwrap();
        economics = block.economics;

        // Invariant: minted never exceeds MAX_SUPPLY
        assert!(
            economics.total_minted <= rust_bc::tokenomics::economics::MAX_SUPPLY,
            "block {i}: minted {} > MAX_SUPPLY",
            economics.total_minted
        );

        // Invariant: burned never exceeds minted
        assert!(
            economics.total_burned <= economics.total_minted,
            "block {i}: burned {} > minted {}",
            economics.total_burned,
            economics.total_minted
        );
    }
}

#[test]
fn proposer_receives_both_reward_and_fee_share() {
    let mempool = default_mempool();
    let store = MemoryAccountStore::with_genesis(&[("alice", 10000)]);
    let economics = EconomicsState::default();

    // Submit tx with high fee
    mempool
        .add(NativeTransaction::new_transfer("alice", "bob", 100, 0, 100))
        .unwrap();

    let _block = produce_block(&mempool, &store, &economics, "miner", 10).unwrap();

    let miner = store.get_account("miner").unwrap();
    // Miner gets: block reward (50) + 20% of 100 fee (20) = 70
    assert_eq!(
        miner.balance,
        50 + 20,
        "miner balance wrong: {}",
        miner.balance
    );
}
