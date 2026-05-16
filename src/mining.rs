//! Mining service backed by BlockStore.
//!
//! Encapsulates block creation, reward calculation, and persistence
//! using the new storage layer. Replaces `Blockchain::mine_block_with_reward`.

use crate::crypto::hasher::{hash, HashAlgorithm};
use crate::identity::signing::SigningAlgorithm;
use crate::storage::traits::{Block, BlockStore, Transaction};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for the mining service.
#[derive(Debug, Clone)]
pub struct MiningConfig {
    pub base_reward: u64,
    pub halving_interval: u64,
    pub burn_percentage: u64,
    pub miner_fee_share: u64,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            base_reward: 50,
            halving_interval: 210_000,
            burn_percentage: 80,
            miner_fee_share: 20,
        }
    }
}

/// Mining service that creates blocks and writes them to BlockStore.
pub struct MiningService {
    store: Arc<dyn BlockStore>,
    config: MiningConfig,
}

impl MiningService {
    pub fn new(store: Arc<dyn BlockStore>, config: MiningConfig) -> Self {
        Self { store, config }
    }

    /// Mine a new block with the given transactions and miner reward.
    ///
    /// Returns the block height on success.
    pub fn mine_block(
        &self,
        miner_address: &str,
        transactions: Vec<Transaction>,
    ) -> Result<u64, String> {
        let latest_height = self.store.get_latest_height().unwrap_or(0);
        let new_height = if self.store.block_exists(0).unwrap_or(false) {
            latest_height + 1
        } else {
            0
        };

        // Calculate rewards (fees not tracked in storage::Transaction — reward only)
        let reward = self.calculate_reward(new_height);
        let total_reward = reward;

        // Build coinbase transaction
        let coinbase = Transaction {
            id: format!("coinbase-{new_height}"),
            block_height: new_height,
            timestamp: now(),
            input_did: "coinbase".to_string(),
            output_recipient: miner_address.to_string(),
            amount: total_reward,
            state: "confirmed".to_string(),
        };

        // Get parent hash
        let parent_hash = if new_height > 0 {
            let parent = self
                .store
                .read_block(new_height - 1)
                .map_err(|e| format!("failed to read parent block: {e}"))?;
            block_hash(&parent)
        } else {
            [0u8; 32]
        };

        // Collect all tx IDs
        let mut all_tx_ids = vec![coinbase.id.clone()];
        all_tx_ids.extend(transactions.iter().map(|tx| tx.id.clone()));

        // Build merkle root (simplified: hash of concatenated tx IDs)
        let merkle_data: String = all_tx_ids.join(",");
        let merkle_root = hash(merkle_data.as_bytes());

        // Create block
        let block = Block {
            height: new_height,
            timestamp: now(),
            parent_hash,
            merkle_root,
            transactions: all_tx_ids,
            proposer: miner_address.to_string(),
            signature: Vec::new(),
            signature_algorithm: SigningAlgorithm::default(),
            endorsements: Vec::new(),
            secondary_signature: None,
            secondary_signature_algorithm: None,
            hash_algorithm: HashAlgorithm::default(),
            orderer_signature: None,
        };

        // Write block and transactions
        self.store
            .write_block(&block)
            .map_err(|e| format!("failed to write block: {e}"))?;

        // Write coinbase tx
        self.store
            .write_transaction(&coinbase)
            .map_err(|e| format!("failed to write coinbase tx: {e}"))?;

        // Write user transactions with block_height set
        for mut tx in transactions {
            tx.block_height = new_height;
            tx.state = "confirmed".to_string();
            self.store
                .write_transaction(&tx)
                .map_err(|e| format!("failed to write tx: {e}"))?;
        }

        Ok(new_height)
    }

    /// Calculate mining reward with halving schedule.
    fn calculate_reward(&self, height: u64) -> u64 {
        let halvings = height / self.config.halving_interval;
        self.config.base_reward >> halvings.min(64)
    }
}

/// Compute SHA-256 hash of a block (for parent_hash linkage).
fn block_hash(block: &Block) -> [u8; 32] {
    let data = format!(
        "{}:{}:{:?}:{:?}",
        block.height, block.timestamp, block.parent_hash, block.transactions
    );
    hash(data.as_bytes())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStore;

    #[test]
    fn mine_genesis_block() {
        let store = Arc::new(MemoryStore::new());
        let service = MiningService::new(store.clone(), MiningConfig::default());

        let height = service.mine_block("miner1", vec![]).unwrap();
        assert_eq!(height, 0);

        let block = store.read_block(0).unwrap();
        assert_eq!(block.proposer, "miner1");
        assert_eq!(block.transactions.len(), 1); // coinbase only
    }

    #[test]
    fn mine_second_block_links_parent() {
        let store = Arc::new(MemoryStore::new());
        let service = MiningService::new(store.clone(), MiningConfig::default());

        service.mine_block("miner1", vec![]).unwrap();
        service.mine_block("miner1", vec![]).unwrap();

        let block1 = store.read_block(1).unwrap();
        let block0 = store.read_block(0).unwrap();
        assert_eq!(block1.parent_hash, block_hash(&block0));
    }

    #[test]
    fn mining_reward_halves() {
        let store = Arc::new(MemoryStore::new());
        let config = MiningConfig {
            halving_interval: 10,
            ..Default::default()
        };
        let service = MiningService::new(store, config);

        assert_eq!(service.calculate_reward(0), 50);
        assert_eq!(service.calculate_reward(9), 50);
        assert_eq!(service.calculate_reward(10), 25);
        assert_eq!(service.calculate_reward(20), 12);
    }

    #[test]
    fn mine_block_with_transactions() {
        let store = Arc::new(MemoryStore::new());
        let service = MiningService::new(store.clone(), MiningConfig::default());

        let tx = Transaction {
            id: "tx-1".to_string(),
            block_height: 0,
            timestamp: 0,
            input_did: "alice".to_string(),
            output_recipient: "bob".to_string(),
            amount: 10,
            state: "pending".to_string(),
        };

        let height = service.mine_block("miner1", vec![tx]).unwrap();
        assert_eq!(height, 0);

        let block = store.read_block(0).unwrap();
        assert_eq!(block.transactions.len(), 2); // coinbase + tx-1

        // Verify transaction was persisted with confirmed state
        let stored_tx = store.read_transaction("tx-1").unwrap();
        assert_eq!(stored_tx.state, "confirmed");
        assert_eq!(stored_tx.block_height, 0);
    }
}
