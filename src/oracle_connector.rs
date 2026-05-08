//! Oracle external connectors — bring real-world data into the chain.
//!
//! Provides a `DataSource` trait and concrete implementations for fetching
//! data from external HTTP APIs. Supports multi-source aggregation where
//! N-of-M sources must agree before a value is accepted.
//!
//! Env vars:
//! - `ORACLE_SOURCES`: comma-separated source configs (e.g. `http://api.example.com/price,http://backup.example.com/price`)
//! - `ORACLE_POLL_INTERVAL_SECS`: polling interval (default: 60)
//! - `ORACLE_MIN_SOURCES`: minimum agreeing sources for consensus (default: 1)

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

/// A single data point fetched from an external source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDataPoint {
    pub source_url: String,
    pub key: String,
    pub value: f64,
    pub timestamp: u64,
}

/// Errors from external data fetching.
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("failed to parse response: {0}")]
    Parse(String),
    #[error("insufficient sources: got {got}, need {need}")]
    InsufficientSources { got: usize, need: usize },
    #[error("sources disagree: spread {spread:.2}% exceeds threshold {threshold:.2}%")]
    SourceDisagreement { spread: f64, threshold: f64 },
    #[error("no sources configured")]
    NoSources,
}

/// Configuration for an external data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// HTTP URL to fetch data from.
    pub url: String,
    /// JSON path to extract the numeric value (dot-separated, e.g. "data.price").
    pub json_path: String,
    /// Optional API key header.
    pub api_key_header: Option<String>,
    /// Optional API key value.
    pub api_key_value: Option<String>,
    /// Request timeout.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_timeout_secs() -> u64 {
    10
}

/// Configuration for the oracle connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub sources: Vec<SourceConfig>,
    /// Minimum sources that must respond for a valid reading.
    #[serde(default = "default_min_sources")]
    pub min_sources: usize,
    /// Maximum spread (%) between sources before rejecting.
    #[serde(default = "default_max_spread")]
    pub max_spread_percent: f64,
    /// Polling interval in seconds.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

fn default_min_sources() -> usize {
    1
}
fn default_max_spread() -> f64 {
    5.0
}
fn default_poll_interval() -> u64 {
    60
}

impl ConnectorConfig {
    /// Build config from environment variable `ORACLE_SOURCES`.
    /// Format: comma-separated URLs. All use default json_path "price".
    pub fn from_env() -> Option<Self> {
        let urls = std::env::var("ORACLE_SOURCES").ok()?;
        if urls.is_empty() {
            return None;
        }
        let sources: Vec<SourceConfig> = urls
            .split(',')
            .map(|u| SourceConfig {
                url: u.trim().to_string(),
                json_path: "price".to_string(),
                api_key_header: None,
                api_key_value: None,
                timeout_secs: default_timeout_secs(),
            })
            .collect();

        let min_sources: usize = std::env::var("ORACLE_MIN_SOURCES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let poll_interval: u64 = std::env::var("ORACLE_POLL_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default_poll_interval());

        Some(Self {
            sources,
            min_sources,
            max_spread_percent: default_max_spread(),
            poll_interval_secs: poll_interval,
        })
    }
}

/// Result of aggregating multiple external sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedReading {
    pub key: String,
    pub value: f64,
    pub source_count: usize,
    pub spread_percent: f64,
    pub timestamp: u64,
    pub sources_used: Vec<String>,
}

/// Extract a numeric value from a JSON response using a dot-separated path.
fn extract_json_value(body: &str, json_path: &str) -> Result<f64, ConnectorError> {
    let parsed: serde_json::Value =
        serde_json::from_str(body).map_err(|e| ConnectorError::Parse(e.to_string()))?;

    let mut current = &parsed;
    for segment in json_path.split('.') {
        current = current
            .get(segment)
            .ok_or_else(|| ConnectorError::Parse(format!("path '{}' not found", json_path)))?;
    }

    current
        .as_f64()
        .ok_or_else(|| ConnectorError::Parse(format!("value at '{}' is not numeric", json_path)))
}

/// Fetch a single data point from one source.
async fn fetch_source(source: &SourceConfig) -> Result<ExternalDataPoint, ConnectorError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(source.timeout_secs))
        .build()
        .map_err(|e| ConnectorError::Http(e.to_string()))?;

    let mut req = client.get(&source.url);
    if let (Some(header), Some(value)) = (&source.api_key_header, &source.api_key_value) {
        req = req.header(header, value);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| ConnectorError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(ConnectorError::Http(format!(
            "status {}",
            resp.status().as_u16()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| ConnectorError::Http(e.to_string()))?;

    let value = extract_json_value(&body, &source.json_path)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(ExternalDataPoint {
        source_url: source.url.clone(),
        key: source.json_path.clone(),
        value,
        timestamp,
    })
}

/// Fetch from all configured sources in parallel and aggregate.
pub async fn fetch_and_aggregate(
    config: &ConnectorConfig,
    key: &str,
) -> Result<AggregatedReading, ConnectorError> {
    if config.sources.is_empty() {
        return Err(ConnectorError::NoSources);
    }

    let futures: Vec<_> = config.sources.iter().map(fetch_source).collect();
    let results = futures::future::join_all(futures).await;

    let mut values: Vec<(String, f64)> = Vec::new();
    for result in results {
        match result {
            Ok(dp) => values.push((dp.source_url, dp.value)),
            Err(e) => warn!(error = %e, "Oracle source fetch failed"),
        }
    }

    if values.len() < config.min_sources {
        return Err(ConnectorError::InsufficientSources {
            got: values.len(),
            need: config.min_sources,
        });
    }

    // Calculate spread
    let prices: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
    let min = prices.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = prices.iter().sum::<f64>() / prices.len() as f64;
    let spread = if avg > 0.0 {
        ((max - min) / avg) * 100.0
    } else {
        0.0
    };

    if spread > config.max_spread_percent {
        return Err(ConnectorError::SourceDisagreement {
            spread,
            threshold: config.max_spread_percent,
        });
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(AggregatedReading {
        key: key.to_string(),
        value: avg,
        source_count: values.len(),
        spread_percent: spread,
        timestamp,
        sources_used: values.into_iter().map(|(url, _)| url).collect(),
    })
}

/// Spawn a background poller that periodically fetches from external sources
/// and submits to the oracle registry.
pub fn spawn_oracle_poller(
    config: ConnectorConfig,
    registry: std::sync::Arc<std::sync::Mutex<crate::oracle_system::OracleRegistry>>,
    symbol: String,
) {
    tokio::spawn(async move {
        info!(
            sources = config.sources.len(),
            interval_secs = config.poll_interval_secs,
            symbol = %symbol,
            "Oracle external poller started"
        );

        loop {
            match fetch_and_aggregate(&config, &symbol).await {
                Ok(reading) => {
                    let mut reg = registry.lock().expect("oracle registry lock");
                    // Register a virtual oracle for the external connector if needed
                    let connector_id = "external-connector";
                    let _ = reg.register_oracle(connector_id.to_string());

                    let price = reading.value as u64;
                    let timestamp = reading.timestamp * 1000; // convert to ms
                    let signature = crate::oracle_system::OracleRegistry::generate_signature(
                        connector_id,
                        price,
                        timestamp,
                    );

                    match reg.submit_price_report(
                        connector_id,
                        symbol.clone(),
                        price,
                        timestamp,
                        signature,
                        (reading.source_count * 25).min(100) as u8,
                    ) {
                        Ok(()) => info!(
                            symbol = %symbol,
                            price,
                            sources = reading.source_count,
                            spread = format!("{:.2}%", reading.spread_percent),
                            "External oracle feed updated"
                        ),
                        Err(e) => warn!(error = %e, "Failed to submit external oracle data"),
                    }
                }
                Err(e) => {
                    error!(error = %e, symbol = %symbol, "External oracle fetch failed");
                }
            }

            tokio::time::sleep(Duration::from_secs(config.poll_interval_secs)).await;
        }
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_simple_path() {
        let body = r#"{"price": 42.5}"#;
        let val = extract_json_value(body, "price").unwrap();
        assert!((val - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_json_nested_path() {
        let body = r#"{"data": {"market": {"price": 100.25}}}"#;
        let val = extract_json_value(body, "data.market.price").unwrap();
        assert!((val - 100.25).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_json_missing_path() {
        let body = r#"{"data": {"other": 1}}"#;
        assert!(extract_json_value(body, "data.price").is_err());
    }

    #[test]
    fn extract_json_non_numeric() {
        let body = r#"{"price": "not a number"}"#;
        assert!(extract_json_value(body, "price").is_err());
    }

    #[test]
    fn extract_json_integer_as_f64() {
        let body = r#"{"value": 1000}"#;
        let val = extract_json_value(body, "value").unwrap();
        assert!((val - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn config_from_env_returns_none_when_unset() {
        std::env::remove_var("ORACLE_SOURCES");
        assert!(ConnectorConfig::from_env().is_none());
    }

    #[test]
    fn connector_error_display() {
        let e = ConnectorError::InsufficientSources { got: 1, need: 3 };
        assert!(e.to_string().contains("got 1"));
        assert!(e.to_string().contains("need 3"));
    }

    #[test]
    fn source_config_defaults() {
        let src = SourceConfig {
            url: "http://test.com".into(),
            json_path: "price".into(),
            api_key_header: None,
            api_key_value: None,
            timeout_secs: default_timeout_secs(),
        };
        assert_eq!(src.timeout_secs, 10);
    }

    #[test]
    fn connector_config_defaults() {
        let cfg = ConnectorConfig {
            sources: vec![],
            min_sources: default_min_sources(),
            max_spread_percent: default_max_spread(),
            poll_interval_secs: default_poll_interval(),
        };
        assert_eq!(cfg.min_sources, 1);
        assert!((cfg.max_spread_percent - 5.0).abs() < f64::EPSILON);
        assert_eq!(cfg.poll_interval_secs, 60);
    }

    #[tokio::test]
    async fn fetch_and_aggregate_no_sources() {
        let cfg = ConnectorConfig {
            sources: vec![],
            min_sources: 1,
            max_spread_percent: 5.0,
            poll_interval_secs: 60,
        };
        let result = fetch_and_aggregate(&cfg, "test").await;
        assert!(matches!(result, Err(ConnectorError::NoSources)));
    }

    #[tokio::test]
    async fn fetch_and_aggregate_unreachable_source() {
        let cfg = ConnectorConfig {
            sources: vec![SourceConfig {
                url: "http://192.0.2.1:1/noop".into(),
                json_path: "price".into(),
                api_key_header: None,
                api_key_value: None,
                timeout_secs: 1,
            }],
            min_sources: 1,
            max_spread_percent: 5.0,
            poll_interval_secs: 60,
        };
        let result = fetch_and_aggregate(&cfg, "test").await;
        assert!(matches!(
            result,
            Err(ConnectorError::InsufficientSources { .. })
        ));
    }
}
