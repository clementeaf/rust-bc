//! Core types for the Asset Registry module.

use serde::{Deserialize, Serialize};

/// A registered physical asset with a unique DID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique identifier (e.g., "asset-001").
    pub id: String,
    /// Decentralized identifier (e.g., "did:cerulean:asset:VIN123").
    pub did: String,
    /// Asset category (e.g., "vehicle", "equipment", "machinery").
    pub asset_type: String,
    /// Owner DID (organization or individual).
    pub owner_did: String,
    /// Human-readable label (e.g., "Scania R450 - Patente ABC123").
    pub label: String,
    /// Free-form metadata (make, model, year, serial number, etc.).
    pub metadata: serde_json::Value,
    /// Current lifecycle status.
    pub status: AssetStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Asset lifecycle status.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetStatus {
    #[default]
    Active,
    Suspended,
    Decommissioned,
}

/// A signed, timestamped event in an asset's history.
///
/// Events are immutable once written — they form the certified
/// audit trail for the asset. Examples: odometer reading, service
/// performed, incident reported, inspection passed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEvent {
    /// Unique event identifier.
    pub id: String,
    /// Asset this event belongs to.
    pub asset_id: String,
    /// Event category (e.g., "telemetry", "service", "inspection", "incident").
    pub event_type: String,
    /// Event payload — schema depends on event_type.
    pub data: serde_json::Value,
    /// ISO 8601 timestamp of when the event occurred (not when it was recorded).
    pub occurred_at: u64,
    /// When the event was recorded on-chain.
    pub recorded_at: u64,
    /// DID of the entity that reported this event.
    pub source_did: String,
    /// Hex-encoded signature over the event data by the source.
    #[serde(default)]
    pub signature: String,
}

/// Query filter for listing asset events.
#[derive(Debug, Clone, Deserialize)]
pub struct EventFilter {
    /// Filter by asset ID.
    pub asset_id: Option<String>,
    /// Filter by event type.
    pub event_type: Option<String>,
    /// Events after this timestamp.
    pub from: Option<u64>,
    /// Events before this timestamp.
    pub to: Option<u64>,
}

/// Certified data export — a signed snapshot of an asset's history,
/// intended for delivery to authorized third parties (insurers, banks,
/// auditors). Includes the asset record + filtered events + export signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertifiedExport {
    /// Export identifier.
    pub id: String,
    /// The asset being exported.
    pub asset: Asset,
    /// Filtered events included in this export.
    pub events: Vec<AssetEvent>,
    /// Who requested the export.
    pub requested_by: String,
    /// Who the export is for (e.g., insurer DID).
    pub recipient: String,
    /// Export timestamp.
    pub exported_at: u64,
    /// Node signature over the export content (proves data integrity).
    #[serde(default)]
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_serializes() {
        let asset = Asset {
            id: "a1".into(),
            did: "did:cerulean:asset:VIN123".into(),
            asset_type: "vehicle".into(),
            owner_did: "did:cerulean:org1".into(),
            label: "Scania R450".into(),
            metadata: serde_json::json!({"vin": "VIN123", "year": 2024}),
            status: AssetStatus::Active,
            created_at: 1700000000,
            updated_at: 1700000000,
        };
        let json = serde_json::to_string(&asset).unwrap();
        assert!(json.contains("VIN123"));
        assert!(json.contains("active"));
    }

    #[test]
    fn event_serializes() {
        let event = AssetEvent {
            id: "ev1".into(),
            asset_id: "a1".into(),
            event_type: "telemetry".into(),
            data: serde_json::json!({"km": 150000, "fuel_l": 45.2}),
            occurred_at: 1700000000,
            recorded_at: 1700000001,
            source_did: "did:cerulean:myscania".into(),
            signature: String::new(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("150000"));
    }
}
