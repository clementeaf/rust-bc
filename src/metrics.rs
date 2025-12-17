/**
 * Prometheus Metrics Module
 *
 * Thread-safe metrics collection for blockchain monitoring and observability.
 * Provides counters, gauges, and histograms for:
 * - Blockchain state (blocks, height, difficulty)
 * - Transactions (validated, rejected, fees)
 * - Mempool (pending transactions, fees)
 * - Network (connected peers)
 * - Performance (block time, validation time)
 */

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Blockchain metrics
#[derive(Clone)]
pub struct BlockchainMetrics {
    pub blocks_total: Arc<AtomicU64>,
    pub transactions_total: Arc<AtomicU64>,
    pub chain_height: Arc<AtomicU64>,
    pub difficulty: Arc<AtomicU64>,
    pub last_block_time_ms: Arc<AtomicU64>,
}

impl BlockchainMetrics {
    #[allow(dead_code)]
    pub fn new() -> Self {
        BlockchainMetrics {
            blocks_total: Arc::new(AtomicU64::new(0)),
            transactions_total: Arc::new(AtomicU64::new(0)),
            chain_height: Arc::new(AtomicU64::new(0)),
            difficulty: Arc::new(AtomicU64::new(0)),
            last_block_time_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    #[allow(dead_code)]
    pub fn record_block(&self, tx_count: u64, difficulty: u8, chain_height: u64) {
        self.blocks_total.fetch_add(1, Ordering::Relaxed);
        self.transactions_total.fetch_add(tx_count, Ordering::Relaxed);
        self.chain_height.store(chain_height, Ordering::Relaxed);
        self.difficulty.store(difficulty as u64, Ordering::Relaxed);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.last_block_time_ms.store(now, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn update_height(&self, height: u64) {
        self.chain_height.store(height, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn update_difficulty(&self, difficulty: u8) {
        self.difficulty.store(difficulty as u64, Ordering::Relaxed);
    }
}

impl Default for BlockchainMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction metrics
#[derive(Clone)]
pub struct TransactionMetrics {
    pub validated_total: Arc<AtomicU64>,
    pub rejected_total: Arc<AtomicU64>,
    pub total_fees: Arc<AtomicU64>,
    pub validation_time_sum_ms: Arc<AtomicU64>,
}

impl TransactionMetrics {
    #[allow(dead_code)]
    pub fn new() -> Self {
        TransactionMetrics {
            validated_total: Arc::new(AtomicU64::new(0)),
            rejected_total: Arc::new(AtomicU64::new(0)),
            total_fees: Arc::new(AtomicU64::new(0)),
            validation_time_sum_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    #[allow(dead_code)]
    pub fn record_validated(&self, fee: u64, validation_time_ms: u64) {
        self.validated_total.fetch_add(1, Ordering::Relaxed);
        self.total_fees.fetch_add(fee, Ordering::Relaxed);
        self.validation_time_sum_ms
            .fetch_add(validation_time_ms, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_rejected(&self, _reason: &str) {
        self.rejected_total.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for TransactionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Mempool metrics
#[derive(Clone)]
pub struct MempoolMetrics {
    pub pending_transactions: Arc<AtomicU64>,
    pub total_fees_pending: Arc<AtomicU64>,
    pub oldest_pending_timestamp: Arc<AtomicU64>,
}

impl MempoolMetrics {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        MempoolMetrics {
            pending_transactions: Arc::new(AtomicU64::new(0)),
            total_fees_pending: Arc::new(AtomicU64::new(0)),
            oldest_pending_timestamp: Arc::new(AtomicU64::new(now)),
        }
    }

    #[allow(dead_code)]
    pub fn update_state(&self, pending_count: u64, total_fees: u64, oldest_timestamp: u64) {
        self.pending_transactions
            .store(pending_count, Ordering::Relaxed);
        self.total_fees_pending.store(total_fees, Ordering::Relaxed);
        self.oldest_pending_timestamp
            .store(oldest_timestamp, Ordering::Relaxed);
    }
}

impl Default for MempoolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Network metrics
#[derive(Clone)]
pub struct NetworkMetrics {
    pub connected_peers: Arc<AtomicU64>,
    pub messages_received: Arc<AtomicU64>,
    pub messages_sent: Arc<AtomicU64>,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        NetworkMetrics {
            connected_peers: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    #[allow(dead_code)]
    pub fn update_peers(&self, count: u64) {
        self.connected_peers.store(count, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_message_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Central metrics collector
#[derive(Clone)]
pub struct MetricsCollector {
    pub blockchain: BlockchainMetrics,
    pub transactions: TransactionMetrics,
    pub mempool: MempoolMetrics,
    pub network: NetworkMetrics,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            blockchain: BlockchainMetrics::new(),
            transactions: TransactionMetrics::new(),
            mempool: MempoolMetrics::new(),
            network: NetworkMetrics::new(),
        }
    }

    /// Generate Prometheus text format metrics
    pub fn collect_metrics(&self) -> String {
        let mut output = String::new();

        // Blockchain metrics
        output.push_str("# HELP blockchain_blocks_total Total number of blocks in the chain\n");
        output.push_str("# TYPE blockchain_blocks_total counter\n");
        output.push_str(&format!(
            "blockchain_blocks_total {}\n",
            self.blockchain.blocks_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP blockchain_transactions_total Total number of transactions\n");
        output.push_str("# TYPE blockchain_transactions_total counter\n");
        output.push_str(&format!(
            "blockchain_transactions_total {}\n",
            self.blockchain.transactions_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP blockchain_height Current blockchain height\n");
        output.push_str("# TYPE blockchain_height gauge\n");
        output.push_str(&format!(
            "blockchain_height {}\n",
            self.blockchain.chain_height.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP blockchain_difficulty Current mining difficulty\n");
        output.push_str("# TYPE blockchain_difficulty gauge\n");
        output.push_str(&format!(
            "blockchain_difficulty {}\n",
            self.blockchain.difficulty.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP blockchain_last_block_time_ms Timestamp of last block (ms since epoch)\n");
        output.push_str("# TYPE blockchain_last_block_time_ms gauge\n");
        output.push_str(&format!(
            "blockchain_last_block_time_ms {}\n",
            self.blockchain.last_block_time_ms.load(Ordering::Relaxed)
        ));

        // Transaction metrics
        output.push_str("# HELP transactions_validated_total Total validated transactions\n");
        output.push_str("# TYPE transactions_validated_total counter\n");
        output.push_str(&format!(
            "transactions_validated_total {}\n",
            self.transactions.validated_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP transactions_rejected_total Total rejected transactions\n");
        output.push_str("# TYPE transactions_rejected_total counter\n");
        output.push_str(&format!(
            "transactions_rejected_total {}\n",
            self.transactions.rejected_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP transactions_total_fees_collected Total fees collected\n");
        output.push_str("# TYPE transactions_total_fees_collected counter\n");
        output.push_str(&format!(
            "transactions_total_fees_collected {}\n",
            self.transactions.total_fees.load(Ordering::Relaxed)
        ));

        let total_validated = self.transactions.validated_total.load(Ordering::Relaxed);
        let avg_validation_time = if total_validated > 0 {
            self.transactions.validation_time_sum_ms.load(Ordering::Relaxed) / total_validated
        } else {
            0
        };

        output.push_str("# HELP transactions_avg_validation_time_ms Average transaction validation time (ms)\n");
        output.push_str("# TYPE transactions_avg_validation_time_ms gauge\n");
        output.push_str(&format!(
            "transactions_avg_validation_time_ms {}\n",
            avg_validation_time
        ));

        // Mempool metrics
        output.push_str("# HELP mempool_pending_transactions Number of pending transactions in mempool\n");
        output.push_str("# TYPE mempool_pending_transactions gauge\n");
        output.push_str(&format!(
            "mempool_pending_transactions {}\n",
            self.mempool.pending_transactions.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP mempool_total_fees_pending Total fees of pending transactions\n");
        output.push_str("# TYPE mempool_total_fees_pending gauge\n");
        output.push_str(&format!(
            "mempool_total_fees_pending {}\n",
            self.mempool.total_fees_pending.load(Ordering::Relaxed)
        ));

        let oldest_ts = self.mempool.oldest_pending_timestamp.load(Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let oldest_age = if now >= oldest_ts { now - oldest_ts } else { 0 };

        output.push_str("# HELP mempool_oldest_pending_age_seconds Age of oldest pending transaction (seconds)\n");
        output.push_str("# TYPE mempool_oldest_pending_age_seconds gauge\n");
        output.push_str(&format!(
            "mempool_oldest_pending_age_seconds {}\n",
            oldest_age
        ));

        // Network metrics
        output.push_str("# HELP network_connected_peers Number of connected peers\n");
        output.push_str("# TYPE network_connected_peers gauge\n");
        output.push_str(&format!(
            "network_connected_peers {}\n",
            self.network.connected_peers.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP network_messages_received_total Total P2P messages received\n");
        output.push_str("# TYPE network_messages_received_total counter\n");
        output.push_str(&format!(
            "network_messages_received_total {}\n",
            self.network.messages_received.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP network_messages_sent_total Total P2P messages sent\n");
        output.push_str("# TYPE network_messages_sent_total counter\n");
        output.push_str(&format!(
            "network_messages_sent_total {}\n",
            self.network.messages_sent.load(Ordering::Relaxed)
        ));

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
