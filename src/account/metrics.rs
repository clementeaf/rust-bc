//! Cryptocurrency observability — counters and gauges for native tx layer.

use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe counters for cryptocurrency operations.
pub struct CryptoMetrics {
    /// Total native transfers executed (successful).
    pub transfers_total: AtomicU64,
    /// Total failed transfers.
    pub transfers_failed: AtomicU64,
    /// Total rejected signatures at mempool admission.
    pub rejected_signatures: AtomicU64,
    /// Total blocks produced.
    pub blocks_produced: AtomicU64,
    /// Total fees burned.
    pub fees_burned: AtomicU64,
    /// Total fees paid to proposers.
    pub fees_to_proposers: AtomicU64,
    /// Total block rewards minted.
    pub rewards_minted: AtomicU64,
}

impl CryptoMetrics {
    pub fn new() -> Self {
        Self {
            transfers_total: AtomicU64::new(0),
            transfers_failed: AtomicU64::new(0),
            rejected_signatures: AtomicU64::new(0),
            blocks_produced: AtomicU64::new(0),
            fees_burned: AtomicU64::new(0),
            fees_to_proposers: AtomicU64::new(0),
            rewards_minted: AtomicU64::new(0),
        }
    }

    pub fn inc_transfers(&self) {
        self.transfers_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_failed(&self) {
        self.transfers_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rejected_sigs(&self) {
        self.rejected_signatures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_blocks(&self) {
        self.blocks_produced.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_burned(&self, amount: u64) {
        self.fees_burned.fetch_add(amount, Ordering::Relaxed);
    }

    pub fn add_proposer_fees(&self, amount: u64) {
        self.fees_to_proposers.fetch_add(amount, Ordering::Relaxed);
    }

    pub fn add_rewards(&self, amount: u64) {
        self.rewards_minted.fetch_add(amount, Ordering::Relaxed);
    }

    /// Snapshot of all metrics as a serializable struct.
    pub fn snapshot(&self) -> CryptoMetricsSnapshot {
        CryptoMetricsSnapshot {
            transfers_total: self.transfers_total.load(Ordering::Relaxed),
            transfers_failed: self.transfers_failed.load(Ordering::Relaxed),
            rejected_signatures: self.rejected_signatures.load(Ordering::Relaxed),
            blocks_produced: self.blocks_produced.load(Ordering::Relaxed),
            fees_burned: self.fees_burned.load(Ordering::Relaxed),
            fees_to_proposers: self.fees_to_proposers.load(Ordering::Relaxed),
            rewards_minted: self.rewards_minted.load(Ordering::Relaxed),
        }
    }
}

impl Default for CryptoMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable snapshot of crypto metrics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoMetricsSnapshot {
    pub transfers_total: u64,
    pub transfers_failed: u64,
    pub rejected_signatures: u64,
    pub blocks_produced: u64,
    pub fees_burned: u64,
    pub fees_to_proposers: u64,
    pub rewards_minted: u64,
}

/// Health check response for the crypto layer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CryptoHealth {
    pub status: String,
    pub height: u64,
    pub mempool_pending: usize,
    pub base_fee: u64,
    pub total_accounts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_increment() {
        let m = CryptoMetrics::new();
        m.inc_transfers();
        m.inc_transfers();
        m.inc_failed();
        m.inc_blocks();
        m.add_burned(100);
        m.add_proposer_fees(20);
        m.add_rewards(50);

        let snap = m.snapshot();
        assert_eq!(snap.transfers_total, 2);
        assert_eq!(snap.transfers_failed, 1);
        assert_eq!(snap.blocks_produced, 1);
        assert_eq!(snap.fees_burned, 100);
        assert_eq!(snap.fees_to_proposers, 20);
        assert_eq!(snap.rewards_minted, 50);
    }

    #[test]
    fn metrics_thread_safe() {
        let m = std::sync::Arc::new(CryptoMetrics::new());
        std::thread::scope(|s| {
            for _ in 0..10 {
                let m = m.clone();
                s.spawn(move || {
                    for _ in 0..100 {
                        m.inc_transfers();
                    }
                });
            }
        });
        assert_eq!(m.snapshot().transfers_total, 1000);
    }

    #[test]
    fn snapshot_serializable() {
        let m = CryptoMetrics::new();
        m.inc_transfers();
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("transfers_total"));
    }
}
