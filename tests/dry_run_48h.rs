//! Dry run simulation — validates the system survives 48h-equivalent load.
//!
//! Simulates in-process:
//! - 1000 native transfers across multiple senders
//! - Faucet spam (100 rapid requests)
//! - Node restart (RocksDB close + reopen)
//! - State consistency verification after each phase
//! - Economic invariant checks throughout
//!
//! This is the "does it break in 48h?" test, run without Docker.

use rust_bc::account::address::address_from_pubkey;
use rust_bc::account::metrics::CryptoMetrics;
use rust_bc::account::rocksdb_store::RocksDbAccountStore;
use rust_bc::account::sync::{compute_state_hash, replay_from_genesis};
use rust_bc::account::{AccountStore, MemoryAccountStore};
use rust_bc::identity::signing::{SigningProvider, SoftwareSigningProvider};
use rust_bc::testnet::faucet::{Faucet, FaucetConfig, FaucetError};
use rust_bc::tokenomics::economics::{EconomicsState, MAX_SUPPLY};
use rust_bc::tokenomics::policy::verify_supply_invariant;
use rust_bc::transaction::block_producer::produce_block;
use rust_bc::transaction::mempool::{Mempool, MempoolConfig};
use rust_bc::transaction::native::{verify_tx_signature, NativeTransaction};

use std::sync::Arc;
use tempfile::TempDir;

// ── Helpers ────────────────────────────────────────────────────────────────

struct TestNode {
    store: MemoryAccountStore,
    mempool: Mempool,
    economics: EconomicsState,
    metrics: CryptoMetrics,
    faucet: Faucet,
}

impl TestNode {
    fn new() -> Self {
        Self {
            store: MemoryAccountStore::new(),
            mempool: Mempool::new(MempoolConfig {
                max_size: 5_000,
                max_per_sender: 200,
                min_fee: 1,
            }),
            economics: EconomicsState::default(),
            metrics: CryptoMetrics::new(),
            faucet: Faucet::new(FaucetConfig {
                drip_amount: 10_000,
                cooldown_blocks: 0, // no cooldown for stress test
                max_total: 0,       // unlimited
                enabled: true,
                max_drips_per_ip_per_day: 0,
                max_daily_total: 0,
            }),
        }
    }

    fn fund_account(&self, addr: &str) {
        let _ = self.faucet.drip(addr, self.economics.height);
        self.store.credit(addr, 10_000).unwrap();
    }

    fn produce_block(&mut self, proposer: &str, max_txs: usize) {
        let block = produce_block(
            &self.mempool,
            &self.store,
            &self.economics,
            proposer,
            max_txs,
        )
        .unwrap();
        self.metrics.inc_blocks();
        self.metrics.add_rewards(block.block_reward);
        self.metrics.add_burned(block.total_burned);
        self.metrics.add_proposer_fees(block.total_proposer_fees);
        for r in &block.tx_results {
            if r.success {
                self.metrics.inc_transfers();
            } else {
                self.metrics.inc_failed();
            }
        }
        self.economics = block.economics;
    }
}

struct Wallet {
    provider: SoftwareSigningProvider,
    address: String,
}

impl Wallet {
    fn generate() -> Self {
        let provider = SoftwareSigningProvider::generate();
        let address = address_from_pubkey(&provider.public_key());
        Self { provider, address }
    }

    fn sign_transfer(&self, to: &str, amount: u64, nonce: u64, fee: u64) -> NativeTransaction {
        let mut tx = NativeTransaction::new_transfer(&self.address, to, amount, nonce, fee);
        let payload = tx.signing_payload();
        tx.signature = self.provider.sign(&payload).unwrap();
        tx.signature_algorithm = "ed25519".to_string();
        tx
    }
}

// ── Phase 1: 1000 Transfers ───────────────────────────────────────────────

#[test]
fn phase1_1000_transfers() {
    let mut node = TestNode::new();

    // Create 10 funded wallets
    let wallets: Vec<Wallet> = (0..10).map(|_| Wallet::generate()).collect();
    for w in &wallets {
        node.fund_account(&w.address);
    }

    // Submit 1000 transfers: each wallet sends 100 txs
    let mut submitted = 0;
    for (w_idx, wallet) in wallets.iter().enumerate() {
        let recipient = &wallets[(w_idx + 1) % wallets.len()].address;
        for nonce in 0..100u64 {
            let tx = wallet.sign_transfer(recipient, 10, nonce, 2);

            // Verify signature before mempool
            assert!(
                verify_tx_signature(&tx, &wallet.provider.public_key()).unwrap(),
                "tx {} signature invalid",
                submitted
            );

            match node.mempool.add(tx) {
                Ok(true) => submitted += 1,
                Ok(false) => {} // dup
                Err(e) => panic!("mempool rejected tx {submitted}: {e}"),
            }
        }
    }
    assert_eq!(submitted, 1000);

    // Produce blocks until mempool drains
    let mut blocks = 0;
    while !node.mempool.is_empty() && blocks < 100 {
        node.produce_block("validator-1", 50);
        blocks += 1;
    }

    assert!(node.mempool.is_empty(), "mempool should drain");
    assert!(
        blocks <= 20,
        "1000 txs in batches of 50 = ~20 blocks, got {blocks}"
    );

    // Economic invariants
    assert!(node.economics.total_minted <= MAX_SUPPLY);
    assert!(node.economics.total_burned <= node.economics.total_minted);
    verify_supply_invariant(node.economics.total_minted, node.economics.total_burned).unwrap();

    let snap = node.metrics.snapshot();
    assert!(snap.transfers_total > 0);
    assert_eq!(snap.blocks_produced, blocks as u64);

    println!(
        "Phase 1: {submitted} txs, {blocks} blocks, {} successful transfers, {} failed",
        snap.transfers_total, snap.transfers_failed
    );
}

// ── Phase 2: Faucet Spam ──────────────────────────────────────────────────

#[test]
fn phase2_faucet_spam() {
    let faucet = Faucet::new(FaucetConfig {
        drip_amount: 100,
        cooldown_blocks: 10,
        max_total: 50_000,
        enabled: true,
        max_drips_per_ip_per_day: 5,
        max_daily_total: 10_000,
    });

    let mut successes = 0;
    let mut ip_rejects = 0;
    let mut _cooldown_rejects = 0;
    let mut daily_rejects = 0;
    let mut depleted_rejects = 0;

    // 100 rapid requests from same IP, different addresses
    for i in 0..100u64 {
        let addr = format!("spammer_{i}");
        match faucet.drip_with_ip(&addr, i * 10, "1.2.3.4", 1) {
            Ok(_) => successes += 1,
            Err(FaucetError::IpLimitExceeded { .. }) => ip_rejects += 1,
            Err(FaucetError::DailyCapReached { .. }) => daily_rejects += 1,
            Err(FaucetError::Depleted { .. }) => depleted_rejects += 1,
            Err(FaucetError::Cooldown { .. }) => _cooldown_rejects += 1,
            Err(e) => panic!("unexpected faucet error: {e}"),
        }
    }

    // IP limit should kick in after 5
    assert_eq!(successes, 5, "only 5 should pass IP limit");
    assert_eq!(ip_rejects, 95, "95 should be IP-rejected");

    // Now different IPs but same day — daily cap should apply
    let mut daily_successes = 0;
    for i in 0..200u64 {
        let addr = format!("user_{i}");
        let ip = format!("10.0.{}.{}", i / 256, i % 256);
        match faucet.drip_with_ip(&addr, i * 10, &ip, 1) {
            Ok(_) => daily_successes += 1,
            Err(FaucetError::DailyCapReached { .. }) => daily_rejects += 1,
            Err(FaucetError::Depleted { .. }) => depleted_rejects += 1,
            Err(_) => {}
        }
    }

    // 5 already dripped (500 NOTA). Daily cap is 10,000. Remaining: 9,500. At 100/drip = 95 more.
    assert!(daily_successes > 0);
    assert!(daily_rejects > 0, "daily cap should trigger");

    println!(
        "Phase 2: faucet spam — {successes} passed IP limit, {daily_successes} more passed, {} daily rejected, {} depleted",
        daily_rejects, depleted_rejects
    );
}

// ── Phase 3: Node Restart (RocksDB persistence) ───────────────────────────

#[test]
fn phase3_node_restart() {
    let dir = TempDir::new().unwrap();

    // Phase A: produce blocks and persist state
    let state_hash_before;
    let height_before;
    {
        let store = RocksDbAccountStore::new(dir.path()).unwrap();
        store.credit("alice", 100_000).unwrap();

        // Simulate 50 transfers
        for i in 0..50u64 {
            store.transfer("alice", "bob", 100, i).unwrap();
        }

        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.nonce, 50);
        assert_eq!(alice.balance, 100_000 - 50 * 100);

        // Compute state hash
        state_hash_before = compute_state_hash(&store).unwrap();
        height_before = alice.nonce; // use nonce as proxy for "progress"
    }
    // RocksDB dropped here — simulates node crash/restart

    // Phase B: reopen and verify
    {
        let store = RocksDbAccountStore::new(dir.path()).unwrap();
        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.nonce, 50, "nonce must survive restart");
        assert_eq!(
            alice.balance,
            100_000 - 50 * 100,
            "balance must survive restart"
        );

        let bob = store.get_account("bob").unwrap();
        assert_eq!(bob.balance, 50 * 100, "bob balance must survive restart");

        let state_hash_after = compute_state_hash(&store).unwrap();
        assert_eq!(
            state_hash_before, state_hash_after,
            "state hash must be identical after restart"
        );
    }

    println!("Phase 3: node restart — state hash identical, {height_before} txs survived");
}

// ── Phase 4: Replay Determinism ───────────────────────────────────────────

#[test]
fn phase4_replay_determinism() {
    // Build a transaction log
    let wallets: Vec<Wallet> = (0..5).map(|_| Wallet::generate()).collect();

    let mut blocks: Vec<(String, Vec<NativeTransaction>)> = Vec::new();
    for block_num in 0..20u64 {
        let sender = &wallets[(block_num as usize) % wallets.len()];
        let recipient = &wallets[((block_num as usize) + 1) % wallets.len()];
        let txs = if block_num % 2 == 0 {
            vec![NativeTransaction::new_transfer(
                &sender.address,
                &recipient.address,
                10,
                block_num / 5, // nonce increments per 5 blocks per sender
                2,
            )]
        } else {
            vec![]
        };
        blocks.push(("proposer".to_string(), txs));
    }

    // Replay on two independent stores
    let store_a = MemoryAccountStore::new();
    let store_b = MemoryAccountStore::new();

    // Fund accounts identically
    for w in &wallets {
        store_a.credit(&w.address, 100_000).unwrap();
        store_b.credit(&w.address, 100_000).unwrap();
    }

    let econ = EconomicsState::default();
    let result_a = replay_from_genesis(blocks.clone().into_iter(), &store_a, &econ).unwrap();
    let result_b = replay_from_genesis(blocks.into_iter(), &store_b, &econ).unwrap();

    assert_eq!(result_a.height, result_b.height);
    assert_eq!(result_a.txs_replayed, result_b.txs_replayed);

    let hash_a = compute_state_hash(&store_a).unwrap();
    let hash_b = compute_state_hash(&store_b).unwrap();
    assert_eq!(hash_a, hash_b, "replays must produce identical state");

    println!(
        "Phase 4: replay determinism — {} blocks, {} txs, state hashes match",
        result_a.height, result_a.txs_replayed
    );
}

// ── Phase 5: Metrics + Invariants Under Load ──────────────────────────────

#[test]
fn phase5_metrics_and_invariants() {
    let mut node = TestNode::new();
    let _metrics = Arc::new(CryptoMetrics::new());

    // Fund one wallet heavily
    node.store.credit("whale", 1_000_000).unwrap();

    // Produce 200 blocks with varying load
    for block in 0..200u64 {
        // Every 3rd block, submit some transfers
        if block % 3 == 0 {
            let nonce = node.store.get_account("whale").unwrap().nonce;
            let tx = NativeTransaction::new_transfer("whale", "receiver", 10, nonce, 2);
            let _ = node.mempool.add(tx);
        }

        node.produce_block("validator", 10);

        // Check invariants every block
        assert!(
            node.economics.total_minted <= MAX_SUPPLY,
            "block {block}: minted exceeds MAX_SUPPLY"
        );
        verify_supply_invariant(node.economics.total_minted, node.economics.total_burned)
            .unwrap_or_else(|e| panic!("block {block}: supply invariant violated: {e}"));
    }

    assert_eq!(node.economics.height, 200);

    let whale = node.store.get_account("whale").unwrap();
    let receiver = node.store.get_account("receiver").unwrap();

    // whale sent ~67 txs (200/3), each 10+2=12 NOTA
    assert!(whale.nonce > 60);
    assert!(receiver.balance > 600);

    // Validator got 200 block rewards + fee shares
    let validator = node.store.get_account("validator").unwrap();
    assert!(validator.balance >= 200 * 50); // at least block rewards

    println!(
        "Phase 5: 200 blocks, whale nonce={}, receiver balance={}, validator balance={}",
        whale.nonce, receiver.balance, validator.balance
    );
}

// ── Phase 6: Concurrent Mempool Stress ────────────────────────────────────

#[test]
fn phase6_concurrent_mempool_stress() {
    let mempool = Arc::new(Mempool::new(MempoolConfig {
        max_size: 10_000,
        max_per_sender: 500,
        min_fee: 1,
    }));

    // 10 threads, each submitting 100 txs
    std::thread::scope(|s| {
        for t in 0..10 {
            let mp = mempool.clone();
            s.spawn(move || {
                for i in 0..100u64 {
                    let tx = NativeTransaction::new_transfer(
                        &format!("sender_{t}"),
                        "recipient",
                        1,
                        i,
                        2,
                    );
                    let _ = mp.add(tx);
                }
            });
        }
    });

    assert!(mempool.len() > 0, "some txs should be in pool");
    assert!(mempool.len() <= 1000, "at most 1000 txs");

    // Drain all
    let drained = mempool.drain_top(10_000);
    assert!(!drained.is_empty());
    assert!(mempool.is_empty(), "pool should be empty after drain");

    println!(
        "Phase 6: concurrent stress — {} txs accepted, {} drained",
        drained.len(),
        drained.len()
    );
}

// ── Phase 7: Full Pipeline End-to-End ─────────────────────────────────────

#[test]
fn phase7_full_pipeline_e2e() {
    let mut node = TestNode::new();

    // 1. Generate wallet
    let wallet = Wallet::generate();

    // 2. Faucet fund
    node.fund_account(&wallet.address);
    let bal = node.store.get_account(&wallet.address).unwrap().balance;
    assert!(bal >= 10_000);

    // 3. Sign and submit transfer
    let tx = wallet.sign_transfer("recipient_001", 500, 0, 5);
    assert!(verify_tx_signature(&tx, &wallet.provider.public_key()).unwrap());
    assert!(node.mempool.add(tx).unwrap());

    // 4. Produce block
    node.produce_block("validator", 10);

    // 5. Verify balances
    let sender = node.store.get_account(&wallet.address).unwrap();
    assert_eq!(sender.balance, 10_000 - 500 - 5);
    assert_eq!(sender.nonce, 1);

    let recipient = node.store.get_account("recipient_001").unwrap();
    assert_eq!(recipient.balance, 500);

    // 6. Verify economics
    assert_eq!(node.economics.height, 1);
    assert!(node.economics.total_minted > 0);
    verify_supply_invariant(node.economics.total_minted, node.economics.total_burned).unwrap();

    println!("Phase 7: full pipeline E2E — wallet → faucet → sign → submit → block → verify ✓");
}
