//! Telemetry Adapter — polls external data sources and ingests into Asset Registry.
//!
//! Generic framework for connecting any telemetry API (MyScania, IoT platforms,
//! fleet management systems) to the Asset Registry. Client-specific configuration
//! is done via environment variables.
//!
//! Usage:
//!   TELEMETRY_SOURCE_URL=https://api.myscania.com/v1/vehicles
//!   TELEMETRY_POLL_INTERVAL_SECS=300
//!   TELEMETRY_API_KEY=your-key-here
//!   TELEMETRY_ASSET_TYPE=vehicle

use crate::registry::types::{Asset, AssetEvent, AssetStatus};
use crate::storage::traits::BlockStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

/// Configuration for the telemetry adapter.
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// URL of the external telemetry source.
    pub source_url: String,
    /// Polling interval in seconds.
    pub poll_interval_secs: u64,
    /// API key or bearer token for authentication.
    pub api_key: String,
    /// Asset type label (e.g., "vehicle", "equipment").
    pub asset_type: String,
    /// JSON path to the array of items in the API response (e.g., "data.vehicles").
    /// Empty string means the response root is the array.
    pub items_path: String,
    /// Field mapping: which JSON fields map to Asset/Event fields.
    pub field_map: FieldMap,
}

/// Maps external API fields to internal Asset Registry fields.
#[derive(Debug, Clone)]
pub struct FieldMap {
    /// JSON field for asset unique ID (e.g., "vin", "serial_number").
    pub asset_id: String,
    /// JSON field for asset label (e.g., "plate", "name").
    pub asset_label: String,
    /// JSON field for owner identifier.
    pub owner_id: String,
}

impl Default for FieldMap {
    fn default() -> Self {
        Self {
            asset_id: "id".to_string(),
            asset_label: "name".to_string(),
            owner_id: "owner".to_string(),
        }
    }
}

impl AdapterConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Option<Self> {
        let source_url = std::env::var("TELEMETRY_SOURCE_URL").ok()?;
        Some(Self {
            source_url,
            poll_interval_secs: std::env::var("TELEMETRY_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            api_key: std::env::var("TELEMETRY_API_KEY").unwrap_or_default(),
            asset_type: std::env::var("TELEMETRY_ASSET_TYPE")
                .unwrap_or_else(|_| "vehicle".to_string()),
            items_path: std::env::var("TELEMETRY_ITEMS_PATH").unwrap_or_default(),
            field_map: FieldMap {
                asset_id: std::env::var("TELEMETRY_FIELD_ID").unwrap_or_else(|_| "id".to_string()),
                asset_label: std::env::var("TELEMETRY_FIELD_LABEL")
                    .unwrap_or_else(|_| "name".to_string()),
                owner_id: std::env::var("TELEMETRY_FIELD_OWNER")
                    .unwrap_or_else(|_| "owner".to_string()),
            },
        })
    }
}

/// Spawn the telemetry poller as a background task.
pub fn spawn_telemetry_poller(config: AdapterConfig, store: Arc<dyn BlockStore>) {
    tokio::spawn(async move {
        log::info!(
            "Telemetry adapter started: polling {} every {}s",
            config.source_url,
            config.poll_interval_secs
        );
        let mut interval = time::interval(Duration::from_secs(config.poll_interval_secs));
        let client = reqwest::Client::new();

        loop {
            interval.tick().await;
            match poll_and_ingest(&client, &config, &store).await {
                Ok(count) => {
                    if count > 0 {
                        log::info!("Telemetry adapter: ingested {count} events");
                    }
                }
                Err(e) => {
                    log::warn!("Telemetry adapter error: {e}");
                }
            }
        }
    });
}

/// Poll the external source and ingest data into the Asset Registry.
async fn poll_and_ingest(
    client: &reqwest::Client,
    config: &AdapterConfig,
    store: &Arc<dyn BlockStore>,
) -> Result<usize, String> {
    // Build request
    let mut req = client.get(&config.source_url);
    if !config.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    // Fetch
    let resp = req.send().await.map_err(|e| format!("HTTP error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {e}"))?;

    // Extract items array
    let items = if config.items_path.is_empty() {
        body.as_array().cloned().unwrap_or_default()
    } else {
        let mut current = &body;
        for key in config.items_path.split('.') {
            current = current.get(key).unwrap_or(&serde_json::Value::Null);
        }
        current.as_array().cloned().unwrap_or_default()
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut event_count = 0;

    for item in &items {
        let asset_id = item
            .get(&config.field_map.asset_id)
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let label = item
            .get(&config.field_map.asset_label)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let owner = item
            .get(&config.field_map.owner_id)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Upsert asset
        let asset = Asset {
            id: asset_id.clone(),
            did: format!("did:cerulean:asset:{asset_id}"),
            asset_type: config.asset_type.clone(),
            owner_did: format!("did:cerulean:{owner}"),
            label,
            metadata: item.clone(),
            status: AssetStatus::Active,
            created_at: now,
            updated_at: now,
        };
        let _ = store.write_asset(&asset);

        // Create telemetry event
        let event = AssetEvent {
            id: format!("tel-{asset_id}-{now}"),
            asset_id: asset_id.clone(),
            event_type: "telemetry".to_string(),
            data: item.clone(),
            occurred_at: now,
            recorded_at: now,
            source_did: format!("did:cerulean:adapter:{}", config.asset_type),
            signature: String::new(),
        };
        let _ = store.write_asset_event(&event);
        event_count += 1;
    }

    Ok(event_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let fm = FieldMap::default();
        assert_eq!(fm.asset_id, "id");
        assert_eq!(fm.asset_label, "name");
    }

    #[test]
    fn items_path_navigation() {
        let body = serde_json::json!({
            "data": {
                "vehicles": [
                    {"id": "V1", "name": "Truck 1"},
                    {"id": "V2", "name": "Truck 2"}
                ]
            }
        });

        let path = "data.vehicles";
        let mut current = &body;
        for key in path.split('.') {
            current = current.get(key).unwrap_or(&serde_json::Value::Null);
        }
        let items = current.as_array().unwrap();
        assert_eq!(items.len(), 2);
    }
}
