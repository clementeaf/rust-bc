//! TPS benchmark — measures throughput of the parallel transaction executor
//! under various workload patterns.
//!
//! Not a Criterion benchmark (those are in benches/) — this is a test that
//! asserts minimum throughput thresholds and reports metrics.

use std::sync::Arc;
use std::time::Instant;

use rust_bc::endorsement::types::Endorsement;
use rust_bc::storage::traits::Transaction;
use rust_bc::storage::MemoryWorldState;
use rust_bc::storage::WorldState;
use rust_bc::transaction::endorsed::EndorsedTransaction;
use rust_bc::transaction::executor::{execute_block_concurrent, execute_block_parallel};
use rust_bc::transaction::proposal::TransactionProposal;
use rust_bc::transaction::rwset::{KVRead, KVWrite, ReadWriteSet};

fn make_tx(id: &str) -> Transaction {
    Transaction {
        id: id.to_string(),
        block_height: 1,
        timestamp: 0,
        input_did: "did:test:sender".to_string(),
        output_recipient: "did:test:recv".to_string(),
        amount: 0,
        state: "pending".to_string(),
    }
}

fn endorsed_tx(id: &str, reads: &[(&str, u64)], writes: &[(&str, &[u8])]) -> EndorsedTransaction {
    let rw = ReadWriteSet {
        reads: reads
            .iter()
            .map(|(k, v)| KVRead {
                key: k.to_string(),
                version: *v,
            })
            .collect(),
        writes: writes
            .iter()
            .map(|(k, v)| KVWrite {
                key: k.to_string(),
                value: v.to_vec(),
            })
            .collect(),
    };
    EndorsedTransaction {
        proposal: TransactionProposal {
            tx: make_tx(id),
            creator_did: "did:test:creator".to_string(),
            creator_signature: vec![0u8; 64],
            rwset: rw.clone(),
        },
        endorsements: vec![Endorsement {
            signer_did: "did:test:org1".to_string(),
            org_id: "Org1".to_string(),
            signature: vec![0u8; 64],
            payload_hash: [0u8; 32],
            timestamp: 0,
        }],
        rwset: rw,
    }
}

fn seed_state(state: &dyn WorldState, n: usize) {
    for i in 0..n {
        state.put(&format!("key_{i}"), b"v1").unwrap();
    }
}

// ── Workload generators ─────────────────────────────────────────────────────

/// All txs touch different keys — maximum parallelism.
fn independent_workload(n: usize) -> Vec<EndorsedTransaction> {
    (0..n)
        .map(|i| {
            let key = format!("key_{i}");
            endorsed_tx(&format!("tx_{i}"), &[(&key, 1)], &[(&key, b"v2")])
        })
        .collect()
}

/// All txs touch the same key — fully sequential (worst case).
fn contended_workload(n: usize) -> Vec<EndorsedTransaction> {
    (0..n)
        .map(|i| endorsed_tx(&format!("tx_{i}"), &[("shared", 1)], &[("shared", b"v2")]))
        .collect()
}

/// 80% independent, 20% contended on a "hot" key — realistic DeFi mix.
fn mixed_workload(n: usize) -> Vec<EndorsedTransaction> {
    (0..n)
        .map(|i| {
            if i % 5 == 0 {
                // Hot key.
                endorsed_tx(&format!("tx_{i}"), &[("hot", 1)], &[("hot", b"v2")])
            } else {
                let key = format!("key_{i}");
                endorsed_tx(&format!("tx_{i}"), &[(&key, 1)], &[(&key, b"v2")])
            }
        })
        .collect()
}

struct BenchResult {
    name: String,
    tx_count: usize,
    duration_us: u128,
    tps: f64,
    waves: usize,
    committed: usize,
    conflicts: usize,
}

impl std::fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} txs in {} us = {:.0} TPS | {} waves | {} committed, {} conflicts",
            self.name,
            self.tx_count,
            self.duration_us,
            self.tps,
            self.waves,
            self.committed,
            self.conflicts
        )
    }
}

// ── Synchronous benchmarks ──────────────────────────────────────────────────

#[test]
fn bench_sync_500_independent() {
    let state = MemoryWorldState::new();
    seed_state(&state, 500);
    let txs = independent_workload(500);

    let start = Instant::now();
    let result = execute_block_parallel(&txs, &state);
    let duration = start.elapsed();

    let bench = BenchResult {
        name: "sync_500_independent".into(),
        tx_count: 500,
        duration_us: duration.as_micros(),
        tps: 500.0 / duration.as_secs_f64(),
        waves: result.schedule.wave_count,
        committed: result.committed_count,
        conflicts: result.conflict_count,
    };
    eprintln!("{bench}");

    assert_eq!(result.schedule.wave_count, 1);
    assert_eq!(result.committed_count, 500);
    // Debug mode overhead is ~5-10x; these thresholds are for unoptimized builds.
    // Release mode (`cargo test --release`) reaches 20K+ TPS.
    assert!(
        bench.tps > 1_000.0,
        "expected >1K TPS (debug), got {:.0}",
        bench.tps
    );
}

#[test]
fn bench_sync_500_contended() {
    let state = MemoryWorldState::new();
    state.put("shared", b"v1").unwrap();
    let txs = contended_workload(500);

    let start = Instant::now();
    let result = execute_block_parallel(&txs, &state);
    let duration = start.elapsed();

    let bench = BenchResult {
        name: "sync_500_contended".into(),
        tx_count: 500,
        duration_us: duration.as_micros(),
        tps: 500.0 / duration.as_secs_f64(),
        waves: result.schedule.wave_count,
        committed: result.committed_count,
        conflicts: result.conflict_count,
    };
    eprintln!("{bench}");

    assert_eq!(result.schedule.wave_count, 500);
    assert_eq!(result.committed_count, 1); // Only first tx commits.
}

#[test]
fn bench_sync_1000_mixed() {
    let state = MemoryWorldState::new();
    seed_state(&state, 1000);
    state.put("hot", b"v1").unwrap();
    let txs = mixed_workload(1000);

    let start = Instant::now();
    let result = execute_block_parallel(&txs, &state);
    let duration = start.elapsed();

    let bench = BenchResult {
        name: "sync_1000_mixed".into(),
        tx_count: 1000,
        duration_us: duration.as_micros(),
        tps: 1000.0 / duration.as_secs_f64(),
        waves: result.schedule.wave_count,
        committed: result.committed_count,
        conflicts: result.conflict_count,
    };
    eprintln!("{bench}");

    // Should have some parallelism (not 1000 waves).
    assert!(
        bench.tps > 1_000.0,
        "expected >1K TPS (debug), got {:.0}",
        bench.tps
    );
    assert!(result.committed_count > 100, "expected >100 committed");
}

// ── Concurrent (tokio) benchmarks ───────────────────────────────────────────

#[tokio::test]
async fn bench_concurrent_500_independent() {
    let state = Arc::new(MemoryWorldState::new());
    seed_state(state.as_ref(), 500);
    let txs = independent_workload(500);

    let start = Instant::now();
    let result = execute_block_concurrent(&txs, state).await;
    let duration = start.elapsed();

    let bench = BenchResult {
        name: "concurrent_500_independent".into(),
        tx_count: 500,
        duration_us: duration.as_micros(),
        tps: 500.0 / duration.as_secs_f64(),
        waves: result.schedule.wave_count,
        committed: result.committed_count,
        conflicts: result.conflict_count,
    };
    eprintln!("{bench}");

    assert_eq!(result.committed_count, 500);
    assert_eq!(result.schedule.wave_count, 1);
}

#[tokio::test]
async fn bench_concurrent_1000_mixed() {
    let state = Arc::new(MemoryWorldState::new());
    seed_state(state.as_ref(), 1000);
    state.put("hot", b"v1").unwrap();
    let txs = mixed_workload(1000);

    let start = Instant::now();
    let result = execute_block_concurrent(&txs, state).await;
    let duration = start.elapsed();

    let bench = BenchResult {
        name: "concurrent_1000_mixed".into(),
        tx_count: 1000,
        duration_us: duration.as_micros(),
        tps: 1000.0 / duration.as_secs_f64(),
        waves: result.schedule.wave_count,
        committed: result.committed_count,
        conflicts: result.conflict_count,
    };
    eprintln!("{bench}");

    assert!(result.committed_count > 100);
}

// ── Scaling test: sync vs concurrent should produce identical results ────────

#[tokio::test]
async fn bench_sync_vs_concurrent_parity() {
    let state_sync = MemoryWorldState::new();
    let state_async = Arc::new(MemoryWorldState::new());

    seed_state(&state_sync, 200);
    seed_state(state_async.as_ref(), 200);
    state_sync.put("hot", b"v1").unwrap();
    state_async.put("hot", b"v1").unwrap();

    let txs = mixed_workload(200);

    let sync_result = execute_block_parallel(&txs, &state_sync);
    let async_result = execute_block_concurrent(&txs, state_async).await;

    assert_eq!(sync_result.committed_count, async_result.committed_count);
    assert_eq!(sync_result.conflict_count, async_result.conflict_count);
    assert_eq!(
        sync_result.schedule.wave_count,
        async_result.schedule.wave_count
    );

    for (i, (s, a)) in sync_result
        .outcomes
        .iter()
        .zip(async_result.outcomes.iter())
        .enumerate()
    {
        assert_eq!(s.1, a.1, "outcome mismatch at tx {i}");
    }
}
