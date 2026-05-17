//! RWA Tokenization — represent physical assets as verifiable digital collateral.
//!
//! Links an Asset Registry entry to a token that can be used as collateral
//! for financial operations. Tracks valuation, collateral status, and
//! provides verifiable proof of asset state for lenders/financiers.

use serde::{Deserialize, Serialize};

/// A tokenized representation of a physical asset (Real World Asset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetToken {
    /// Unique token identifier.
    pub id: String,
    /// Asset Registry ID this token represents.
    pub asset_id: String,
    /// DID of the token issuer (e.g., Scania Credit).
    pub issuer_did: String,
    /// DID of the current token holder.
    pub holder_did: String,
    /// Current valuation in smallest currency unit (e.g., centavos).
    pub valuation: u64,
    /// ISO 4217 currency code (e.g., "ARS", "USD").
    pub currency: String,
    /// Collateral status.
    pub collateral_status: CollateralStatus,
    /// Regulatory framework reference (e.g., "CNV RG 1069/2025").
    #[serde(default)]
    pub regulatory_ref: String,
    /// Free-form metadata (terms, conditions, liens).
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Collateral lifecycle status.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollateralStatus {
    /// Token issued, not pledged.
    #[default]
    Free,
    /// Pledged as collateral to a lender.
    Pledged,
    /// Collateral released after loan repayment.
    Released,
    /// Collateral seized due to default.
    Seized,
    /// Token invalidated (asset decommissioned).
    Invalidated,
}

/// A valuation update for a tokenized asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationUpdate {
    /// Unique update identifier.
    pub id: String,
    /// Token this valuation applies to.
    pub token_id: String,
    /// New valuation amount.
    pub valuation: u64,
    /// Currency code.
    pub currency: String,
    /// Source of the valuation (e.g., "oracle:market", "appraiser:did:cerulean:xyz").
    pub source: String,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_serializes() {
        let token = AssetToken {
            id: "tok-1".into(),
            asset_id: "asset-1".into(),
            issuer_did: "did:cerulean:scania-credit".into(),
            holder_did: "did:cerulean:fleet-owner".into(),
            valuation: 15_000_000_00, // $150,000.00 in centavos
            currency: "ARS".into(),
            collateral_status: CollateralStatus::Free,
            regulatory_ref: "CNV RG 1069/2025".into(),
            metadata: serde_json::json!({}),
            created_at: 1700000000,
            updated_at: 1700000000,
        };
        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("scania-credit"));
        assert!(json.contains("free"));
    }
}
