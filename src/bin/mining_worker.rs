/**
 * Mining Worker Binary
 *
 * Autonomous miner that:
 * - Connects to a running node via HTTP API
 * - Continuously polls mempool for pending transactions
 * - Constructs and mines blocks with fee-prioritized transactions
 * - Submits mined blocks via API
 * - Reports metrics and handles errors gracefully
 *
 * Usage:
 *   cargo run --release --bin mining_worker
 *
 * Configuration (environment variables):
 *   MINER_API_URL: Target node API (default: http://127.0.0.1:8080)
 *   MINER_ADDRESS: Coinbase receiver address (default: MINER)
 *   MINER_POLL_INTERVAL: Mempool poll interval in seconds (default: 5)
 *   MINER_TX_BATCH_SIZE: Max transactions per block (default: 100)
 *   MINER_MIN_BLOCK_INTERVAL: Min seconds between blocks (default: 0)
 *   MINER_WORKER_THREADS: Parallel mining threads (default: CPU count)
 *   RUST_LOG: Logging level (default: info)
 */

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// API Response wrapper
#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

/// Mempool response structure
#[derive(Debug, Deserialize)]
struct MempoolData {
    #[allow(dead_code)]
    count: usize,
    transactions: Vec<MempoolTransaction>,
}

/// Transaction from mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MempoolTransaction {
    id: String,
    from: String,
    to: String,
    amount: u64,
    fee: u64,
    #[serde(default)]
    timestamp: u64,
    #[serde(default)]
    signature: String,
    #[serde(default)]
    data: Option<String>,
}

/// Request to create a block with transactions
#[derive(Debug, Serialize)]
struct CreateBlockRequest {
    transactions: Vec<BlockTransaction>,
}

/// Transaction for block creation
#[derive(Debug, Serialize, Clone)]
struct BlockTransaction {
    from: String,
    to: String,
    amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

/// Block creation response
#[derive(Debug, Deserialize)]
struct BlockResponse {
    hash: String,
    reward: u64,
    transactions_count: usize,
    #[serde(default)]
    #[allow(dead_code)]
    validator: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    consensus: String,
}

/// Mining worker configuration
#[derive(Debug, Clone)]
struct MiningConfig {
    api_url: String,
    miner_address: String,
    poll_interval: u64,
    tx_batch_size: usize,
    min_block_interval: u64,
    worker_threads: usize,
}

impl MiningConfig {
    fn from_env() -> Self {
        MiningConfig {
            api_url: env::var("MINER_API_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()),
            miner_address: env::var("MINER_ADDRESS")
                .unwrap_or_else(|_| "MINER".to_string()),
            poll_interval: env::var("MINER_POLL_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            tx_batch_size: env::var("MINER_TX_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            min_block_interval: env::var("MINER_MIN_BLOCK_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            worker_threads: env::var("MINER_WORKER_THREADS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| num_cpus::get()),
        }
    }
}

/// Mining worker metrics
#[derive(Debug, Clone)]
struct MiningMetrics {
    blocks_mined: Arc<AtomicU64>,
    transactions_mined: Arc<AtomicU64>,
    api_errors: Arc<AtomicU64>,
    start_time: Instant,
}

impl MiningMetrics {
    fn new() -> Self {
        MiningMetrics {
            blocks_mined: Arc::new(AtomicU64::new(0)),
            transactions_mined: Arc::new(AtomicU64::new(0)),
            api_errors: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }

    fn record_block(&self, tx_count: u64) {
        self.blocks_mined.fetch_add(1, Ordering::Relaxed);
        self.transactions_mined.fetch_add(tx_count, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.api_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn print_stats(&self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let blocks = self.blocks_mined.load(Ordering::Relaxed);
        let txs = self.transactions_mined.load(Ordering::Relaxed);
        let errors = self.api_errors.load(Ordering::Relaxed);

        println!("\n📊 Mining Statistics:");
        println!("   Blocks mined: {}", blocks);
        println!("   Transactions: {}", txs);
        println!("   Errors: {}", errors);
        println!("   Uptime: {:.2}s", elapsed);
        if blocks > 0 {
            println!("   Avg block time: {:.2}s", elapsed / blocks as f64);
            println!("   Block rate: {:.4} blocks/s", blocks as f64 / elapsed);
        }
    }
}

/// Mining worker
struct MiningWorker {
    config: MiningConfig,
    client: Client,
    metrics: MiningMetrics,
    running: Arc<AtomicBool>,
    backoff_attempts: u32,
}

impl MiningWorker {
    fn new(config: MiningConfig) -> Self {
        let client = Client::new();
        MiningWorker {
            config,
            client,
            metrics: MiningMetrics::new(),
            running: Arc::new(AtomicBool::new(true)),
            backoff_attempts: 0,
        }
    }

    /// Fetch mempool transactions from API
    async fn fetch_mempool(&self) -> Result<Vec<MempoolTransaction>, String> {
        let url = format!("{}/api/v1/mempool", self.config.api_url);

        match self.client.get(&url).send().await {
            Ok(response) => {
                match response.json::<ApiResponse<MempoolData>>().await {
                    Ok(api_resp) => {
                        if api_resp.success {
                            Ok(api_resp
                                .data
                                .map(|d| d.transactions)
                                .unwrap_or_default())
                        } else {
                            Err(format!(
                                "API error: {}",
                                api_resp.message.unwrap_or_default()
                            ))
                        }
                    }
                    Err(e) => Err(format!("JSON parse error: {}", e)),
                }
            }
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    /// Construct block request with fee-prioritized transactions
    fn construct_block(&self, mut transactions: Vec<MempoolTransaction>) -> CreateBlockRequest {
        // Sort by fee descending (highest fees first)
        transactions.sort_by(|a, b| b.fee.cmp(&a.fee));

        // Cap to batch size
        let tx_count = transactions.len().min(self.config.tx_batch_size);
        let transactions = transactions.into_iter().take(tx_count).collect::<Vec<_>>();

        let block_txs = transactions
            .iter()
            .map(|tx| BlockTransaction {
                from: tx.from.clone(),
                to: tx.to.clone(),
                amount: tx.amount,
                fee: if tx.fee > 0 { Some(tx.fee) } else { None },
                data: tx.data.clone(),
                signature: if tx.signature.is_empty() {
                    None
                } else {
                    Some(tx.signature.clone())
                },
            })
            .collect();

        CreateBlockRequest {
            transactions: block_txs,
        }
    }

    /// Submit block to API for mining
    async fn submit_block(&self, block_request: CreateBlockRequest) -> Result<BlockResponse, String> {
        let url = format!("{}/api/v1/blocks", self.config.api_url);

        match self.client.post(&url).json(&block_request).send().await {
            Ok(response) => {
                match response.json::<ApiResponse<BlockResponse>>().await {
                    Ok(api_resp) => {
                        if api_resp.success {
                            Ok(api_resp.data.ok_or_else(|| "No block data".to_string())?)
                        } else {
                            Err(format!(
                                "API error: {}",
                                api_resp.message.unwrap_or_default()
                            ))
                        }
                    }
                    Err(e) => Err(format!("JSON parse error: {}", e)),
                }
            }
            Err(e) => Err(format!("Request failed: {}", e)),
        }
    }

    /// Calculate exponential backoff delay
    fn backoff_delay(&self) -> Duration {
        let delay_ms = 100 * 2u64.min(self.backoff_attempts as u64);
        Duration::from_millis(delay_ms.min(5000))
    }

    /// Run main mining loop
    async fn run(&mut self) {
        println!("⛏️  Mining Worker Starting");
        println!("   API URL: {}", self.config.api_url);
        println!("   Miner Address: {}", self.config.miner_address);
        println!("   Batch Size: {}", self.config.tx_batch_size);
        println!("   Poll Interval: {} seconds", self.config.poll_interval);
        println!("   Min Block Interval: {} seconds", self.config.min_block_interval);
        println!("   Worker Threads: {}", self.config.worker_threads);
        println!();

        let mut last_block_time = Instant::now();

        while self.running.load(Ordering::Relaxed) {
            match self.fetch_mempool().await {
                Ok(txs) => {
                    if txs.is_empty() {
                        log::debug!("Mempool empty, waiting...");
                    } else {
                        // Check minimum block interval
                        let time_since_last = last_block_time.elapsed().as_secs();
                        if time_since_last < self.config.min_block_interval {
                            let wait_time = self.config.min_block_interval - time_since_last;
                            log::debug!(
                                "Min block interval not met, waiting {}s",
                                wait_time
                            );
                            sleep(Duration::from_secs(wait_time)).await;
                        }

                        let block_req = self.construct_block(txs.clone());
                        let tx_count = block_req.transactions.len() as u64;

                        match self.submit_block(block_req).await {
                            Ok(response) => {
                                println!(
                                    "✅ Block mined! Hash: {} | TXs: {} | Reward: {}",
                                    &response.hash[..16.min(response.hash.len())],
                                    response.transactions_count,
                                    response.reward
                                );
                                self.metrics.record_block(tx_count);
                                self.backoff_attempts = 0;
                                last_block_time = Instant::now();
                            }
                            Err(e) => {
                                eprintln!("❌ Block submission failed: {}", e);
                                self.metrics.record_error();
                                self.backoff_attempts += 1;
                                let delay = self.backoff_delay();
                                sleep(delay).await;
                            }
                        }
                    }

                    sleep(Duration::from_secs(self.config.poll_interval)).await;
                }
                Err(e) => {
                    eprintln!("⚠️  Mempool fetch failed: {}", e);
                    self.metrics.record_error();
                    self.backoff_attempts += 1;
                    let delay = self.backoff_delay();
                    eprintln!("   Retrying in {:?}...", delay);
                    sleep(delay).await;
                }
            }
        }

        println!("\n🛑 Mining Worker Stopping");
        self.metrics.print_stats();
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .init();

    let config = MiningConfig::from_env();
    let mut worker = MiningWorker::new(config);

    // Setup signal handling for graceful shutdown
    let running = worker.running.clone();
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl-C: {}", e);
        } else {
            println!("\n⏸️  Shutdown signal received...");
            running.store(false, Ordering::Relaxed);
        }
    });

    worker.run().await;
}
