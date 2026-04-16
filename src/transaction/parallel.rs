//! Parallel transaction execution — conflict detection and wave scheduling.
//!
//! Given a batch of transactions with their read/write sets, this module:
//! 1. Builds a dependency graph based on key conflicts (WAR, WAW, RAW)
//! 2. Schedules transactions into waves — each wave contains non-conflicting txs
//! 3. Waves execute concurrently; waves are applied sequentially in order
//!
//! Conflict rules:
//! - **RAW (read-after-write)**: TX_b reads a key that TX_a writes → TX_b depends on TX_a
//! - **WAW (write-after-write)**: TX_b writes a key that TX_a writes → TX_b depends on TX_a
//! - **WAR (write-after-read)**: TX_b writes a key that TX_a reads → TX_b depends on TX_a
//! - **RAR (read-after-read)**: no conflict — both can run in parallel

use std::collections::HashSet;

use super::rwset::ReadWriteSet;

/// A transaction with its index in the original batch and its RW set.
#[derive(Debug, Clone)]
pub struct TxWithRwSet {
    /// Index in the original batch (preserves deterministic ordering).
    pub index: usize,
    /// Opaque transaction ID for correlation.
    pub tx_id: String,
    /// The read/write set produced during simulation.
    pub rwset: ReadWriteSet,
}

/// A wave of transactions that can execute concurrently (no mutual conflicts).
#[derive(Debug, Clone)]
pub struct Wave {
    /// Wave number (0-indexed). Wave 0 runs first, then wave 1, etc.
    pub wave_id: usize,
    /// Transaction indices (into the original batch) in this wave.
    pub tx_indices: Vec<usize>,
}

/// Result of conflict analysis: waves + dependency info.
#[derive(Debug, Clone)]
pub struct BatchSchedule {
    /// Ordered waves. Wave 0 has no dependencies. Wave N depends on waves 0..N-1.
    pub waves: Vec<Wave>,
    /// Total transactions in the batch.
    pub total_txs: usize,
    /// Number of waves (1 = fully parallel, N = fully sequential).
    pub wave_count: usize,
    /// Parallelism ratio: total_txs / wave_count (higher = more parallel).
    pub parallelism_ratio: f64,
}

/// Analyze a batch of transactions and produce a parallel execution schedule.
///
/// Algorithm:
/// 1. For each tx, collect its read keys and write keys.
/// 2. Build a dependency graph: tx_i → tx_j if j > i and they conflict on a key.
/// 3. Topological sort into waves using the "longest path" method.
///
/// Determinism: transactions within a wave are ordered by their original batch index.
/// This ensures all validators produce identical results.
pub fn schedule_batch(txs: &[TxWithRwSet]) -> BatchSchedule {
    let n = txs.len();
    if n == 0 {
        return BatchSchedule {
            waves: vec![],
            total_txs: 0,
            wave_count: 0,
            parallelism_ratio: 0.0,
        };
    }

    // 1. Extract key sets per tx.
    let key_sets: Vec<(HashSet<&str>, HashSet<&str>)> = txs
        .iter()
        .map(|tx| {
            let reads: HashSet<&str> = tx.rwset.reads.iter().map(|r| r.key.as_str()).collect();
            let writes: HashSet<&str> = tx.rwset.writes.iter().map(|w| w.key.as_str()).collect();
            (reads, writes)
        })
        .collect();

    // 2. Build dependency graph: deps[j] = set of tx indices that j depends on.
    //    Only look at i < j (earlier txs in batch order).
    let mut deps: Vec<HashSet<usize>> = vec![HashSet::new(); n];

    #[allow(clippy::needless_range_loop)]
    for j in 1..n {
        let (reads_j, writes_j) = &key_sets[j];
        for i in 0..j {
            let (reads_i, writes_i) = &key_sets[i];

            // RAW: j reads what i writes
            let raw = !writes_i.is_disjoint(reads_j);
            // WAW: j writes what i writes
            let waw = !writes_i.is_disjoint(writes_j);
            // WAR: j writes what i reads
            let war = !reads_i.is_disjoint(writes_j);

            if raw || waw || war {
                deps[j].insert(i);
            }
        }
    }

    // 3. Assign each tx to a wave using longest-path in the DAG.
    //    wave[i] = 1 + max(wave[d] for d in deps[i]), or 0 if no deps.
    let mut wave_assignment: Vec<usize> = vec![0; n];
    for i in 0..n {
        if deps[i].is_empty() {
            wave_assignment[i] = 0;
        } else {
            let max_dep_wave = deps[i].iter().map(|&d| wave_assignment[d]).max().unwrap_or(0);
            wave_assignment[i] = max_dep_wave + 1;
        }
    }

    // 4. Group into waves.
    let wave_count = wave_assignment.iter().copied().max().map_or(0, |m| m + 1);
    let mut waves: Vec<Wave> = (0..wave_count)
        .map(|wid| Wave {
            wave_id: wid,
            tx_indices: Vec::new(),
        })
        .collect();

    for (i, &w) in wave_assignment.iter().enumerate() {
        waves[w].tx_indices.push(txs[i].index);
    }

    let parallelism_ratio = if wave_count > 0 {
        n as f64 / wave_count as f64
    } else {
        0.0
    };

    BatchSchedule {
        waves,
        total_txs: n,
        wave_count,
        parallelism_ratio,
    }
}

/// Check if two transactions conflict on any key.
///
/// Public utility for callers that need pairwise conflict checks.
pub fn conflicts(a: &ReadWriteSet, b: &ReadWriteSet) -> bool {
    let a_reads: HashSet<&str> = a.reads.iter().map(|r| r.key.as_str()).collect();
    let a_writes: HashSet<&str> = a.writes.iter().map(|w| w.key.as_str()).collect();
    let b_reads: HashSet<&str> = b.reads.iter().map(|r| r.key.as_str()).collect();
    let b_writes: HashSet<&str> = b.writes.iter().map(|w| w.key.as_str()).collect();

    // RAW (either direction)
    let raw = !a_writes.is_disjoint(&b_reads) || !b_writes.is_disjoint(&a_reads);
    // WAW
    let waw = !a_writes.is_disjoint(&b_writes);

    raw || waw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::rwset::{KVRead, KVWrite};

    fn rw(reads: &[&str], writes: &[&str]) -> ReadWriteSet {
        ReadWriteSet {
            reads: reads
                .iter()
                .map(|k| KVRead {
                    key: k.to_string(),
                    version: 1,
                })
                .collect(),
            writes: writes
                .iter()
                .map(|k| KVWrite {
                    key: k.to_string(),
                    value: vec![1],
                })
                .collect(),
        }
    }

    fn tx(index: usize, reads: &[&str], writes: &[&str]) -> TxWithRwSet {
        TxWithRwSet {
            index,
            tx_id: format!("tx{index}"),
            rwset: rw(reads, writes),
        }
    }

    // --- conflicts() ---

    #[test]
    fn no_conflict_disjoint_keys() {
        let a = rw(&["k1"], &["k2"]);
        let b = rw(&["k3"], &["k4"]);
        assert!(!conflicts(&a, &b));
    }

    #[test]
    fn no_conflict_both_read_same_key() {
        let a = rw(&["k1"], &[]);
        let b = rw(&["k1"], &[]);
        assert!(!conflicts(&a, &b)); // RAR is not a conflict
    }

    #[test]
    fn conflict_raw() {
        let a = rw(&[], &["k1"]);      // a writes k1
        let b = rw(&["k1"], &[]);       // b reads k1
        assert!(conflicts(&a, &b));
    }

    #[test]
    fn conflict_war() {
        let a = rw(&["k1"], &[]);        // a reads k1
        let b = rw(&[], &["k1"]);        // b writes k1
        assert!(conflicts(&a, &b));
    }

    #[test]
    fn conflict_waw() {
        let a = rw(&[], &["k1"]);        // a writes k1
        let b = rw(&[], &["k1"]);        // b writes k1
        assert!(conflicts(&a, &b));
    }

    // --- schedule_batch() --- empty/single ---

    #[test]
    fn empty_batch() {
        let s = schedule_batch(&[]);
        assert_eq!(s.total_txs, 0);
        assert_eq!(s.wave_count, 0);
        assert!(s.waves.is_empty());
    }

    #[test]
    fn single_tx() {
        let batch = vec![tx(0, &["k1"], &["k2"])];
        let s = schedule_batch(&batch);
        assert_eq!(s.total_txs, 1);
        assert_eq!(s.wave_count, 1);
        assert_eq!(s.waves[0].tx_indices, vec![0]);
    }

    // --- fully parallel (no conflicts) ---

    #[test]
    fn fully_parallel_disjoint_keys() {
        // 4 txs touching completely different keys → all in wave 0.
        let batch = vec![
            tx(0, &["a"], &["a1"]),
            tx(1, &["b"], &["b1"]),
            tx(2, &["c"], &["c1"]),
            tx(3, &["d"], &["d1"]),
        ];
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 1, "all txs should be in one wave");
        assert_eq!(s.waves[0].tx_indices, vec![0, 1, 2, 3]);
        assert!((s.parallelism_ratio - 4.0).abs() < f64::EPSILON);
    }

    // --- fully sequential (chain of conflicts) ---

    #[test]
    fn fully_sequential_chain() {
        // tx0 writes k → tx1 reads k and writes k → tx2 reads k
        let batch = vec![
            tx(0, &[], &["k"]),
            tx(1, &["k"], &["k"]),
            tx(2, &["k"], &[]),
        ];
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 3);
        assert_eq!(s.waves[0].tx_indices, vec![0]);
        assert_eq!(s.waves[1].tx_indices, vec![1]);
        assert_eq!(s.waves[2].tx_indices, vec![2]);
        assert!((s.parallelism_ratio - 1.0).abs() < f64::EPSILON);
    }

    // --- diamond dependency ---

    #[test]
    fn diamond_dependency() {
        // tx0 writes k1, k2
        // tx1 reads k1 (depends on tx0)
        // tx2 reads k2 (depends on tx0)
        // tx3 reads k1, k2 (depends on tx1 and tx2, but tx1/tx2 are parallel)
        // Expected: wave0=[tx0], wave1=[tx1,tx2], wave2=[tx3]
        let batch = vec![
            tx(0, &[], &["k1", "k2"]),
            tx(1, &["k1"], &["out1"]),
            tx(2, &["k2"], &["out2"]),
            tx(3, &["k1", "k2"], &[]),
        ];
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 2);
        // tx0 in wave 0
        assert!(s.waves[0].tx_indices.contains(&0));
        // tx1, tx2 depend on tx0 → wave 1. tx3 also depends on tx0 → wave 1.
        // But tx3 doesn't conflict with tx1 or tx2 directly (RAR on k1/k2 is not a conflict...
        // wait, tx1 writes out1 and tx3 reads k1 — no overlap with tx1's writes.
        // tx3 reads k1 which tx0 writes → depends on tx0. Same wave as tx1/tx2.
        // Actually tx3 reads k1 and tx1 reads k1 — RAR, no conflict.
        // tx3 reads k2 and tx2 reads k2 — RAR, no conflict.
        // So tx1, tx2, tx3 are all in wave 1.
        assert!(s.waves[1].tx_indices.contains(&1));
        assert!(s.waves[1].tx_indices.contains(&2));
        assert!(s.waves[1].tx_indices.contains(&3));
    }

    // --- WAW forces sequential ---

    #[test]
    fn waw_creates_dependency() {
        // Both write to "k" → sequential.
        let batch = vec![
            tx(0, &[], &["k"]),
            tx(1, &[], &["k"]),
        ];
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 2);
    }

    // --- mixed parallel and sequential ---

    #[test]
    fn mixed_parallel_sequential() {
        // tx0: writes k1
        // tx1: writes k2 (no conflict with tx0)
        // tx2: reads k1, writes k3 (depends on tx0)
        // tx3: reads k2, writes k4 (depends on tx1)
        // Expected: wave0=[tx0,tx1], wave1=[tx2,tx3]
        let batch = vec![
            tx(0, &[], &["k1"]),
            tx(1, &[], &["k2"]),
            tx(2, &["k1"], &["k3"]),
            tx(3, &["k2"], &["k4"]),
        ];
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 2);
        assert_eq!(s.waves[0].tx_indices, vec![0, 1]);
        assert_eq!(s.waves[1].tx_indices, vec![2, 3]);
        assert!((s.parallelism_ratio - 2.0).abs() < f64::EPSILON);
    }

    // --- stress: 100 independent txs → 1 wave ---

    #[test]
    fn stress_100_independent_txs() {
        let batch: Vec<TxWithRwSet> = (0..100)
            .map(|i| {
                let key = format!("key_{i}");
                TxWithRwSet {
                    index: i,
                    tx_id: format!("tx{i}"),
                    rwset: ReadWriteSet {
                        reads: vec![],
                        writes: vec![KVWrite {
                            key,
                            value: vec![1],
                        }],
                    },
                }
            })
            .collect();
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 1, "100 independent txs → 1 wave");
        assert_eq!(s.waves[0].tx_indices.len(), 100);
    }

    // --- stress: 100 sequential txs on same key → 100 waves ---

    #[test]
    fn stress_100_sequential_txs() {
        let batch: Vec<TxWithRwSet> = (0..100)
            .map(|i| TxWithRwSet {
                index: i,
                tx_id: format!("tx{i}"),
                rwset: ReadWriteSet {
                    reads: vec![],
                    writes: vec![KVWrite {
                        key: "shared".to_string(),
                        value: vec![i as u8],
                    }],
                },
            })
            .collect();
        let s = schedule_batch(&batch);
        assert_eq!(s.wave_count, 100, "100 conflicting txs → 100 waves");
    }

    // --- realistic: DeFi-style mixed workload ---

    #[test]
    fn realistic_defi_workload() {
        // Simulate: 3 token transfers + 2 independent state updates + 1 dependent query.
        // transfer1: reads balance_a, balance_b; writes balance_a, balance_b
        // transfer2: reads balance_c, balance_d; writes balance_c, balance_d (independent)
        // transfer3: reads balance_a, balance_c; writes balance_a, balance_c (conflicts with 1 and 2)
        // update1: writes config_x (independent)
        // update2: writes config_y (independent)
        // query: reads balance_a (depends on transfer1/transfer3)
        let batch = vec![
            TxWithRwSet {
                index: 0,
                tx_id: "transfer1".into(),
                rwset: rw(&["bal_a", "bal_b"], &["bal_a", "bal_b"]),
            },
            TxWithRwSet {
                index: 1,
                tx_id: "transfer2".into(),
                rwset: rw(&["bal_c", "bal_d"], &["bal_c", "bal_d"]),
            },
            TxWithRwSet {
                index: 2,
                tx_id: "transfer3".into(),
                rwset: rw(&["bal_a", "bal_c"], &["bal_a", "bal_c"]),
            },
            TxWithRwSet {
                index: 3,
                tx_id: "update1".into(),
                rwset: rw(&[], &["config_x"]),
            },
            TxWithRwSet {
                index: 4,
                tx_id: "update2".into(),
                rwset: rw(&[], &["config_y"]),
            },
            TxWithRwSet {
                index: 5,
                tx_id: "query".into(),
                rwset: rw(&["bal_a"], &[]),
            },
        ];
        let s = schedule_batch(&batch);

        // transfer1 and transfer2 are independent → wave 0
        // update1 and update2 are independent → wave 0
        // transfer3 conflicts with both transfer1 (bal_a) and transfer2 (bal_c) → wave 1
        // query reads bal_a which transfer1 writes → depends on transfer1 → wave 1
        // But query also depends on transfer3 (bal_a)? No — query only reads bal_a,
        // transfer3 writes bal_a. So query depends on both transfer1 and transfer3.
        // transfer3 is wave 1, so query is wave 2.
        assert!(s.wave_count >= 2, "should have at least 2 waves, got {}", s.wave_count);
        assert!(s.wave_count <= 3, "should have at most 3 waves, got {}", s.wave_count);

        // Parallelism: 6 txs in 2-3 waves → ratio 2.0-3.0
        assert!(
            s.parallelism_ratio >= 2.0,
            "expected good parallelism, got {}",
            s.parallelism_ratio
        );
    }
}
