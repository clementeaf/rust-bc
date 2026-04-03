//! MVCC (Multi-Version Concurrency Control) validation for transactions.
//!
//! Before a transaction can be committed its read-set is checked against the
//! current world state.  If any key was modified after the transaction read it
//! (i.e. the committed version differs from the read version) the transaction
//! is rejected with an [`MvccConflict`].
//!
//! This mirrors Hyperledger Fabric's MVCC check that happens during block
//! validation in the committer peer.

use crate::storage::{traits::Transaction, WorldState};

use super::endorsed::EndorsedTransaction;
use super::rwset::ReadWriteSet;

/// Conflict detected during MVCC validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvccConflict {
    /// The key that caused the conflict.
    pub key: String,
    /// Version the transaction read at simulation time.
    pub read_version: u64,
    /// Current committed version at validation time.
    pub current_version: u64,
}

impl std::fmt::Display for MvccConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MVCC conflict on key '{}': read v{} but current is v{}",
            self.key, self.read_version, self.current_version
        )
    }
}

/// Validate a transaction's read-write set against the current world state.
///
/// For every [`KVRead`] in `rwset.reads`:
/// - If the key is absent in state and `read.version == 0` → no conflict (phantom read is ok).
/// - If the key is absent in state and `read.version != 0` → conflict (key was deleted).
/// - If the key is present and versions match → no conflict.
/// - If the key is present and versions differ → conflict.
///
/// Returns the first conflict found, or `Ok(())` if all reads are valid.
///
/// [`KVRead`]: crate::transaction::KVRead
pub fn validate_rwset(
    rwset: &ReadWriteSet,
    state: &dyn WorldState,
) -> Result<(), MvccConflict> {
    for read in &rwset.reads {
        let current_version = match state
            .get(&read.key)
            .map_err(|_| MvccConflict {
                key: read.key.clone(),
                read_version: read.version,
                current_version: 0,
            })? {
            Some(vv) => vv.version,
            None => 0,
        };

        if read.version != current_version {
            return Err(MvccConflict {
                key: read.key.clone(),
                read_version: read.version,
                current_version,
            });
        }
    }
    Ok(())
}

/// Apply a block's endorsed transactions against the world state using MVCC.
///
/// For each transaction:
/// - If its read-set is valid (no version conflicts) the writes are applied to
///   `state` and the transaction is returned with `state = "committed"`.
/// - If there is a conflict the writes are **skipped** and the transaction is
///   returned with `state = "mvcc_conflict"`.
///
/// The block itself is never rejected — invalid transactions are simply marked,
/// mirroring Hyperledger Fabric's committer behaviour.
pub fn commit_block(txs: &[EndorsedTransaction], state: &dyn WorldState) -> Vec<Transaction> {
    txs.iter()
        .map(|endorsed| {
            let mut tx = endorsed.proposal.tx.clone();
            match validate_rwset(&endorsed.rwset, state) {
                Ok(()) => {
                    // Apply writes to world state
                    for write in &endorsed.rwset.writes {
                        // Best-effort: ignore individual write errors (shouldn't happen
                        // with a healthy MemoryWorldState or RocksDB store).
                        let _ = state.put(&write.key, &write.value);
                    }
                    tx.state = "committed".to_string();
                }
                Err(_conflict) => {
                    tx.state = "mvcc_conflict".to_string();
                }
            }
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

    fn rwset(reads: &[(&str, u64)]) -> ReadWriteSet {
        ReadWriteSet {
            reads: reads
                .iter()
                .map(|(k, v)| KVRead { key: k.to_string(), version: *v })
                .collect(),
            writes: vec![],
        }
    }

    // ── no conflict ──────────────────────────────────────────────────────────

    #[test]
    fn empty_rwset_always_passes() {
        let state = ws();
        assert!(validate_rwset(&ReadWriteSet::default(), &state).is_ok());
    }

    #[test]
    fn read_absent_key_at_version_0_passes() {
        let state = ws();
        // Key does not exist; tx read it at version 0 (never written)
        assert!(validate_rwset(&rwset(&[("k", 0)]), &state).is_ok());
    }

    #[test]
    fn read_existing_key_at_correct_version_passes() {
        let state = ws();
        state.put("asset", b"v1").unwrap(); // version = 1
        assert!(validate_rwset(&rwset(&[("asset", 1)]), &state).is_ok());
    }

    #[test]
    fn multiple_reads_all_match_passes() {
        let state = ws();
        state.put("a", b"1").unwrap(); // v1
        state.put("b", b"x").unwrap(); // v1
        state.put("b", b"y").unwrap(); // v2
        assert!(validate_rwset(&rwset(&[("a", 1), ("b", 2)]), &state).is_ok());
    }

    // ── conflict ─────────────────────────────────────────────────────────────

    #[test]
    fn stale_read_returns_conflict() {
        let state = ws();
        state.put("asset", b"v1").unwrap(); // v1
        state.put("asset", b"v2").unwrap(); // v2 — concurrent update

        let err = validate_rwset(&rwset(&[("asset", 1)]), &state).unwrap_err();
        assert_eq!(err.key, "asset");
        assert_eq!(err.read_version, 1);
        assert_eq!(err.current_version, 2);
    }

    #[test]
    fn read_deleted_key_at_nonzero_version_returns_conflict() {
        let state = ws();
        state.put("gone", b"v").unwrap(); // v1
        state.delete("gone").unwrap(); // now absent

        // tx read it at v1, but it's gone (current = 0)
        let err = validate_rwset(&rwset(&[("gone", 1)]), &state).unwrap_err();
        assert_eq!(err.key, "gone");
        assert_eq!(err.read_version, 1);
        assert_eq!(err.current_version, 0);
    }

    #[test]
    fn read_phantom_key_at_nonzero_version_returns_conflict() {
        let state = ws();
        // key never existed; tx claims it read version 1
        let err = validate_rwset(&rwset(&[("phantom", 1)]), &state).unwrap_err();
        assert_eq!(err.read_version, 1);
        assert_eq!(err.current_version, 0);
    }

    // ── commit_block helpers ─────────────────────────────────────────────────

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

    fn endorsed(id: &str, reads: &[(&str, u64)], writes: &[(&str, &[u8])]) -> EndorsedTransaction {
        EndorsedTransaction {
            proposal: TransactionProposal {
                tx: base_tx(id),
                creator_did: "did:test:creator".to_string(),
                creator_signature: [0u8; 64],
                rwset: ReadWriteSet {
                    reads: reads
                        .iter()
                        .map(|(k, v)| KVRead { key: k.to_string(), version: *v })
                        .collect(),
                    writes: writes
                        .iter()
                        .map(|(k, v)| KVWrite { key: k.to_string(), value: v.to_vec() })
                        .collect(),
                },
            },
            endorsements: vec![Endorsement {
                signer_did: "did:test:org1".to_string(),
                org_id: "Org1".to_string(),
                signature: [0u8; 64],
                payload_hash: [0u8; 32],
                timestamp: 0,
            }],
            rwset: ReadWriteSet {
                reads: reads
                    .iter()
                    .map(|(k, v)| KVRead { key: k.to_string(), version: *v })
                    .collect(),
                writes: writes
                    .iter()
                    .map(|(k, v)| KVWrite { key: k.to_string(), value: v.to_vec() })
                    .collect(),
            },
        }
    }

    // ── commit_block tests ───────────────────────────────────────────────────

    #[test]
    fn commit_block_all_valid_marks_committed() {
        let state = ws();
        state.put("asset", b"v1").unwrap(); // v1

        let txs = vec![endorsed("tx1", &[("asset", 1)], &[("asset", b"v2")])];
        let results = commit_block(&txs, &state);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].state, "committed");
        // Write was applied
        assert_eq!(state.get("asset").unwrap().unwrap().version, 2);
    }

    #[test]
    fn commit_block_conflict_marks_mvcc_conflict() {
        let state = ws();
        state.put("asset", b"v1").unwrap();
        state.put("asset", b"v2").unwrap(); // now at v2

        // TX read asset at v1 — stale
        let txs = vec![endorsed("tx1", &[("asset", 1)], &[("asset", b"v3")])];
        let results = commit_block(&txs, &state);

        assert_eq!(results[0].state, "mvcc_conflict");
        // Write must NOT have been applied
        assert_eq!(state.get("asset").unwrap().unwrap().version, 2);
    }

    #[test]
    fn commit_block_three_txs_one_conflict() {
        let state = ws();
        state.put("k1", b"a").unwrap(); // v1
        state.put("k2", b"b").unwrap(); // v1
        state.put("k2", b"c").unwrap(); // v2 — concurrent update
        state.put("k3", b"d").unwrap(); // v1

        let txs = vec![
            endorsed("tx1", &[("k1", 1)], &[("k1", b"new1")]), // valid
            endorsed("tx2", &[("k2", 1)], &[("k2", b"new2")]), // conflict (k2 is v2)
            endorsed("tx3", &[("k3", 1)], &[("k3", b"new3")]), // valid
        ];
        let results = commit_block(&txs, &state);

        assert_eq!(results[0].id, "tx1");
        assert_eq!(results[0].state, "committed");
        assert_eq!(results[1].id, "tx2");
        assert_eq!(results[1].state, "mvcc_conflict");
        assert_eq!(results[2].id, "tx3");
        assert_eq!(results[2].state, "committed");

        // k2 write must not have been applied
        assert_eq!(state.get("k2").unwrap().unwrap().data, b"c");
    }

    #[test]
    fn conflict_reported_on_first_conflicting_key() {
        let state = ws();
        state.put("ok_key", b"v").unwrap(); // v1 — matches read
        state.put("bad_key", b"v").unwrap();
        state.put("bad_key", b"v2").unwrap(); // v2 — tx read at v1

        let rw = ReadWriteSet {
            reads: vec![
                KVRead { key: "ok_key".to_string(), version: 1 },
                KVRead { key: "bad_key".to_string(), version: 1 },
            ],
            writes: vec![KVWrite { key: "x".to_string(), value: b"y".to_vec() }],
        };
        let err = validate_rwset(&rw, &state).unwrap_err();
        assert_eq!(err.key, "bad_key");
    }
}
