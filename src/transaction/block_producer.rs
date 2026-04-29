//! Block production pipeline for native cryptocurrency transactions.
//!
//! `produce_block` drains the mempool, executes transfers against the account
//! store, applies block rewards, and returns a summary of the produced block.

use crate::account::{AccountError, AccountStore};
use crate::tokenomics::economics::EconomicsState;
use crate::tokenomics::reward::{apply_block_rewards, BlockRewardResult};
use crate::transaction::mempool::Mempool;
use crate::transaction::native::{execute_transfer, NativeTransaction, NativeTxError};

/// Result of executing a single transaction within a block.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TxExecResult {
    pub tx_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub fee_burned: u64,
    pub fee_to_proposer: u64,
}

/// Summary of a produced block.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProducedBlock {
    /// Block height.
    pub height: u64,
    /// Proposer who receives rewards.
    pub proposer: String,
    /// Transactions included (both successful and failed).
    pub tx_results: Vec<TxExecResult>,
    /// Number of successfully executed transactions.
    pub tx_success_count: usize,
    /// Total fees burned across all successful txs.
    pub total_burned: u64,
    /// Total fees paid to proposer across all successful txs.
    pub total_proposer_fees: u64,
    /// Block reward minted to proposer.
    pub block_reward: u64,
    /// Updated economics state after this block.
    pub economics: EconomicsState,
}

/// Produce a block by draining the mempool and executing transactions.
///
/// Pipeline:
/// 1. Drain up to `max_txs` highest-fee transactions from mempool
/// 2. Execute each transfer against the account store
/// 3. Failed txs are recorded but don't revert the block
/// 4. Apply block rewards (reward + fee split) to proposer
/// 5. Return block summary with updated economics
pub fn produce_block(
    mempool: &Mempool,
    account_store: &dyn AccountStore,
    economics: &EconomicsState,
    proposer: &str,
    max_txs: usize,
) -> Result<ProducedBlock, AccountError> {
    let txs = mempool.drain_top(max_txs);

    let mut tx_results = Vec::with_capacity(txs.len());
    let mut total_fees_collected: u64 = 0;

    for tx in &txs {
        match execute_single(account_store, tx, proposer) {
            Ok((burned, to_proposer)) => {
                total_fees_collected += tx.fee;
                tx_results.push(TxExecResult {
                    tx_id: tx.id.clone(),
                    success: true,
                    error: None,
                    fee_burned: burned,
                    fee_to_proposer: to_proposer,
                });
            }
            Err(e) => {
                tx_results.push(TxExecResult {
                    tx_id: tx.id.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    fee_burned: 0,
                    fee_to_proposer: 0,
                });
            }
        }
    }

    let success_count = tx_results.iter().filter(|r| r.success).count();

    // Apply block rewards (reward is separate from tx fee split which was already applied
    // in execute_transfer — here we only apply the minting reward)
    let reward_result = apply_block_rewards_mint_only(
        economics,
        account_store,
        proposer,
        success_count as u64,
        total_fees_collected,
    )?;

    let total_burned: u64 = tx_results.iter().map(|r| r.fee_burned).sum();
    let total_proposer_fees: u64 = tx_results.iter().map(|r| r.fee_to_proposer).sum();

    Ok(ProducedBlock {
        height: reward_result.economics.height,
        proposer: proposer.to_string(),
        tx_results,
        tx_success_count: success_count,
        total_burned,
        total_proposer_fees,
        block_reward: reward_result.reward,
        economics: reward_result.economics,
    })
}

/// Execute a single native transaction. Fee split is handled by `execute_transfer`.
fn execute_single(
    store: &dyn AccountStore,
    tx: &NativeTransaction,
    proposer: &str,
) -> Result<(u64, u64), NativeTxError> {
    execute_transfer(store, tx, proposer)
}

/// Apply only the block reward (minting) — fee split was already done per-tx.
/// We still call `apply_block_rewards` for the economics state transition but
/// pass total_fees=0 so it doesn't double-credit fees.
fn apply_block_rewards_mint_only(
    economics: &EconomicsState,
    store: &dyn AccountStore,
    proposer: &str,
    tx_count: u64,
    total_fees: u64,
) -> Result<BlockRewardResult, AccountError> {
    // Pass total_fees for base fee adjustment calculation,
    // but set fee=0 for the actual credit (fees already distributed per-tx)
    let result = apply_block_rewards(economics, store, proposer, tx_count, 0)?;

    // Still need economics to know about fees for base fee adjustment
    // Re-derive the state with correct fee tracking
    let (new_state, _, _) =
        crate::tokenomics::economics::process_block(economics, tx_count, total_fees);

    Ok(BlockRewardResult {
        economics: EconomicsState {
            // Use process_block's state for correct base_fee and epoch tracking
            height: new_state.height,
            total_minted: result.economics.total_minted,
            total_burned: new_state.total_burned,
            base_fee: new_state.base_fee,
            epoch: new_state.epoch,
            epoch_fees: new_state.epoch_fees,
        },
        reward: result.reward,
        fee_split: result.fee_split,
    })
}

// ── Persistence ────────────────────────────────────────────────────────────

use crate::storage::errors::StorageResult;
use crate::storage::traits::{Block, BlockStore, Transaction as StorageTx};

/// Convert a `ProducedBlock` to a storage `Block` and persist it along with
/// its transactions to the given `BlockStore`.
///
/// `parent_hash` must be the hash of the previous block (or `[0u8; 32]` for genesis).
pub fn persist_block(
    produced: &ProducedBlock,
    store: &dyn BlockStore,
    parent_hash: [u8; 32],
) -> StorageResult<()> {
    let tx_ids: Vec<String> = produced
        .tx_results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.tx_id.clone())
        .collect();

    // Simple merkle root: hash of concatenated tx IDs
    let merkle_root = {
        use pqc_crypto_module::legacy::legacy_sha256;
        let concat: String = tx_ids.join("");
        legacy_sha256(concat.as_bytes()).unwrap_or([0u8; 32])
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let block = Block {
        height: produced.height,
        timestamp: now,
        parent_hash,
        merkle_root,
        transactions: tx_ids,
        proposer: produced.proposer.clone(),
        signature: Vec::new(),
        signature_algorithm: Default::default(),
        endorsements: Vec::new(),
        secondary_signature: None,
        secondary_signature_algorithm: None,
        hash_algorithm: Default::default(),
        orderer_signature: None,
    };

    store.write_block(&block)?;

    // Persist successful transactions
    for result in &produced.tx_results {
        if !result.success {
            continue;
        }
        let tx = StorageTx {
            id: result.tx_id.clone(),
            block_height: produced.height,
            timestamp: now,
            input_did: String::new(),
            output_recipient: String::new(),
            amount: 0,
            state: "confirmed".to_string(),
        };
        store.write_transaction(&tx)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::MemoryAccountStore;
    use crate::tokenomics::economics::INITIAL_BLOCK_REWARD;
    use crate::transaction::mempool::MempoolConfig;

    fn setup() -> (Mempool, MemoryAccountStore, EconomicsState) {
        let mempool = Mempool::new(MempoolConfig {
            max_size: 100,
            max_per_sender: 20,
            min_fee: 1,
        });
        let store = MemoryAccountStore::with_genesis(&[
            ("alice", 10_000),
            ("bob", 5_000),
            ("charlie", 1_000),
        ]);
        let economics = EconomicsState::default();
        (mempool, store, economics)
    }

    #[test]
    fn empty_block_still_rewards_proposer() {
        let (mempool, store, economics) = setup();
        let block = produce_block(&mempool, &store, &economics, "miner", 100).unwrap();

        assert_eq!(block.height, 1);
        assert_eq!(block.tx_success_count, 0);
        assert_eq!(block.block_reward, INITIAL_BLOCK_REWARD);
        assert_eq!(block.tx_results.len(), 0);

        let miner = store.get_account("miner").unwrap();
        assert_eq!(miner.balance, INITIAL_BLOCK_REWARD);
    }

    #[test]
    fn block_with_transfers() {
        let (mempool, store, economics) = setup();

        // Queue 3 transfers from different senders (avoids nonce ordering issues)
        let tx1 = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
        let tx2 = NativeTransaction::new_transfer("bob", "charlie", 50, 0, 10);
        let tx3 = NativeTransaction::new_transfer("charlie", "alice", 30, 0, 3);
        mempool.add(tx1).unwrap();
        mempool.add(tx2).unwrap();
        mempool.add(tx3).unwrap();

        let block = produce_block(&mempool, &store, &economics, "miner", 100).unwrap();

        assert_eq!(block.tx_results.len(), 3);
        assert_eq!(block.tx_success_count, 3);
        assert_eq!(block.height, 1);

        // Mempool drained
        assert!(mempool.is_empty());

        // Miner got block reward
        let miner = store.get_account("miner").unwrap();
        assert!(miner.balance >= INITIAL_BLOCK_REWARD);
    }

    #[test]
    fn failed_tx_recorded_but_others_proceed() {
        let (mempool, store, economics) = setup();

        // Good tx
        let tx1 = NativeTransaction::new_transfer("alice", "bob", 100, 0, 10);
        // Bad tx: charlie only has 1000, trying to send 5000
        let tx2 = NativeTransaction::new_transfer("charlie", "alice", 5000, 0, 5);
        mempool.add(tx1).unwrap();
        mempool.add(tx2).unwrap();

        let block = produce_block(&mempool, &store, &economics, "miner", 100).unwrap();

        assert_eq!(block.tx_results.len(), 2);
        // One succeeds, one fails
        let successes = block.tx_results.iter().filter(|r| r.success).count();
        let failures = block.tx_results.iter().filter(|r| !r.success).count();
        assert_eq!(successes, 1);
        assert_eq!(failures, 1);

        // Failed tx has error message
        let failed = block.tx_results.iter().find(|r| !r.success).unwrap();
        assert!(failed.error.is_some());
    }

    #[test]
    fn max_txs_limit_respected() {
        let (mempool, store, economics) = setup();

        for i in 0..10 {
            let tx = NativeTransaction::new_transfer("alice", "bob", 1, i, 5);
            mempool.add(tx).unwrap();
        }

        let block = produce_block(&mempool, &store, &economics, "miner", 3).unwrap();

        // Only 3 drained, 7 remain
        assert_eq!(block.tx_results.len(), 3);
        assert_eq!(mempool.len(), 7);
    }

    #[test]
    fn sequential_blocks_advance_economics() {
        let (mempool, store, economics) = setup();

        let b1 = produce_block(&mempool, &store, &economics, "miner", 100).unwrap();
        assert_eq!(b1.height, 1);

        let b2 = produce_block(&mempool, &store, &b1.economics, "miner", 100).unwrap();
        assert_eq!(b2.height, 2);
        assert_eq!(b2.economics.total_minted, INITIAL_BLOCK_REWARD * 2);

        let miner = store.get_account("miner").unwrap();
        assert_eq!(miner.balance, INITIAL_BLOCK_REWARD * 2);
    }

    #[test]
    fn highest_fee_txs_processed_first() {
        let (mempool, store, economics) = setup();

        let tx_low = NativeTransaction::new_transfer("alice", "bob", 10, 0, 1);
        let tx_high = NativeTransaction::new_transfer("bob", "charlie", 10, 0, 100);
        mempool.add(tx_low).unwrap();
        mempool.add(tx_high).unwrap();

        // Only take 1
        let block = produce_block(&mempool, &store, &economics, "miner", 1).unwrap();

        assert_eq!(block.tx_results.len(), 1);
        assert!(block.tx_results[0].success);
        // The high-fee tx from bob should have been picked
        // bob started with 5000, paid 10 + 100 fee
        let bob = store.get_account("bob").unwrap();
        assert_eq!(bob.balance, 5000 - 10 - 100);
    }

    #[test]
    fn persist_block_writes_to_store() {
        use crate::storage::memory::MemoryStore;
        use std::sync::Arc;

        let (mempool, account_store, economics) = setup();
        let tx = NativeTransaction::new_transfer("alice", "bob", 50, 0, 5);
        mempool.add(tx).unwrap();

        let produced = produce_block(&mempool, &account_store, &economics, "miner", 100).unwrap();

        let block_store = Arc::new(MemoryStore::new());
        persist_block(&produced, block_store.as_ref(), [0u8; 32]).unwrap();

        // Block is persisted at height 1
        let stored: Block = block_store.read_block(1).expect("block should exist");
        assert_eq!(stored.height, 1);
        assert_eq!(stored.proposer, "miner");
        assert_eq!(stored.transactions.len(), 1); // 1 successful tx
    }

    #[test]
    fn persist_block_only_includes_successful_txs() {
        use crate::storage::memory::MemoryStore;
        use std::sync::Arc;

        let (mempool, account_store, economics) = setup();
        // Good tx
        let tx1 = NativeTransaction::new_transfer("alice", "bob", 50, 0, 5);
        // Bad tx: charlie only has 1000, trying 5000
        let tx2 = NativeTransaction::new_transfer("charlie", "alice", 5000, 0, 3);
        mempool.add(tx1).unwrap();
        mempool.add(tx2).unwrap();

        let produced = produce_block(&mempool, &account_store, &economics, "miner", 100).unwrap();
        assert_eq!(produced.tx_success_count, 1);

        let block_store = Arc::new(MemoryStore::new());
        persist_block(&produced, block_store.as_ref(), [0u8; 32]).unwrap();

        let stored: Block = block_store.read_block(1).expect("block should exist");
        // Only 1 successful tx persisted
        assert_eq!(stored.transactions.len(), 1);
    }
}
