//! State reconstruction — replay blocks from genesis to rebuild account state.
//!
//! A new node downloads blocks from peers and replays all native transactions
//! to deterministically reconstruct the same account state as the rest of the
//! network.

use crate::account::{AccountError, AccountStore};
use crate::tokenomics::economics::EconomicsState;
use crate::tokenomics::reward::apply_block_rewards;
use crate::transaction::native::{execute_transfer, NativeTransaction, TransactionKind};

/// Result of replaying the chain from genesis.
#[derive(Debug)]
pub struct ReplayResult {
    /// Final block height after replay.
    pub height: u64,
    /// Final economics state.
    pub economics: EconomicsState,
    /// Total transactions replayed (successful).
    pub txs_replayed: u64,
    /// Total transactions that failed during replay.
    pub txs_failed: u64,
}

/// Replay a sequence of produced blocks against an account store to
/// reconstruct state from genesis.
///
/// `blocks` is an iterator of `(proposer, successful_tx_list)` tuples
/// ordered by height. Each block's transactions are executed in order,
/// followed by block reward application.
///
/// This is deterministic: same blocks in same order → identical state.
pub fn replay_from_genesis(
    blocks: impl Iterator<Item = (String, Vec<NativeTransaction>)>,
    store: &dyn AccountStore,
    genesis_economics: &EconomicsState,
) -> Result<ReplayResult, AccountError> {
    let mut economics = genesis_economics.clone();
    let mut txs_replayed: u64 = 0;
    let mut txs_failed: u64 = 0;

    for (proposer, txs) in blocks {
        let mut block_fees: u64 = 0;
        let mut block_tx_count: u64 = 0;

        for tx in &txs {
            match &tx.kind {
                TransactionKind::Transfer { .. } => match execute_transfer(store, tx, &proposer) {
                    Ok(_) => {
                        block_fees += tx.fee;
                        block_tx_count += 1;
                        txs_replayed += 1;
                    }
                    Err(e) => {
                        log::warn!(
                            "Replay: tx {} failed at height {}: {e}",
                            tx.id,
                            economics.height
                        );
                        txs_failed += 1;
                    }
                },
                TransactionKind::Coinbase { to, amount } => {
                    store.credit(to, *amount)?;
                    txs_replayed += 1;
                }
            }
        }

        // Apply block reward (mint only — fees already distributed in execute_transfer)
        let reward_result = apply_block_rewards(&economics, store, &proposer, block_tx_count, 0)?;
        economics = reward_result.economics;

        // Update base fee using actual fee data
        let (new_state, _, _) =
            crate::tokenomics::economics::process_block(&economics, block_tx_count, block_fees);
        economics.base_fee = new_state.base_fee;
    }

    Ok(ReplayResult {
        height: economics.height,
        economics,
        txs_replayed,
        txs_failed,
    })
}

/// Compute a deterministic state hash from all accounts.
///
/// `hash = sha256(sorted accounts concatenated as "addr:balance:nonce\n")`
pub fn compute_state_hash(store: &dyn AccountStore) -> Result<[u8; 32], AccountError> {
    use pqc_crypto_module::legacy::legacy_sha256;

    let mut accounts = store.all_accounts()?;
    accounts.sort_by(|a, b| a.0.cmp(&b.0));

    let mut data = String::new();
    for (addr, state) in &accounts {
        data.push_str(&format!("{}:{}:{}\n", addr, state.balance, state.nonce));
    }

    legacy_sha256(data.as_bytes()).map_err(|e| AccountError::Internal(format!("hash: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::MemoryAccountStore;

    #[test]
    fn replay_empty_chain() {
        let store = MemoryAccountStore::new();
        let economics = EconomicsState::default();
        let result = replay_from_genesis(std::iter::empty(), &store, &economics).unwrap();
        assert_eq!(result.height, 0);
        assert_eq!(result.txs_replayed, 0);
    }

    #[test]
    fn replay_blocks_with_transfers() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
        let economics = EconomicsState::default();

        // Simulate 3 blocks
        let blocks = vec![
            (
                "miner".to_string(),
                vec![NativeTransaction::new_transfer("alice", "bob", 100, 0, 5)],
            ),
            (
                "miner".to_string(),
                vec![NativeTransaction::new_transfer(
                    "alice", "charlie", 200, 1, 5,
                )],
            ),
            ("miner".to_string(), vec![]), // empty block
        ];

        let result = replay_from_genesis(blocks.into_iter(), &store, &economics).unwrap();

        assert_eq!(result.height, 3);
        assert_eq!(result.txs_replayed, 2);
        assert_eq!(result.txs_failed, 0);

        let alice = store.get_account("alice").unwrap();
        assert_eq!(alice.balance, 10_000 - 100 - 5 - 200 - 5);
        assert_eq!(alice.nonce, 2);

        let bob = store.get_account("bob").unwrap();
        assert_eq!(bob.balance, 100);

        let miner = store.get_account("miner").unwrap();
        assert!(miner.balance > 0); // got block rewards
    }

    #[test]
    fn replay_deterministic_state_hash() {
        let store_a = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
        let store_b = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
        let economics = EconomicsState::default();

        let blocks: Vec<(String, Vec<NativeTransaction>)> = vec![
            (
                "miner".to_string(),
                vec![NativeTransaction::new_transfer("alice", "bob", 500, 0, 5)],
            ),
            (
                "miner".to_string(),
                vec![NativeTransaction::new_transfer("bob", "charlie", 100, 0, 5)],
            ),
        ];

        replay_from_genesis(blocks.clone().into_iter(), &store_a, &economics).unwrap();
        replay_from_genesis(blocks.into_iter(), &store_b, &economics).unwrap();

        let hash_a = compute_state_hash(&store_a).unwrap();
        let hash_b = compute_state_hash(&store_b).unwrap();
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn replay_100_blocks() {
        let store = MemoryAccountStore::with_genesis(&[("alice", 1_000_000)]);
        let economics = EconomicsState::default();

        let mut blocks = Vec::new();
        for i in 0u64..100 {
            let txs = if i % 3 == 0 {
                vec![NativeTransaction::new_transfer(
                    "alice",
                    "bob",
                    10,
                    i / 3,
                    2,
                )]
            } else {
                vec![]
            };
            blocks.push(("miner".to_string(), txs));
        }

        let result = replay_from_genesis(blocks.into_iter(), &store, &economics).unwrap();

        assert_eq!(result.height, 100);
        // 34 txs (i=0,3,6,...,99 → 34 values)
        assert_eq!(result.txs_replayed, 34);

        // Miner got 100 block rewards + fee proposer shares
        let miner = store.get_account("miner").unwrap();
        // 100 blocks × 50 reward = 5000
        // 34 txs × fee 2 → proposer gets 20% = 0.4 → integer: 2 - (2*80/100) = 2 - 1 = 1 per tx
        // Total: 5000 + 34 = 5034
        assert_eq!(miner.balance, 5034);
    }

    #[test]
    fn state_hash_differs_with_different_state() {
        let store_a = MemoryAccountStore::with_genesis(&[("alice", 1000)]);
        let store_b = MemoryAccountStore::with_genesis(&[("alice", 2000)]);

        let hash_a = compute_state_hash(&store_a).unwrap();
        let hash_b = compute_state_hash(&store_b).unwrap();
        assert_ne!(hash_a, hash_b);
    }
}
