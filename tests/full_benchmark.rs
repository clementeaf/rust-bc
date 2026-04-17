//! Full-stack benchmark — measures TPS across the complete pipeline:
//! BFT consensus + parallel execution + MVCC + world state writes.
//!
//! Run with: `cargo test --release --test full_benchmark -- --nocapture`

use std::sync::Arc;
use std::time::Instant;

use rust_bc::consensus::bft::quorum::{QuorumValidator, SignatureVerifier};
use rust_bc::consensus::bft::round::{BftRound, RoundEvent, RoundState};
use rust_bc::consensus::bft::types::{BftPhase, VoteMessage};
use rust_bc::consensus::dpos::{select_committee, DposConfig, ValidatorStake};
use rust_bc::endorsement::types::Endorsement;
use rust_bc::storage::traits::Transaction;
use rust_bc::storage::MemoryWorldState;
use rust_bc::storage::WorldState;
use rust_bc::transaction::endorsed::EndorsedTransaction;
use rust_bc::transaction::executor::{execute_block_concurrent, execute_block_parallel};
use rust_bc::transaction::proposal::TransactionProposal;
use rust_bc::transaction::rwset::{KVRead, KVWrite, ReadWriteSet};

#[derive(Clone)]
struct BenchVerifier;
impl SignatureVerifier for BenchVerifier {
    fn verify(&self, _: &str, _: &[u8], sig: &[u8]) -> bool {
        !sig.is_empty()
    }
}

fn make_endorsed(id: &str, key: &str, version: u64) -> EndorsedTransaction {
    let rw = ReadWriteSet {
        reads: vec![KVRead {
            key: key.into(),
            version,
        }],
        writes: vec![KVWrite {
            key: key.into(),
            value: vec![1u8; 32],
        }],
    };
    EndorsedTransaction {
        proposal: TransactionProposal {
            tx: Transaction {
                id: id.into(),
                block_height: 0,
                timestamp: 0,
                input_did: "did:bc:sender".into(),
                output_recipient: "did:bc:recv".into(),
                amount: 1,
                state: "pending".into(),
            },
            creator_did: "did:bc:creator".into(),
            creator_signature: vec![0u8; 64],
            rwset: rw.clone(),
        },
        endorsements: vec![Endorsement {
            signer_did: "did:bc:org1".into(),
            org_id: "Org1".into(),
            signature: vec![1u8; 64],
            payload_hash: [0u8; 32],
            timestamp: 0,
        }],
        rwset: rw,
    }
}

fn block_hash(round: u64) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[..8].copy_from_slice(&round.to_le_bytes());
    h
}

fn make_vote(phase: BftPhase, bh: [u8; 32], round: u64, voter: &str) -> VoteMessage {
    VoteMessage {
        block_hash: bh,
        round,
        phase,
        voter_id: voter.into(),
        signature: vec![1u8; 64],
    }
}

// ── Benchmark 1: Pure execution throughput (release mode) ───────────────────

#[test]
fn bench_release_1000_independent_sync() {
    let state = MemoryWorldState::new();
    let txs: Vec<EndorsedTransaction> = (0..1000)
        .map(|i| {
            let key = format!("k_{i}");
            state.put(&key, b"v1").unwrap();
            make_endorsed(&format!("tx_{i}"), &key, 1)
        })
        .collect();

    let start = Instant::now();
    let result = execute_block_parallel(&txs, &state);
    let duration = start.elapsed();

    let tps = 1000.0 / duration.as_secs_f64();
    eprintln!(
        "RELEASE sync 1000 independent: {:.0} TPS | {} waves | {:.2} ms",
        tps,
        result.schedule.wave_count,
        duration.as_secs_f64() * 1000.0
    );

    assert_eq!(result.committed_count, 1000);
    assert_eq!(result.schedule.wave_count, 1);
}

#[tokio::test]
async fn bench_release_1000_independent_concurrent() {
    let state = Arc::new(MemoryWorldState::new());
    let txs: Vec<EndorsedTransaction> = (0..1000)
        .map(|i| {
            let key = format!("k_{i}");
            state.put(&key, b"v1").unwrap();
            make_endorsed(&format!("tx_{i}"), &key, 1)
        })
        .collect();

    let start = Instant::now();
    let result = execute_block_concurrent(&txs, state).await;
    let duration = start.elapsed();

    let tps = 1000.0 / duration.as_secs_f64();
    eprintln!(
        "RELEASE concurrent 1000 independent: {:.0} TPS | {} waves | {:.2} ms",
        tps,
        result.schedule.wave_count,
        duration.as_secs_f64() * 1000.0
    );

    assert_eq!(result.committed_count, 1000);
}

// ── Benchmark 2: BFT consensus rounds throughput ────────────────────────────

#[test]
fn bench_bft_100_rounds() {
    let validators: Vec<String> = (0..4).map(|i| format!("v{i}")).collect();

    let start = Instant::now();
    let mut decided = 0u64;

    for round in 0..100 {
        let bh = block_hash(round);
        let leader = format!("v{}", round % 4);
        let mut r = BftRound::new(
            round,
            leader.clone(),
            leader.clone(),
            validators.clone(),
            BenchVerifier,
        );

        r.process(RoundEvent::StartAsLeader { block_hash: bh });

        for phase in [BftPhase::Prepare, BftPhase::PreCommit, BftPhase::Commit] {
            for v in &["v0", "v1", "v2"] {
                r.process(RoundEvent::Vote(make_vote(phase, bh, round, v)));
            }
        }

        if r.state() == RoundState::Decided {
            decided += 1;
        }
    }

    let duration = start.elapsed();
    let rounds_per_sec = 100.0 / duration.as_secs_f64();
    eprintln!(
        "BFT 100 rounds: {:.0} rounds/sec | {:.2} ms total | {decided}/100 decided",
        rounds_per_sec,
        duration.as_secs_f64() * 1000.0
    );

    assert_eq!(decided, 100);
}

// ── Benchmark 3: DPoS committee selection ───────────────────────────────────

#[test]
fn bench_dpos_1000_candidates() {
    let candidates: Vec<ValidatorStake> = (0..1000)
        .map(|i| ValidatorStake {
            address: format!("v{i:04}"),
            stake: 1000 + i * 10,
            active: true,
        })
        .collect();
    let config = DposConfig {
        max_validators: 150,
        min_stake: 1000,
    };

    let start = Instant::now();
    let committee = select_committee(&candidates, &config, 0);
    let duration = start.elapsed();

    eprintln!(
        "DPoS select 150 from 1000: {:.2} ms | total_stake: {}",
        duration.as_secs_f64() * 1000.0,
        committee.total_stake
    );

    assert_eq!(committee.size(), 150);
}

// ── Benchmark 4: Full pipeline (BFT + execution + state) ────────────────────

#[test]
fn bench_full_pipeline_10_blocks() {
    let state = MemoryWorldState::new();
    let validators: Vec<String> = (0..4).map(|i| format!("v{i}")).collect();

    let mut total_txs = 0usize;
    let start = Instant::now();

    for block_num in 0..10u64 {
        // 1. BFT round.
        let bh = block_hash(block_num);
        let leader = format!("v{}", block_num % 4);
        let mut r = BftRound::new(
            block_num,
            leader.clone(),
            leader,
            validators.clone(),
            BenchVerifier,
        );
        r.process(RoundEvent::StartAsLeader { block_hash: bh });
        for phase in [BftPhase::Prepare, BftPhase::PreCommit, BftPhase::Commit] {
            for v in &["v0", "v1", "v2"] {
                r.process(RoundEvent::Vote(make_vote(phase, bh, block_num, v)));
            }
        }
        assert_eq!(r.state(), RoundState::Decided);

        // 2. Build 100 txs for this block.
        let txs: Vec<EndorsedTransaction> = (0..100)
            .map(|i| {
                let key = format!("b{block_num}_k{i}");
                state.put(&key, b"v1").unwrap();
                make_endorsed(&format!("b{block_num}_tx{i}"), &key, 1)
            })
            .collect();

        // 3. Execute in parallel.
        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.committed_count, 100);
        total_txs += 100;
    }

    let duration = start.elapsed();
    let tps = total_txs as f64 / duration.as_secs_f64();
    let blocks_per_sec = 10.0 / duration.as_secs_f64();

    eprintln!(
        "FULL PIPELINE 10 blocks x 100 txs: {:.0} TPS | {:.0} blocks/sec | {:.2} ms total",
        tps,
        blocks_per_sec,
        duration.as_secs_f64() * 1000.0
    );

    assert!(
        tps > 1000.0,
        "expected >1K TPS in full pipeline, got {tps:.0}"
    );
}
