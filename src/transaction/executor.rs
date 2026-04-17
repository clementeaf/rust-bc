//! Parallel block executor — applies transactions in wave-parallel order.
//!
//! Uses the [`BatchSchedule`] from `parallel.rs` to execute non-conflicting
//! transactions concurrently within each wave while preserving deterministic
//! ordering across waves.
//!
//! Execution modes:
//! - `execute_block_parallel`: synchronous, validates within waves sequentially
//! - `execute_block_concurrent`: async, validates within waves with tokio tasks
//!
//! Both modes guarantee determinism: writes are applied in ascending index order
//! within each wave, and waves are processed sequentially.

use std::sync::Arc;

use super::endorsed::EndorsedTransaction;
use super::mvcc;
use super::parallel::{schedule_batch, BatchSchedule, TxWithRwSet};
use crate::storage::traits::Transaction;
use crate::storage::WorldState;

/// Result of executing a single transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxOutcome {
    Committed,
    MvccConflict { key: String },
}

/// Result of executing a full block in wave-parallel mode.
#[derive(Debug, Clone)]
pub struct BlockExecResult {
    /// Per-transaction outcomes, indexed by original batch position.
    pub outcomes: Vec<(String, TxOutcome)>,
    /// The schedule that was used (for metrics/logging).
    pub schedule: BatchSchedule,
    /// Number of transactions committed.
    pub committed_count: usize,
    /// Number of transactions rejected due to MVCC conflicts.
    pub conflict_count: usize,
}

/// MVCC validation result for a single tx (used internally).
#[derive(Debug)]
enum ValidationResult {
    Valid(usize), // index
    Conflict(usize, String), // index, conflict key
}

// ── Synchronous executor ────────────────────────────────────────────────────

/// Execute a block of endorsed transactions using wave-parallel scheduling.
///
/// Within each wave, txs are validated and applied sequentially (but waves
/// themselves are independent). Use `execute_block_concurrent` for true
/// intra-wave parallelism.
pub fn execute_block_parallel(
    txs: &[EndorsedTransaction],
    state: &dyn WorldState,
) -> BlockExecResult {
    let (schedule, _batch) = prepare_schedule(txs);
    let mut outcomes: Vec<Option<(String, TxOutcome)>> = vec![None; txs.len()];
    let mut committed_count = 0usize;
    let mut conflict_count = 0usize;

    for wave in &schedule.waves {
        let mut sorted_indices = wave.tx_indices.clone();
        sorted_indices.sort_unstable();

        for &idx in &sorted_indices {
            let endorsed = &txs[idx];
            match mvcc::validate_rwset(&endorsed.rwset, state) {
                Ok(()) => {
                    for write in &endorsed.rwset.writes {
                        let _ = state.put(&write.key, &write.value);
                    }
                    outcomes[idx] = Some((
                        endorsed.proposal.tx.id.clone(),
                        TxOutcome::Committed,
                    ));
                    committed_count += 1;
                }
                Err(conflict) => {
                    outcomes[idx] = Some((
                        endorsed.proposal.tx.id.clone(),
                        TxOutcome::MvccConflict { key: conflict.key },
                    ));
                    conflict_count += 1;
                }
            }
        }
    }

    fill_unscheduled(&mut outcomes, txs, &mut conflict_count);

    BlockExecResult {
        outcomes: outcomes.into_iter().map(|o| o.unwrap()).collect(),
        schedule,
        committed_count,
        conflict_count,
    }
}

// ── Concurrent executor (tokio) ─────────────────────────────────────────────

/// Execute a block with true intra-wave concurrency using tokio tasks.
///
/// For each wave:
/// 1. Spawn one task per tx to MVCC-validate concurrently (read-only)
/// 2. Collect results
/// 3. Apply writes from valid txs in deterministic order (sequential)
///
/// This maximizes throughput for waves with many independent txs while
/// preserving determinism in the write phase.
pub async fn execute_block_concurrent(
    txs: &[EndorsedTransaction],
    state: Arc<dyn WorldState>,
) -> BlockExecResult {
    let (schedule, _batch) = prepare_schedule(txs);
    let mut outcomes: Vec<Option<(String, TxOutcome)>> = vec![None; txs.len()];
    let mut committed_count = 0usize;
    let mut conflict_count = 0usize;

    for wave in &schedule.waves {
        let mut sorted_indices = wave.tx_indices.clone();
        sorted_indices.sort_unstable();

        // Phase 1: Validate all txs in the wave concurrently.
        let mut handles = Vec::with_capacity(sorted_indices.len());

        for &idx in &sorted_indices {
            let rwset = txs[idx].rwset.clone();
            let ws = Arc::clone(&state);

            handles.push(tokio::task::spawn_blocking(move || {
                match mvcc::validate_rwset(&rwset, ws.as_ref()) {
                    Ok(()) => ValidationResult::Valid(idx),
                    Err(conflict) => ValidationResult::Conflict(idx, conflict.key),
                }
            }));
        }

        // Collect all validation results.
        let mut valid_indices: Vec<usize> = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(ValidationResult::Valid(idx)) => {
                    valid_indices.push(idx);
                }
                Ok(ValidationResult::Conflict(idx, key)) => {
                    outcomes[idx] = Some((
                        txs[idx].proposal.tx.id.clone(),
                        TxOutcome::MvccConflict { key },
                    ));
                    conflict_count += 1;
                }
                Err(e) => {
                    // JoinError — treat as conflict.
                    log::error!("task join error during MVCC validation: {e}");
                }
            }
        }

        // Phase 2: Apply writes from valid txs in deterministic order.
        valid_indices.sort_unstable();
        for idx in valid_indices {
            let endorsed = &txs[idx];
            for write in &endorsed.rwset.writes {
                let _ = state.put(&write.key, &write.value);
            }
            outcomes[idx] = Some((
                endorsed.proposal.tx.id.clone(),
                TxOutcome::Committed,
            ));
            committed_count += 1;
        }
    }

    fill_unscheduled(&mut outcomes, txs, &mut conflict_count);

    BlockExecResult {
        outcomes: outcomes.into_iter().map(|o| o.unwrap()).collect(),
        schedule,
        committed_count,
        conflict_count,
    }
}

// ── Shared helpers ──────────────────────────────────────────────────────────

fn prepare_schedule(txs: &[EndorsedTransaction]) -> (BatchSchedule, Vec<TxWithRwSet>) {
    let batch: Vec<TxWithRwSet> = txs
        .iter()
        .enumerate()
        .map(|(i, endorsed)| TxWithRwSet {
            index: i,
            tx_id: endorsed.proposal.tx.id.clone(),
            rwset: endorsed.rwset.clone(),
        })
        .collect();
    let schedule = schedule_batch(&batch);
    (schedule, batch)
}

fn fill_unscheduled(
    outcomes: &mut [Option<(String, TxOutcome)>],
    txs: &[EndorsedTransaction],
    conflict_count: &mut usize,
) {
    for (i, slot) in outcomes.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some((
                txs[i].proposal.tx.id.clone(),
                TxOutcome::MvccConflict {
                    key: "unscheduled".into(),
                },
            ));
            *conflict_count += 1;
        }
    }
}

/// Convert a `BlockExecResult` to the legacy `Vec<Transaction>` format.
pub fn to_legacy_results(
    txs: &[EndorsedTransaction],
    result: &BlockExecResult,
) -> Vec<Transaction> {
    result
        .outcomes
        .iter()
        .enumerate()
        .map(|(i, (_, outcome))| {
            let mut tx = txs[i].proposal.tx.clone();
            tx.state = match outcome {
                TxOutcome::Committed => "committed".to_string(),
                TxOutcome::MvccConflict { .. } => "mvcc_conflict".to_string(),
            };
            tx
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::types::Endorsement;
    use crate::storage::MemoryWorldState;
    use crate::transaction::proposal::TransactionProposal;
    use crate::transaction::rwset::{KVRead, KVWrite, ReadWriteSet};

    fn ws() -> MemoryWorldState {
        MemoryWorldState::new()
    }

    fn base_tx(id: &str) -> Transaction {
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

    fn endorsed(
        id: &str,
        reads: &[(&str, u64)],
        writes: &[(&str, &[u8])],
    ) -> EndorsedTransaction {
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
                tx: base_tx(id),
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

    // --- synchronous executor ---

    #[test]
    fn empty_block() {
        let state = ws();
        let result = execute_block_parallel(&[], &state);
        assert_eq!(result.committed_count, 0);
        assert_eq!(result.conflict_count, 0);
        assert_eq!(result.schedule.wave_count, 0);
    }

    #[test]
    fn single_tx_commits() {
        let state = ws();
        state.put("k", b"v1").unwrap();
        let txs = vec![endorsed("tx1", &[("k", 1)], &[("k", b"v2")])];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.committed_count, 1);
        assert_eq!(result.outcomes[0].1, TxOutcome::Committed);
        assert_eq!(state.get("k").unwrap().unwrap().data, b"v2");
    }

    #[test]
    fn single_tx_conflicts() {
        let state = ws();
        state.put("k", b"v1").unwrap();
        state.put("k", b"v2").unwrap();
        let txs = vec![endorsed("tx1", &[("k", 1)], &[("k", b"v3")])];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.conflict_count, 1);
        assert!(matches!(result.outcomes[0].1, TxOutcome::MvccConflict { .. }));
        assert_eq!(state.get("k").unwrap().unwrap().data, b"v2");
    }

    #[test]
    fn independent_txs_execute_in_one_wave() {
        let state = ws();
        state.put("a", b"v1").unwrap();
        state.put("b", b"v1").unwrap();

        let txs = vec![
            endorsed("tx1", &[("a", 1)], &[("a", b"a2")]),
            endorsed("tx2", &[("b", 1)], &[("b", b"b2")]),
        ];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.schedule.wave_count, 1);
        assert_eq!(result.committed_count, 2);
    }

    #[test]
    fn dependent_txs_execute_in_separate_waves() {
        let state = ws();
        state.put("k", b"v1").unwrap();

        let txs = vec![
            endorsed("tx1", &[("k", 1)], &[("k", b"v2")]),
            endorsed("tx2", &[("k", 1)], &[("k", b"v3")]),
        ];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.schedule.wave_count, 2);
        assert_eq!(result.committed_count, 1);
        assert_eq!(result.conflict_count, 1);
    }

    #[test]
    fn to_legacy_results_format() {
        let state = ws();
        state.put("k", b"v1").unwrap();
        let txs = vec![
            endorsed("tx1", &[("k", 1)], &[("k", b"v2")]),
            endorsed("tx2", &[("k", 1)], &[("k", b"v3")]),
        ];

        let result = execute_block_parallel(&txs, &state);
        let legacy = to_legacy_results(&txs, &result);

        assert_eq!(legacy[0].id, "tx1");
        assert_eq!(legacy[0].state, "committed");
        assert_eq!(legacy[1].id, "tx2");
        assert_eq!(legacy[1].state, "mvcc_conflict");
    }

    #[test]
    fn parallelism_ratio_reported() {
        let state = ws();
        for i in 0..4 {
            state.put(&format!("k{i}"), b"v1").unwrap();
        }

        let txs: Vec<EndorsedTransaction> = (0..4)
            .map(|i| {
                let key = format!("k{i}");
                endorsed(&format!("tx{i}"), &[(&key, 1)], &[(&key, b"v2")])
            })
            .collect();

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.schedule.wave_count, 1);
        assert!((result.schedule.parallelism_ratio - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn stress_50_independent_txs() {
        let state = ws();
        let mut txs = Vec::new();

        for i in 0..50 {
            let key = format!("key_{i}");
            state.put(&key, b"v1").unwrap();
            txs.push(endorsed(
                &format!("tx{i}"),
                &[(&key, 1)],
                &[(&key, b"v2")],
            ));
        }

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.schedule.wave_count, 1);
        assert_eq!(result.committed_count, 50);
    }

    #[test]
    fn mixed_workload_correct_outcomes() {
        let state = ws();
        state.put("shared", b"v1").unwrap();
        state.put("indep_a", b"v1").unwrap();
        state.put("indep_b", b"v1").unwrap();

        let txs = vec![
            endorsed("tx0", &[("indep_a", 1)], &[("indep_a", b"a2")]),
            endorsed("tx1", &[("indep_b", 1)], &[("indep_b", b"b2")]),
            endorsed("tx2", &[("shared", 1)], &[("shared", b"s2")]),
            endorsed("tx3", &[("shared", 1)], &[("shared", b"s3")]),
        ];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.outcomes[0].1, TxOutcome::Committed);
        assert_eq!(result.outcomes[1].1, TxOutcome::Committed);
        assert_eq!(result.outcomes[2].1, TxOutcome::Committed);
        assert!(matches!(result.outcomes[3].1, TxOutcome::MvccConflict { .. }));
        assert_eq!(result.committed_count, 3);
    }

    // --- concurrent executor (tokio) ---

    #[tokio::test]
    async fn concurrent_empty_block() {
        let state = Arc::new(ws());
        let result = execute_block_concurrent(&[], state).await;
        assert_eq!(result.committed_count, 0);
        assert_eq!(result.schedule.wave_count, 0);
    }

    #[tokio::test]
    async fn concurrent_single_tx_commits() {
        let state = Arc::new(ws());
        state.put("k", b"v1").unwrap();
        let txs = vec![endorsed("tx1", &[("k", 1)], &[("k", b"v2")])];

        let result = execute_block_concurrent(&txs, state.clone()).await;
        assert_eq!(result.committed_count, 1);
        assert_eq!(state.get("k").unwrap().unwrap().data, b"v2");
    }

    #[tokio::test]
    async fn concurrent_independent_txs_one_wave() {
        let state = Arc::new(ws());
        state.put("a", b"v1").unwrap();
        state.put("b", b"v1").unwrap();

        let txs = vec![
            endorsed("tx1", &[("a", 1)], &[("a", b"a2")]),
            endorsed("tx2", &[("b", 1)], &[("b", b"b2")]),
        ];

        let result = execute_block_concurrent(&txs, state.clone()).await;
        assert_eq!(result.schedule.wave_count, 1);
        assert_eq!(result.committed_count, 2);
        assert_eq!(state.get("a").unwrap().unwrap().data, b"a2");
        assert_eq!(state.get("b").unwrap().unwrap().data, b"b2");
    }

    #[tokio::test]
    async fn concurrent_conflicting_txs() {
        let state = Arc::new(ws());
        state.put("k", b"v1").unwrap();

        let txs = vec![
            endorsed("tx1", &[("k", 1)], &[("k", b"v2")]),
            endorsed("tx2", &[("k", 1)], &[("k", b"v3")]),
        ];

        let result = execute_block_concurrent(&txs, state).await;
        assert_eq!(result.committed_count, 1);
        assert_eq!(result.conflict_count, 1);
    }

    #[tokio::test]
    async fn concurrent_stress_100_independent() {
        let state = Arc::new(ws());
        let mut txs = Vec::new();

        for i in 0..100 {
            let key = format!("key_{i}");
            state.put(&key, b"v1").unwrap();
            txs.push(endorsed(
                &format!("tx{i}"),
                &[(&key, 1)],
                &[(&key, b"v2")],
            ));
        }

        let result = execute_block_concurrent(&txs, state).await;
        assert_eq!(result.schedule.wave_count, 1);
        assert_eq!(result.committed_count, 100);
        assert_eq!(result.conflict_count, 0);
    }

    #[tokio::test]
    async fn concurrent_matches_sync_results() {
        // Same workload through both executors should produce identical outcomes.
        let state_sync = ws();
        let state_async = Arc::new(ws());

        // Seed both with identical state.
        for s in [&state_sync as &dyn WorldState, state_async.as_ref()] {
            s.put("a", b"v1").unwrap();
            s.put("b", b"v1").unwrap();
            s.put("shared", b"v1").unwrap();
        }

        let txs = vec![
            endorsed("tx0", &[("a", 1)], &[("a", b"a2")]),
            endorsed("tx1", &[("b", 1)], &[("b", b"b2")]),
            endorsed("tx2", &[("shared", 1)], &[("shared", b"s2")]),
            endorsed("tx3", &[("shared", 1)], &[("shared", b"s3")]),
        ];

        let sync_result = execute_block_parallel(&txs, &state_sync);
        let async_result = execute_block_concurrent(&txs, state_async).await;

        assert_eq!(sync_result.committed_count, async_result.committed_count);
        assert_eq!(sync_result.conflict_count, async_result.conflict_count);
        assert_eq!(sync_result.schedule.wave_count, async_result.schedule.wave_count);

        // Per-tx outcomes must match.
        for (i, (s, a)) in sync_result.outcomes.iter().zip(async_result.outcomes.iter()).enumerate() {
            assert_eq!(s.1, a.1, "outcome mismatch at tx {i}");
        }
    }
}
