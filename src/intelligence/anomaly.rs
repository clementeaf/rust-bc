//! Anomaly detection engine — identifies unusual patterns in transactions and network behavior.
//!
//! Uses statistical methods (z-score, IQR) and configurable rules to flag anomalies
//! without external ML dependencies.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// An observed data point for time-series analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: u64,
    pub value: f64,
    pub source: String,
}

/// A detected anomaly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: u64,
    pub source: String,
    pub value: f64,
    pub expected_range: (f64, f64),
    pub severity: AnomalySeverity,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Configuration for the anomaly detector.
#[derive(Debug, Clone)]
pub struct AnomalyConfig {
    /// Z-score threshold for flagging (default: 3.0 = 99.7% confidence).
    pub z_threshold: f64,
    /// Minimum data points before detection activates.
    pub min_samples: usize,
    /// Rolling window size for statistics.
    pub window_size: usize,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            z_threshold: 3.0,
            min_samples: 10,
            window_size: 100,
        }
    }
}

/// Statistical anomaly detector using rolling z-score.
pub struct AnomalyDetector {
    config: AnomalyConfig,
    window: VecDeque<f64>,
    anomalies: Vec<Anomaly>,
}

impl AnomalyDetector {
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            window: VecDeque::with_capacity(config.window_size),
            config,
            anomalies: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AnomalyConfig::default())
    }

    /// Feed a data point and check for anomaly.
    pub fn observe(&mut self, point: &DataPoint) -> Option<Anomaly> {
        // Add to window
        if self.window.len() >= self.config.window_size {
            self.window.pop_front();
        }
        self.window.push_back(point.value);

        // Need minimum samples
        if self.window.len() < self.config.min_samples {
            return None;
        }

        let mean = self.mean();
        let std_dev = self.std_dev(mean);

        // Avoid division by zero
        if std_dev < f64::EPSILON {
            return None;
        }

        let z_score = (point.value - mean).abs() / std_dev;

        if z_score > self.config.z_threshold {
            let low = mean - self.config.z_threshold * std_dev;
            let high = mean + self.config.z_threshold * std_dev;

            let severity = if z_score > 5.0 {
                AnomalySeverity::Critical
            } else if z_score > 4.0 {
                AnomalySeverity::High
            } else if z_score > 3.5 {
                AnomalySeverity::Medium
            } else {
                AnomalySeverity::Low
            };

            let anomaly = Anomaly {
                timestamp: point.timestamp,
                source: point.source.clone(),
                value: point.value,
                expected_range: (low, high),
                severity,
                reason: format!(
                    "z-score {z_score:.2} exceeds threshold {:.1}",
                    self.config.z_threshold
                ),
            };

            self.anomalies.push(anomaly.clone());
            Some(anomaly)
        } else {
            None
        }
    }

    /// Get all detected anomalies.
    pub fn anomalies(&self) -> &[Anomaly] {
        &self.anomalies
    }

    /// Current window mean.
    fn mean(&self) -> f64 {
        if self.window.is_empty() {
            return 0.0;
        }
        self.window.iter().sum::<f64>() / self.window.len() as f64
    }

    /// Current window standard deviation.
    fn std_dev(&self, mean: f64) -> f64 {
        if self.window.len() < 2 {
            return 0.0;
        }
        let variance = self.window.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / (self.window.len() - 1) as f64;
        variance.sqrt()
    }

    /// Current statistics snapshot.
    pub fn stats(&self) -> DetectorStats {
        let mean = self.mean();
        DetectorStats {
            samples: self.window.len(),
            mean,
            std_dev: self.std_dev(mean),
            anomaly_count: self.anomalies.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorStats {
    pub samples: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub anomaly_count: usize,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_point(value: f64, ts: u64) -> DataPoint {
        DataPoint {
            timestamp: ts,
            value,
            source: "test".into(),
        }
    }

    #[test]
    fn no_anomaly_within_normal_range() {
        let mut det = AnomalyDetector::with_defaults();
        // Feed 20 normal points around 100
        for i in 0..20 {
            let v = 100.0 + (i as f64 % 5.0) - 2.0;
            assert!(det.observe(&make_point(v, i)).is_none());
        }
    }

    #[test]
    fn detects_spike() {
        let mut det = AnomalyDetector::new(AnomalyConfig {
            z_threshold: 2.0,
            min_samples: 10,
            window_size: 50,
        });

        // Feed stable data
        for i in 0..20 {
            det.observe(&make_point(100.0, i));
        }

        // Spike
        let result = det.observe(&make_point(999.0, 20));
        assert!(result.is_some());
        let a = result.unwrap();
        assert!(a.value > a.expected_range.1);
    }

    #[test]
    fn no_detection_below_min_samples() {
        let mut det = AnomalyDetector::with_defaults();
        // Only 5 points (min is 10)
        for i in 0..5 {
            assert!(det.observe(&make_point(100.0, i)).is_none());
        }
        // Even a spike shouldn't trigger
        assert!(det.observe(&make_point(999.0, 5)).is_none());
    }

    #[test]
    fn severity_scales_with_z_score() {
        let mut det = AnomalyDetector::new(AnomalyConfig {
            z_threshold: 2.0,
            min_samples: 10,
            window_size: 50,
        });

        for i in 0..20 {
            det.observe(&make_point(100.0, i));
        }

        // Moderate spike
        let r1 = det.observe(&make_point(200.0, 20));
        // Extreme spike
        let r2 = det.observe(&make_point(9999.0, 21));

        assert!(r1.is_some());
        assert!(r2.is_some());

        // r2 should be higher severity
        let s1 = r1.unwrap().severity;
        let s2 = r2.unwrap().severity;
        assert!(s2 as u8 >= s1 as u8);
    }

    #[test]
    fn stats_reflect_window() {
        let mut det = AnomalyDetector::with_defaults();
        for i in 0..15 {
            det.observe(&make_point(10.0 * (i as f64 + 1.0), i));
        }
        let s = det.stats();
        assert_eq!(s.samples, 15);
        assert!(s.mean > 0.0);
        assert!(s.std_dev > 0.0);
    }

    #[test]
    fn window_rolls_over() {
        let mut det = AnomalyDetector::new(AnomalyConfig {
            z_threshold: 3.0,
            min_samples: 5,
            window_size: 10,
        });

        for i in 0..20 {
            det.observe(&make_point(100.0, i));
        }

        assert_eq!(det.stats().samples, 10); // Window capped at 10
    }

    #[test]
    fn anomaly_serde_roundtrip() {
        let a = Anomaly {
            timestamp: 1000,
            source: "tx".into(),
            value: 999.0,
            expected_range: (90.0, 110.0),
            severity: AnomalySeverity::High,
            reason: "z-score 5.2".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let restored: Anomaly = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.severity, AnomalySeverity::High);
    }
}
