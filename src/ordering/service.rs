use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::metrics::MetricsCollector;
use crate::storage::{errors::StorageResult, traits::{Block, BlockStore, Transaction}};

/// Collects endorsed transactions and cuts them into ordered blocks.
pub struct OrderingService {
    pub(crate) pending_txs: Mutex<VecDeque<Transaction>>,
    pub max_batch_size: usize,
    pub batch_timeout_ms: u64,
    metrics: Option<Arc<MetricsCollector>>,
    signing_key: Option<ed25519_dalek::SigningKey>,
}

impl OrderingService {
    /// Create a new `OrderingService` reading config from env:
    /// - `ORDERING_BATCH_SIZE` (default 100)
    /// - `ORDERING_BATCH_TIMEOUT_MS` (default 2000)
    pub fn new() -> Self {
        let max_batch_size = std::env::var("ORDERING_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let batch_timeout_ms = std::env::var("ORDERING_BATCH_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2000);

        Self::with_config(max_batch_size, batch_timeout_ms)
    }

    pub fn with_config(max_batch_size: usize, batch_timeout_ms: u64) -> Self {
        Self {
            pending_txs: Mutex::new(VecDeque::new()),
            max_batch_size,
            batch_timeout_ms,
            metrics: None,
            signing_key: None,
        }
    }

    /// Attach a metrics collector so `cut_block` increments `ordering_blocks_cut_total`.
    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Attach an Ed25519 signing key so `cut_block` signs each block.
    pub fn with_signing_key(mut self, key: ed25519_dalek::SigningKey) -> Self {
        self.signing_key = Some(key);
        self
    }

    /// Enqueue a transaction for the next ordered block.
    pub fn submit_tx(&self, tx: Transaction) -> StorageResult<()> {
        self.pending_txs.lock().unwrap().push_back(tx);
        Ok(())
    }

    /// Enqueue an endorsed transaction for the next ordered block.
    ///
    /// Extracts the inner `Transaction` from the proposal and enqueues it.
    /// For MVP the endorsement metadata is validated by the Gateway before
    /// calling this method; a future version should carry the full
    /// `EndorsedTransaction` through to the block so committer peers can
    /// re-validate endorsements.
    pub fn submit_endorsed_tx(
        &self,
        etx: crate::transaction::endorsed::EndorsedTransaction,
    ) -> StorageResult<()> {
        self.pending_txs.lock().unwrap().push_back(etx.proposal.tx);
        Ok(())
    }

    /// Number of transactions currently waiting to be ordered.
    pub fn pending_count(&self) -> usize {
        self.pending_txs.lock().unwrap().len()
    }

    /// Drain up to `max_batch_size` transactions and create an ordered `Block`.
    /// Returns `None` if the pending queue is empty.
    pub fn cut_block(&self, height: u64, proposer: &str) -> StorageResult<Option<Block>> {
        let mut queue = self.pending_txs.lock().unwrap();
        if queue.is_empty() {
            return Ok(None);
        }

        let count = queue.len().min(self.max_batch_size);
        let tx_ids: Vec<String> = queue.drain(..count).map(|tx| tx.id).collect();

        let mut block = Block {
            height,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            parent_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            transactions: tx_ids,
            proposer: proposer.to_string(),
            signature: [0u8; 64],
            endorsements: vec![],
            orderer_signature: None,
        };

        if let Some(key) = &self.signing_key {
            super::sign_block(&mut block, key);
        }

        if let Some(m) = &self.metrics {
            m.record_ordering_block_cut();
        }
        Ok(Some(block))
    }
}

/// Continuously drain pending transactions into ordered blocks on a timer.
///
/// Launched via `tokio::spawn` in `main.rs` when `role == Orderer || PeerAndOrderer`.
/// The height counter is local to this loop; a future phase can derive it from the store.
pub async fn run_batch_loop(service: Arc<OrderingService>, store: Arc<dyn BlockStore>) {
    let timeout = tokio::time::Duration::from_millis(service.batch_timeout_ms);
    let mut height: u64 = store.get_latest_height().unwrap_or(0) + 1;

    loop {
        tokio::time::sleep(timeout).await;
        match service.cut_block(height, "orderer") {
            Ok(Some(block)) => {
                height += 1;
                if let Err(e) = store.write_block(&block) {
                    eprintln!("ordering: failed to write block {}: {e}", block.height);
                }
            }
            Ok(None) => {} // No pending txs — nothing to do.
            Err(e) => eprintln!("ordering: cut_block error: {e}"),
        }
    }
}

impl super::OrderingBackend for OrderingService {
    fn submit_tx(&self, tx: &Transaction) -> StorageResult<()> {
        self.submit_tx(tx.clone())
    }

    fn cut_block(&self, height: u64, proposer: &str) -> StorageResult<Option<Block>> {
        self.cut_block(height, proposer)
    }

    fn pending_count(&self) -> usize {
        self.pending_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_service_with_defaults() {
        let svc = OrderingService::with_config(100, 2000);
        assert_eq!(svc.max_batch_size, 100);
        assert_eq!(svc.batch_timeout_ms, 2000);
        assert_eq!(svc.pending_txs.lock().unwrap().len(), 0);
    }

    #[test]
    fn respects_custom_config() {
        let svc = OrderingService::with_config(50, 500);
        assert_eq!(svc.max_batch_size, 50);
        assert_eq!(svc.batch_timeout_ms, 500);
    }

    fn make_tx(id: &str) -> Transaction {
        Transaction {
            id: id.to_string(),
            block_height: 0,
            timestamp: 0,
            input_did: "did:bc:alice".to_string(),
            output_recipient: "did:bc:bob".to_string(),
            amount: 1,
            state: "pending".to_string(),
        }
    }

    #[test]
    fn submit_three_txs_pending_count_is_three() {
        let svc = OrderingService::with_config(100, 2000);
        svc.submit_tx(make_tx("tx1")).unwrap();
        svc.submit_tx(make_tx("tx2")).unwrap();
        svc.submit_tx(make_tx("tx3")).unwrap();
        assert_eq!(svc.pending_count(), 3);
    }

    #[tokio::test]
    async fn batch_loop_cuts_block_after_timeout() {
        use crate::storage::{traits::BlockStore, MemoryStore};

        let svc = Arc::new(OrderingService::with_config(100, 50)); // 50ms timeout
        let store: Arc<dyn BlockStore> = Arc::new(MemoryStore::new());

        svc.submit_tx(make_tx("tx1")).unwrap();

        let svc2 = svc.clone();
        let store2 = store.clone();
        let handle = tokio::spawn(super::run_batch_loop(svc2, store2));

        // Wait long enough for at least one cut (>50ms).
        tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
        handle.abort();

        // Block should be persisted in the store.
        let block = store.read_block(1).unwrap();
        assert_eq!(block.transactions, vec!["tx1"]);
    }

    #[test]
    fn cut_block_batches_up_to_max_size() {
        let svc = OrderingService::with_config(3, 2000);
        for i in 0..5 {
            svc.submit_tx(make_tx(&format!("tx{i}"))).unwrap();
        }

        // First cut: 3 txs
        let b1 = svc.cut_block(1, "orderer1").unwrap().unwrap();
        assert_eq!(b1.transactions.len(), 3);
        assert_eq!(b1.height, 1);
        assert_eq!(b1.proposer, "orderer1");

        // Second cut: remaining 2 txs
        let b2 = svc.cut_block(2, "orderer1").unwrap().unwrap();
        assert_eq!(b2.transactions.len(), 2);

        // Queue now empty
        assert!(svc.cut_block(3, "orderer1").unwrap().is_none());
    }
}
