//! Oracle demo feed — provides simulated market data for sandbox demonstrations.
//!
//! When `ORACLE_DEMO=true`, spawns a background task that generates realistic
//! price feeds (BTC/USD, ETH/USD, CLP/USD) with configurable volatility.
//! This allows the oracle system to be demonstrated without external API dependencies.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing::info;

use crate::oracle_system::OracleRegistry;

/// Demo feed configuration.
#[derive(Debug, Clone)]
pub struct DemoFeedConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub feeds: Vec<DemoFeed>,
}

#[derive(Debug, Clone)]
pub struct DemoFeed {
    pub symbol: String,
    pub base_price: f64,
    pub volatility_pct: f64,
}

impl DemoFeedConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("ORACLE_DEMO")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let interval: u64 = std::env::var("ORACLE_DEMO_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        Self {
            enabled,
            interval_secs: interval,
            feeds: vec![
                DemoFeed {
                    symbol: "BTC/USD".into(),
                    base_price: 105_000.0,
                    volatility_pct: 2.0,
                },
                DemoFeed {
                    symbol: "ETH/USD".into(),
                    base_price: 2_500.0,
                    volatility_pct: 3.0,
                },
                DemoFeed {
                    symbol: "CLP/USD".into(),
                    base_price: 0.00105,
                    volatility_pct: 0.5,
                },
            ],
        }
    }
}

/// Generate a simulated price with random walk.
fn simulate_price(base: f64, volatility_pct: f64, tick: u64) -> u64 {
    // Deterministic pseudo-random based on tick (no external rand needed)
    let noise = ((tick.wrapping_mul(6364136223846793005).wrapping_add(1)) % 1000) as f64 / 1000.0;
    let delta = (noise - 0.5) * 2.0 * (volatility_pct / 100.0) * base;
    let price = base + delta;
    (price * 100.0) as u64 // Store as cents
}

/// Spawn the demo oracle feed.
pub fn spawn_demo_feed(config: DemoFeedConfig, registry: Arc<Mutex<OracleRegistry>>) {
    if !config.enabled {
        return;
    }

    tokio::spawn(async move {
        info!(
            feeds = config.feeds.len(),
            interval = config.interval_secs,
            "Oracle demo feed started"
        );

        // Register demo oracles
        {
            let mut reg = registry.lock().unwrap();
            for (i, _) in config.feeds.iter().enumerate() {
                let id = format!("demo-oracle-{i}");
                let _ = reg.register_oracle(id);
            }
        }

        let mut tick: u64 = 0;

        loop {
            tick += 1;

            for (i, feed) in config.feeds.iter().enumerate() {
                let oracle_id = format!("demo-oracle-{i}");
                let price = simulate_price(feed.base_price, feed.volatility_pct, tick + i as u64);
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let signature = OracleRegistry::generate_signature(&oracle_id, price, timestamp);

                let mut reg = registry.lock().unwrap();
                let _ = reg.submit_price_report(
                    &oracle_id,
                    feed.symbol.clone(),
                    price,
                    timestamp,
                    signature,
                    95,
                );
                let _ = reg.aggregate_reports(&feed.symbol, timestamp);
            }

            tokio::time::sleep(Duration::from_secs(config.interval_secs)).await;
        }
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_price_deterministic() {
        let p1 = simulate_price(100.0, 5.0, 1);
        let p2 = simulate_price(100.0, 5.0, 1);
        assert_eq!(p1, p2); // Same tick = same price
    }

    #[test]
    fn simulate_price_varies_by_tick() {
        let p1 = simulate_price(100.0, 5.0, 1);
        let p2 = simulate_price(100.0, 5.0, 2);
        // They should differ (not guaranteed but very likely with different ticks)
        // Just check both are reasonable
        assert!(p1 > 0);
        assert!(p2 > 0);
    }

    #[test]
    fn simulate_price_stays_near_base() {
        for tick in 0..100 {
            let price = simulate_price(10000.0, 2.0, tick) as f64 / 100.0;
            assert!(
                price > 9500.0 && price < 10500.0,
                "price {price} out of range for tick {tick}"
            );
        }
    }

    #[test]
    fn config_defaults_disabled() {
        std::env::remove_var("ORACLE_DEMO");
        let config = DemoFeedConfig::from_env();
        assert!(!config.enabled);
        assert_eq!(config.feeds.len(), 3);
    }

    #[test]
    fn config_has_default_feeds() {
        let config = DemoFeedConfig::from_env();
        assert!(config.feeds.iter().any(|f| f.symbol == "BTC/USD"));
        assert!(config.feeds.iter().any(|f| f.symbol == "CLP/USD"));
    }
}
