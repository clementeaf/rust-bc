//! Pattern recognition — identifies behavioral patterns across transactions and events.
//!
//! Detects:
//! - Velocity patterns (sudden burst of activity)
//! - Structuring (splitting to avoid thresholds)
//! - Round-trip transfers (A→B→A)
//! - Dormant account activation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A transaction record for pattern analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx_id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: u64,
}

/// Detected pattern type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    /// Sudden burst of transactions from one identity.
    VelocitySpike {
        identity: String,
        count: usize,
        window_secs: u64,
    },
    /// Multiple transactions just below a threshold (structuring/smurfing).
    Structuring {
        identity: String,
        count: usize,
        threshold: u64,
    },
    /// Funds sent A→B→A (round-trip).
    RoundTrip { a: String, b: String, amount: u64 },
    /// Account inactive for a long period, then sudden activity.
    DormantActivation { identity: String, dormant_days: u64 },
}

/// A detected pattern with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern: PatternType,
    pub confidence: f64,
    pub tx_ids: Vec<String>,
    pub detected_at: u64,
}

/// Pattern detection engine.
pub struct PatternEngine {
    /// Velocity: max transactions per window before flagging.
    pub velocity_threshold: usize,
    pub velocity_window_secs: u64,
    /// Structuring: flag if N transactions are within X% below threshold.
    pub structuring_threshold: u64,
    pub structuring_margin_pct: u64,
    pub structuring_min_count: usize,
    /// Dormancy: days of inactivity before activation is suspicious.
    pub dormancy_days: u64,
}

impl Default for PatternEngine {
    fn default() -> Self {
        Self {
            velocity_threshold: 20,
            velocity_window_secs: 3600,
            structuring_threshold: 100_000,
            structuring_margin_pct: 10,
            structuring_min_count: 3,
            dormancy_days: 90,
        }
    }
}

impl PatternEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a batch of transactions for patterns.
    pub fn analyze(&self, transactions: &[TxRecord]) -> Vec<DetectedPattern> {
        let mut results = Vec::new();

        results.extend(self.detect_velocity(transactions));
        results.extend(self.detect_structuring(transactions));
        results.extend(self.detect_round_trips(transactions));
        results.extend(self.detect_dormant_activation(transactions));

        results
    }

    /// Detect velocity spikes per identity.
    fn detect_velocity(&self, txs: &[TxRecord]) -> Vec<DetectedPattern> {
        let mut by_sender: HashMap<&str, Vec<&TxRecord>> = HashMap::new();
        for tx in txs {
            by_sender.entry(&tx.from).or_default().push(tx);
        }

        let mut results = Vec::new();
        for (identity, sender_txs) in &by_sender {
            if sender_txs.len() < self.velocity_threshold {
                continue;
            }

            // Check if they cluster within the window
            let mut sorted: Vec<u64> = sender_txs.iter().map(|t| t.timestamp).collect();
            sorted.sort_unstable();

            for window_start_idx in 0..sorted.len() {
                let window_end = sorted[window_start_idx] + self.velocity_window_secs;
                let count = sorted[window_start_idx..]
                    .iter()
                    .take_while(|&&ts| ts <= window_end)
                    .count();

                if count >= self.velocity_threshold {
                    results.push(DetectedPattern {
                        pattern: PatternType::VelocitySpike {
                            identity: identity.to_string(),
                            count,
                            window_secs: self.velocity_window_secs,
                        },
                        confidence: (count as f64 / self.velocity_threshold as f64).min(1.0),
                        tx_ids: sender_txs.iter().map(|t| t.tx_id.clone()).collect(),
                        detected_at: sorted.last().copied().unwrap_or(0),
                    });
                    break; // One per identity
                }
            }
        }

        results
    }

    /// Detect structuring (amounts just below threshold).
    fn detect_structuring(&self, txs: &[TxRecord]) -> Vec<DetectedPattern> {
        let margin = self.structuring_threshold * self.structuring_margin_pct / 100;
        let lower_bound = self.structuring_threshold - margin;

        let mut by_sender: HashMap<&str, Vec<&TxRecord>> = HashMap::new();
        for tx in txs {
            if tx.amount >= lower_bound && tx.amount < self.structuring_threshold {
                by_sender.entry(&tx.from).or_default().push(tx);
            }
        }

        let mut results = Vec::new();
        for (identity, suspicious_txs) in &by_sender {
            if suspicious_txs.len() >= self.structuring_min_count {
                results.push(DetectedPattern {
                    pattern: PatternType::Structuring {
                        identity: identity.to_string(),
                        count: suspicious_txs.len(),
                        threshold: self.structuring_threshold,
                    },
                    confidence: (suspicious_txs.len() as f64
                        / (self.structuring_min_count as f64 + 2.0))
                        .min(1.0),
                    tx_ids: suspicious_txs.iter().map(|t| t.tx_id.clone()).collect(),
                    detected_at: suspicious_txs.last().map(|t| t.timestamp).unwrap_or(0),
                });
            }
        }

        results
    }

    /// Detect round-trip transfers (A→B then B→A with same amount).
    fn detect_round_trips(&self, txs: &[TxRecord]) -> Vec<DetectedPattern> {
        let mut results = Vec::new();
        let mut matched = std::collections::HashSet::new();

        for (i, fwd) in txs.iter().enumerate() {
            for rev in &txs[i + 1..] {
                if rev.from == fwd.to
                    && rev.to == fwd.from
                    && rev.amount == fwd.amount
                    && rev.timestamp > fwd.timestamp
                    && !matched.contains(&fwd.tx_id)
                {
                    matched.insert(fwd.tx_id.clone());
                    results.push(DetectedPattern {
                        pattern: PatternType::RoundTrip {
                            a: fwd.from.clone(),
                            b: fwd.to.clone(),
                            amount: fwd.amount,
                        },
                        confidence: 0.9,
                        tx_ids: vec![fwd.tx_id.clone(), rev.tx_id.clone()],
                        detected_at: rev.timestamp,
                    });
                }
            }
        }

        results
    }

    /// Detect dormant account activation.
    fn detect_dormant_activation(&self, txs: &[TxRecord]) -> Vec<DetectedPattern> {
        let mut by_sender: HashMap<&str, Vec<u64>> = HashMap::new();
        for tx in txs {
            by_sender.entry(&tx.from).or_default().push(tx.timestamp);
        }

        let mut results = Vec::new();
        let dormancy_secs = self.dormancy_days * 86400;

        for (identity, timestamps) in &by_sender {
            if timestamps.len() < 2 {
                continue;
            }
            let mut sorted = timestamps.clone();
            sorted.sort_unstable();

            for i in 1..sorted.len() {
                let gap = sorted[i] - sorted[i - 1];
                if gap >= dormancy_secs {
                    results.push(DetectedPattern {
                        pattern: PatternType::DormantActivation {
                            identity: identity.to_string(),
                            dormant_days: gap / 86400,
                        },
                        confidence: (gap as f64 / dormancy_secs as f64).min(1.0),
                        tx_ids: vec![],
                        detected_at: sorted[i],
                    });
                    break; // One per identity
                }
            }
        }

        results
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(id: &str, from: &str, to: &str, amount: u64, ts: u64) -> TxRecord {
        TxRecord {
            tx_id: id.into(),
            from: from.into(),
            to: to.into(),
            amount,
            timestamp: ts,
        }
    }

    #[test]
    fn no_patterns_in_clean_data() {
        let engine = PatternEngine::new();
        let txs = vec![
            tx("1", "alice", "bob", 1000, 100),
            tx("2", "bob", "charlie", 2000, 200),
        ];
        assert!(engine.analyze(&txs).is_empty());
    }

    #[test]
    fn detects_velocity_spike() {
        let mut engine = PatternEngine::new();
        engine.velocity_threshold = 5;
        engine.velocity_window_secs = 60;

        let txs: Vec<TxRecord> = (0..10)
            .map(|i| tx(&format!("t{i}"), "alice", "bob", 100, 10 + i))
            .collect();

        let results = engine.analyze(&txs);
        assert!(results
            .iter()
            .any(|r| matches!(r.pattern, PatternType::VelocitySpike { .. })));
    }

    #[test]
    fn detects_structuring() {
        let mut engine = PatternEngine::new();
        engine.structuring_threshold = 10_000;
        engine.structuring_margin_pct = 10;
        engine.structuring_min_count = 3;

        let txs = vec![
            tx("1", "alice", "bob", 9500, 100),
            tx("2", "alice", "charlie", 9800, 200),
            tx("3", "alice", "dave", 9200, 300),
        ];

        let results = engine.analyze(&txs);
        assert!(results
            .iter()
            .any(|r| matches!(r.pattern, PatternType::Structuring { .. })));
    }

    #[test]
    fn detects_round_trip() {
        let engine = PatternEngine::new();
        let txs = vec![
            tx("1", "alice", "bob", 5000, 100),
            tx("2", "bob", "alice", 5000, 200),
        ];

        let results = engine.analyze(&txs);
        assert!(results
            .iter()
            .any(|r| matches!(r.pattern, PatternType::RoundTrip { .. })));
    }

    #[test]
    fn no_round_trip_different_amounts() {
        let engine = PatternEngine::new();
        let txs = vec![
            tx("1", "alice", "bob", 5000, 100),
            tx("2", "bob", "alice", 3000, 200),
        ];

        let results = engine.analyze(&txs);
        assert!(!results
            .iter()
            .any(|r| matches!(r.pattern, PatternType::RoundTrip { .. })));
    }

    #[test]
    fn detects_dormant_activation() {
        let mut engine = PatternEngine::new();
        engine.dormancy_days = 30;

        let txs = vec![
            tx("1", "alice", "bob", 100, 1000),
            tx("2", "alice", "bob", 100, 1000 + 31 * 86400), // 31 days later
        ];

        let results = engine.analyze(&txs);
        assert!(results
            .iter()
            .any(|r| matches!(r.pattern, PatternType::DormantActivation { .. })));
    }

    #[test]
    fn pattern_serde_roundtrip() {
        let p = DetectedPattern {
            pattern: PatternType::RoundTrip {
                a: "alice".into(),
                b: "bob".into(),
                amount: 5000,
            },
            confidence: 0.9,
            tx_ids: vec!["t1".into(), "t2".into()],
            detected_at: 200,
        };
        let json = serde_json::to_string(&p).unwrap();
        let restored: DetectedPattern = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.pattern, p.pattern);
    }
}
