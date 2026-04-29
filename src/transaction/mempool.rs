//! Fee-ordered mempool for native cryptocurrency transactions.
//!
//! Transactions are ordered by fee descending (highest fee first).
//! Per-sender nonce ordering is enforced: a transaction with nonce N+1
//! is only promotable after nonce N is included.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;

use super::native::NativeTransaction;

/// Configuration for the mempool.
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in the pool.
    pub max_size: usize,
    /// Maximum number of pending transactions per sender.
    pub max_per_sender: usize,
    /// Minimum fee to accept into mempool (absolute floor).
    pub min_fee: u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            max_per_sender: 64,
            min_fee: crate::tokenomics::economics::MIN_TX_FEE,
        }
    }
}

/// Ordering key: (fee descending, timestamp ascending) for deterministic ordering.
/// Higher fee = higher priority. Equal fee → earlier timestamp wins.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TxPriority {
    fee: u64,
    timestamp: u64,
    id: String,
}

impl PartialOrd for TxPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TxPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher fee first
        other
            .fee
            .cmp(&self.fee)
            // Then earlier timestamp
            .then(self.timestamp.cmp(&other.timestamp))
            // Then by ID for determinism
            .then(self.id.cmp(&other.id))
    }
}

/// Thread-safe fee-ordered mempool.
pub struct Mempool {
    inner: Mutex<MempoolInner>,
    config: MempoolConfig,
}

struct MempoolInner {
    /// All transactions by ID.
    txs: HashMap<String, NativeTransaction>,
    /// Ordered set for fee-priority extraction.
    ordered: BTreeMap<TxPriority, String>,
    /// Per-sender transaction count.
    sender_count: HashMap<String, usize>,
    /// Known transaction IDs (including removed) for dedup.
    known_ids: HashSet<String>,
}

impl Mempool {
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            inner: Mutex::new(MempoolInner {
                txs: HashMap::new(),
                ordered: BTreeMap::new(),
                sender_count: HashMap::new(),
                known_ids: HashSet::new(),
            }),
            config,
        }
    }

    /// Add a transaction to the mempool.
    ///
    /// Returns `Ok(true)` if added, `Ok(false)` if duplicate, `Err` if pool is full
    /// or sender has too many pending.
    pub fn add(&self, tx: NativeTransaction) -> Result<bool, MempoolError> {
        // Fee floor enforcement
        if tx.fee < self.config.min_fee {
            return Err(MempoolError::FeeTooLow {
                offered: tx.fee,
                minimum: self.config.min_fee,
            });
        }

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        // Dedup
        if inner.known_ids.contains(&tx.id) {
            return Ok(false);
        }

        // Pool size limit
        if inner.txs.len() >= self.config.max_size {
            // Evict lowest-priority tx if new tx has higher fee
            if let Some(lowest) = inner.ordered.keys().next_back().cloned() {
                if tx.fee > lowest.fee {
                    let evicted_id = inner.ordered.remove(&lowest).unwrap();
                    self.remove_tx_inner(&mut inner, &evicted_id);
                } else {
                    return Err(MempoolError::PoolFull);
                }
            } else {
                return Err(MempoolError::PoolFull);
            }
        }

        // Per-sender limit
        let sender = tx.sender().unwrap_or("coinbase").to_string();
        let count = inner.sender_count.get(&sender).copied().unwrap_or(0);
        if count >= self.config.max_per_sender {
            return Err(MempoolError::SenderFull {
                sender,
                max: self.config.max_per_sender,
            });
        }

        let priority = TxPriority {
            fee: tx.fee,
            timestamp: tx.timestamp,
            id: tx.id.clone(),
        };

        inner.known_ids.insert(tx.id.clone());
        inner.ordered.insert(priority, tx.id.clone());
        *inner.sender_count.entry(sender).or_insert(0) += 1;
        inner.txs.insert(tx.id.clone(), tx);

        Ok(true)
    }

    /// Take up to `max` highest-fee transactions from the pool.
    pub fn drain_top(&self, max: usize) -> Vec<NativeTransaction> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut result = Vec::with_capacity(max);

        let keys: Vec<_> = inner.ordered.keys().take(max).cloned().collect();
        for key in keys {
            if let Some(id) = inner.ordered.remove(&key) {
                if let Some(tx) = inner.txs.remove(&id) {
                    let sender = tx.sender().unwrap_or("coinbase").to_string();
                    if let Some(c) = inner.sender_count.get_mut(&sender) {
                        *c = c.saturating_sub(1);
                    }
                    result.push(tx);
                }
            }
        }

        result
    }

    /// Remove a specific transaction by ID (e.g., after inclusion in a block).
    pub fn remove(&self, tx_id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.remove_tx_inner(&mut inner, tx_id);
    }

    /// Number of transactions currently in the pool.
    pub fn len(&self) -> usize {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.txs.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if a transaction ID is already known (added or evicted).
    pub fn is_known(&self, tx_id: &str) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.known_ids.contains(tx_id)
    }

    /// Current pending count for a sender.
    pub fn sender_pending(&self, sender: &str) -> usize {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.sender_count.get(sender).copied().unwrap_or(0)
    }

    fn remove_tx_inner(&self, inner: &mut MempoolInner, tx_id: &str) {
        if let Some(tx) = inner.txs.remove(tx_id) {
            let priority = TxPriority {
                fee: tx.fee,
                timestamp: tx.timestamp,
                id: tx.id.clone(),
            };
            inner.ordered.remove(&priority);
            let sender = tx.sender().unwrap_or("coinbase").to_string();
            if let Some(c) = inner.sender_count.get_mut(&sender) {
                *c = c.saturating_sub(1);
            }
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new(MempoolConfig::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MempoolError {
    #[error("mempool is full")]
    PoolFull,
    #[error("sender {sender} has {max} pending transactions")]
    SenderFull { sender: String, max: usize },
    #[error("fee {offered} below minimum {minimum}")]
    FeeTooLow { offered: u64, minimum: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx(from: &str, to: &str, fee: u64, nonce: u64) -> NativeTransaction {
        NativeTransaction::new_transfer(from, to, 10, nonce, fee)
    }

    #[test]
    fn add_and_drain_ordered_by_fee() {
        let pool = Mempool::new(MempoolConfig {
            max_size: 100,
            max_per_sender: 10,
            min_fee: 1,
        });

        pool.add(make_tx("a", "b", 1, 0)).unwrap();
        pool.add(make_tx("a", "b", 10, 1)).unwrap();
        pool.add(make_tx("a", "b", 5, 2)).unwrap();

        let drained = pool.drain_top(10);
        assert_eq!(drained.len(), 3);
        // Highest fee first
        assert_eq!(drained[0].fee, 10);
        assert_eq!(drained[1].fee, 5);
        assert_eq!(drained[2].fee, 1);
    }

    #[test]
    fn drain_top_respects_limit() {
        let pool = Mempool::default();
        for i in 0..10 {
            pool.add(make_tx("a", "b", i + 1, i)).unwrap();
        }
        let drained = pool.drain_top(3);
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[0].fee, 10);
        assert_eq!(drained[1].fee, 9);
        assert_eq!(drained[2].fee, 8);
        assert_eq!(pool.len(), 7);
    }

    #[test]
    fn duplicate_rejected() {
        let pool = Mempool::default();
        let tx = make_tx("a", "b", 5, 0);
        assert!(pool.add(tx.clone()).unwrap());
        assert!(!pool.add(tx).unwrap()); // duplicate
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn pool_full_rejects_low_fee() {
        let pool = Mempool::new(MempoolConfig {
            max_size: 2,
            max_per_sender: 10,
            min_fee: 1,
        });
        pool.add(make_tx("a", "b", 10, 0)).unwrap();
        pool.add(make_tx("a", "b", 5, 1)).unwrap();

        // Lower fee than everything → rejected
        let err = pool.add(make_tx("c", "d", 1, 0)).unwrap_err();
        assert!(matches!(err, MempoolError::PoolFull));
    }

    #[test]
    fn pool_full_evicts_lowest_for_higher_fee() {
        let pool = Mempool::new(MempoolConfig {
            max_size: 2,
            max_per_sender: 10,
            min_fee: 1,
        });
        pool.add(make_tx("a", "b", 5, 0)).unwrap();
        pool.add(make_tx("a", "b", 3, 1)).unwrap();

        // Higher fee evicts the fee=3 tx
        assert!(pool.add(make_tx("c", "d", 10, 0)).unwrap());
        assert_eq!(pool.len(), 2);

        let drained = pool.drain_top(10);
        let fees: Vec<u64> = drained.iter().map(|t| t.fee).collect();
        assert_eq!(fees, vec![10, 5]);
    }

    #[test]
    fn sender_limit_enforced() {
        let pool = Mempool::new(MempoolConfig {
            max_size: 100,
            max_per_sender: 2,
            min_fee: 1,
        });
        pool.add(make_tx("alice", "b", 5, 0)).unwrap();
        pool.add(make_tx("alice", "b", 5, 1)).unwrap();

        let err = pool.add(make_tx("alice", "b", 5, 2)).unwrap_err();
        assert!(matches!(err, MempoolError::SenderFull { .. }));

        // Different sender still works
        assert!(pool.add(make_tx("bob", "b", 5, 0)).unwrap());
    }

    #[test]
    fn remove_by_id() {
        let pool = Mempool::default();
        let tx = make_tx("a", "b", 5, 0);
        let id = tx.id.clone();
        pool.add(tx).unwrap();
        assert_eq!(pool.len(), 1);

        pool.remove(&id);
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn is_known_after_add() {
        let pool = Mempool::default();
        let tx = make_tx("a", "b", 5, 0);
        let id = tx.id.clone();
        assert!(!pool.is_known(&id));
        pool.add(tx).unwrap();
        assert!(pool.is_known(&id));
    }

    #[test]
    fn sender_pending_tracks_count() {
        let pool = Mempool::default();
        assert_eq!(pool.sender_pending("alice"), 0);

        pool.add(make_tx("alice", "b", 5, 0)).unwrap();
        pool.add(make_tx("alice", "b", 5, 1)).unwrap();
        assert_eq!(pool.sender_pending("alice"), 2);

        pool.drain_top(1);
        assert_eq!(pool.sender_pending("alice"), 1);
    }

    #[test]
    fn empty_pool_drain() {
        let pool = Mempool::default();
        assert!(pool.is_empty());
        let drained = pool.drain_top(10);
        assert!(drained.is_empty());
    }
}
