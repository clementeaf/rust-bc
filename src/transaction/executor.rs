//! Parallel block executor — applies transactions in wave-parallel order.
//!
//! Uses the [`BatchSchedule`] from `parallel.rs` to execute non-conflicting
//! transactions concurrently within each wave while preserving deterministic
//! ordering across waves.
//!
//! Execution flow:
//! 1. Schedule the batch into waves (via `schedule_batch`)
//! 2. For each wave (sequentially):
//!    a. MVCC-validate all txs in the wave against current world state
//!    b. Apply writes from valid txs (within a wave, order by original index)
//! 3. Return per-tx results: committed or mvcc_conflict
//!
//! Determinism guarantee: transactions within a wave are always applied in
//! ascending index order, so all validators produce identical state transitions.

use std::sync::Arc;

use super::endorsed::EndorsedTransaction;
use super::mvcc;
use super::parallel::{schedule_batch, BatchSchedule, TxWithRwSet};
use super::rwset::ReadWriteSet;
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

/// Execute a block of endorsed transactions using wave-parallel scheduling.
///
/// Transactions are grouped into waves by conflict analysis. Within each wave,
/// txs are independent and could run concurrently (the MVCC check + write
/// application is done per-wave sequentially for determinism).
///
/// Between waves, writes from the previous wave are visible to the next,
/// enabling dependent transactions to validate correctly.
pub fn execute_block_parallel(
    txs: &[EndorsedTransaction],
    state: &dyn WorldState,
) -> BlockExecResult {
    // 1. Build TxWithRwSet entries for the scheduler.
    let batch: Vec<TxWithRwSet> = txs
        .iter()
        .enumerate()
        .map(|(i, endorsed)| TxWithRwSet {
            index: i,
            tx_id: endorsed.proposal.tx.id.clone(),
            rwset: endorsed.rwset.clone(),
        })
        .collect();

    // 2. Schedule into waves.
    let schedule = schedule_batch(&batch);

    // 3. Execute wave by wave.
    let mut outcomes: Vec<Option<(String, TxOutcome)>> = vec![None; txs.len()];
    let mut committed_count = 0usize;
    let mut conflict_count = 0usize;

    for wave in &schedule.waves {
        // Within a wave, process txs in ascending index order (deterministic).
        let mut sorted_indices = wave.tx_indices.clone();
        sorted_indices.sort_unstable();

        for &idx in &sorted_indices {
            let endorsed = &txs[idx];

            match mvcc::validate_rwset(&endorsed.rwset, state) {
                Ok(()) => {
                    // Apply writes to world state.
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
                        TxOutcome::MvccConflict {
                            key: conflict.key,
                        },
                    ));
                    conflict_count += 1;
                }
            }
        }
    }

    // Fill any unscheduled txs (shouldn't happen, but defensive).
    for (i, slot) in outcomes.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some((
                txs[i].proposal.tx.id.clone(),
                TxOutcome::MvccConflict {
                    key: "unscheduled".into(),
                },
            ));
            conflict_count += 1;
        }
    }

    BlockExecResult {
        outcomes: outcomes.into_iter().map(|o| o.unwrap()).collect(),
        schedule,
        committed_count,
        conflict_count,
    }
}

/// Convert a `BlockExecResult` to the legacy `Vec<Transaction>` format
/// with `state = "committed"` or `state = "mvcc_conflict"`.
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
    use crate::transaction::rwset::{KVRead, KVWrite};

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

    // --- basic execution ---

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
        state.put("k", b"v1").unwrap(); // v1
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
        state.put("k", b"v2").unwrap(); // v2
        let txs = vec![endorsed("tx1", &[("k", 1)], &[("k", b"v3")])];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.conflict_count, 1);
        assert!(matches!(result.outcomes[0].1, TxOutcome::MvccConflict { .. }));
        assert_eq!(state.get("k").unwrap().unwrap().data, b"v2"); // unchanged
    }

    // --- parallel execution (independent txs in same wave) ---

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
        assert_eq!(result.schedule.wave_count, 1, "should be 1 wave");
        assert_eq!(result.committed_count, 2);
        assert_eq!(state.get("a").unwrap().unwrap().data, b"a2");
        assert_eq!(state.get("b").unwrap().unwrap().data, b"b2");
    }

    // --- dependent txs across waves ---

    #[test]
    fn dependent_txs_execute_in_separate_waves() {
        let state = ws();
        state.put("k", b"v1").unwrap();

        // tx1 writes k (v1→v2), tx2 reads k at v2.
        // But tx2 was simulated against v1 — so it reads v1.
        // After tx1 commits (k now v2), tx2 reads at v1 → MVCC conflict.
        // This is correct Fabric behavior: tx2 was simulated against stale state.
        let txs = vec![
            endorsed("tx1", &[("k", 1)], &[("k", b"v2")]),
            endorsed("tx2", &[("k", 1)], &[("k", b"v3")]),
        ];

        let result = execute_block_parallel(&txs, &state);
        assert_eq!(result.schedule.wave_count, 2, "WAW → 2 waves");
        assert_eq!(result.committed_count, 1);
        assert_eq!(result.conflict_count, 1);
        assert_eq!(result.outcomes[0].1, TxOutcome::Committed);
        assert!(matches!(result.outcomes[1].1, TxOutcome::MvccConflict { .. }));
    }

    // --- legacy format ---

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

    // --- parallelism metrics ---

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

    // --- stress: 50 independent txs ---

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
        assert_eq!(result.conflict_count, 0);
    }

    // --- mixed: some parallel, some sequential ---

    #[test]
    fn mixed_workload_correct_outcomes() {
        let state = ws();
        state.put("shared", b"v1").unwrap();
        state.put("indep_a", b"v1").unwrap();
        state.put("indep_b", b"v1").unwrap();

        let txs = vec![
            // wave 0: tx0 (indep_a) and tx1 (indep_b) are independent
            endorsed("tx0", &[("indep_a", 1)], &[("indep_a", b"a2")]),
            endorsed("tx1", &[("indep_b", 1)], &[("indep_b", b"b2")]),
            // wave 0 also: tx2 writes shared
            endorsed("tx2", &[("shared", 1)], &[("shared", b"s2")]),
            // wave 1: tx3 reads shared (depends on tx2)
            endorsed("tx3", &[("shared", 1)], &[("shared", b"s3")]),
        ];

        let result = execute_block_parallel(&txs, &state);

        // tx0, tx1, tx2 should commit (wave 0)
        assert_eq!(result.outcomes[0].1, TxOutcome::Committed);
        assert_eq!(result.outcomes[1].1, TxOutcome::Committed);
        assert_eq!(result.outcomes[2].1, TxOutcome::Committed);

        // tx3 read shared at v1 but tx2 bumped it to v2 → conflict
        assert!(matches!(result.outcomes[3].1, TxOutcome::MvccConflict { .. }));

        assert_eq!(result.committed_count, 3);
        assert_eq!(result.conflict_count, 1);
    }
}
