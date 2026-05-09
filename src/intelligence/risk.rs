//! Risk scoring engine — evaluates transaction and identity risk for AML/compliance.
//!
//! Configurable rule-based scoring. Each rule contributes points; total determines risk level.

use serde::{Deserialize, Serialize};

/// Risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Input data for risk evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskInput {
    /// Transaction or operation amount.
    pub amount: u64,
    /// Country code (ISO 3166) of originator.
    pub country: Option<String>,
    /// Number of transactions in the last hour from this identity.
    pub tx_count_last_hour: u32,
    /// Whether identity has verified KYC claims.
    pub kyc_verified: bool,
    /// Whether identity is on a watchlist.
    pub watchlisted: bool,
    /// Age of identity in days.
    pub identity_age_days: u32,
}

/// Result of risk evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskResult {
    pub score: u32,
    pub level: RiskLevel,
    pub factors: Vec<RiskFactor>,
    pub recommendation: String,
}

/// A single risk factor that contributed to the score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub rule: String,
    pub points: u32,
    pub detail: String,
}

/// Configuration for risk thresholds.
#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub high_amount_threshold: u64,
    pub very_high_amount_threshold: u64,
    pub high_frequency_threshold: u32,
    pub new_identity_days: u32,
    pub high_risk_countries: Vec<String>,
    /// Score thresholds: (medium, high, critical)
    pub thresholds: (u32, u32, u32),
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            high_amount_threshold: 100_000,
            very_high_amount_threshold: 1_000_000,
            high_frequency_threshold: 50,
            new_identity_days: 30,
            high_risk_countries: vec![],
            thresholds: (20, 50, 80),
        }
    }
}

/// Risk scoring engine.
pub struct RiskEngine {
    config: RiskConfig,
}

impl RiskEngine {
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(RiskConfig::default())
    }

    /// Evaluate risk for a given input.
    pub fn evaluate(&self, input: &RiskInput) -> RiskResult {
        let mut factors = Vec::new();
        let mut score: u32 = 0;

        // Rule 1: Watchlist
        if input.watchlisted {
            let points = 100;
            score += points;
            factors.push(RiskFactor {
                rule: "watchlist".into(),
                points,
                detail: "Identity is on a watchlist".into(),
            });
        }

        // Rule 2: KYC not verified
        if !input.kyc_verified {
            let points = 30;
            score += points;
            factors.push(RiskFactor {
                rule: "kyc_missing".into(),
                points,
                detail: "Identity has not completed KYC verification".into(),
            });
        }

        // Rule 3: High amount
        if input.amount >= self.config.very_high_amount_threshold {
            let points = 40;
            score += points;
            factors.push(RiskFactor {
                rule: "very_high_amount".into(),
                points,
                detail: format!(
                    "Amount {} exceeds very high threshold {}",
                    input.amount, self.config.very_high_amount_threshold
                ),
            });
        } else if input.amount >= self.config.high_amount_threshold {
            let points = 20;
            score += points;
            factors.push(RiskFactor {
                rule: "high_amount".into(),
                points,
                detail: format!(
                    "Amount {} exceeds high threshold {}",
                    input.amount, self.config.high_amount_threshold
                ),
            });
        }

        // Rule 4: High frequency
        if input.tx_count_last_hour >= self.config.high_frequency_threshold {
            let points = 25;
            score += points;
            factors.push(RiskFactor {
                rule: "high_frequency".into(),
                points,
                detail: format!(
                    "{} transactions in last hour (threshold: {})",
                    input.tx_count_last_hour, self.config.high_frequency_threshold
                ),
            });
        }

        // Rule 5: New identity
        if input.identity_age_days < self.config.new_identity_days {
            let points = 15;
            score += points;
            factors.push(RiskFactor {
                rule: "new_identity".into(),
                points,
                detail: format!(
                    "Identity is {} days old (threshold: {})",
                    input.identity_age_days, self.config.new_identity_days
                ),
            });
        }

        // Rule 6: High-risk country
        if let Some(ref country) = input.country {
            if self.config.high_risk_countries.iter().any(|c| c == country) {
                let points = 20;
                score += points;
                factors.push(RiskFactor {
                    rule: "high_risk_country".into(),
                    points,
                    detail: format!("Country {country} is in high-risk list"),
                });
            }
        }

        let (med, high, crit) = self.config.thresholds;
        let level = if score >= crit {
            RiskLevel::Critical
        } else if score >= high {
            RiskLevel::High
        } else if score >= med {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let recommendation = match level {
            RiskLevel::Low => "Proceed normally".into(),
            RiskLevel::Medium => "Enhanced monitoring recommended".into(),
            RiskLevel::High => "Manual review required before processing".into(),
            RiskLevel::Critical => "Block transaction and escalate to compliance officer".into(),
        };

        RiskResult {
            score,
            level,
            factors,
            recommendation,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_input() -> RiskInput {
        RiskInput {
            amount: 1000,
            country: Some("CL".into()),
            tx_count_last_hour: 5,
            kyc_verified: true,
            watchlisted: false,
            identity_age_days: 365,
        }
    }

    #[test]
    fn clean_transaction_is_low_risk() {
        let engine = RiskEngine::with_defaults();
        let result = engine.evaluate(&clean_input());
        assert_eq!(result.level, RiskLevel::Low);
        assert_eq!(result.score, 0);
        assert!(result.factors.is_empty());
    }

    #[test]
    fn watchlisted_is_critical() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.watchlisted = true;
        let result = engine.evaluate(&input);
        assert_eq!(result.level, RiskLevel::Critical);
        assert!(result.score >= 100);
    }

    #[test]
    fn no_kyc_adds_risk() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.kyc_verified = false;
        let result = engine.evaluate(&input);
        assert_eq!(result.level, RiskLevel::Medium);
        assert!(result.factors.iter().any(|f| f.rule == "kyc_missing"));
    }

    #[test]
    fn high_amount_adds_risk() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.amount = 500_000;
        let result = engine.evaluate(&input);
        assert!(result.factors.iter().any(|f| f.rule == "high_amount"));
    }

    #[test]
    fn very_high_amount_adds_more_risk() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.amount = 5_000_000;
        let result = engine.evaluate(&input);
        assert!(result.factors.iter().any(|f| f.rule == "very_high_amount"));
    }

    #[test]
    fn high_frequency_adds_risk() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.tx_count_last_hour = 100;
        let result = engine.evaluate(&input);
        assert!(result.factors.iter().any(|f| f.rule == "high_frequency"));
    }

    #[test]
    fn new_identity_adds_risk() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.identity_age_days = 5;
        let result = engine.evaluate(&input);
        assert!(result.factors.iter().any(|f| f.rule == "new_identity"));
    }

    #[test]
    fn multiple_factors_accumulate() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.kyc_verified = false; // +30
        input.amount = 500_000; // +20
        input.tx_count_last_hour = 100; // +25
        input.identity_age_days = 5; // +15
        let result = engine.evaluate(&input);
        assert_eq!(result.score, 90);
        assert_eq!(result.level, RiskLevel::Critical);
        assert_eq!(result.factors.len(), 4);
    }

    #[test]
    fn high_risk_country() {
        let engine = RiskEngine::new(RiskConfig {
            high_risk_countries: vec!["XX".into()],
            ..RiskConfig::default()
        });
        let mut input = clean_input();
        input.country = Some("XX".into());
        let result = engine.evaluate(&input);
        assert!(result.factors.iter().any(|f| f.rule == "high_risk_country"));
    }

    #[test]
    fn recommendation_matches_level() {
        let engine = RiskEngine::with_defaults();
        let r = engine.evaluate(&clean_input());
        assert!(r.recommendation.contains("normally"));

        let mut input = clean_input();
        input.watchlisted = true;
        let r = engine.evaluate(&input);
        assert!(r.recommendation.contains("Block"));
    }

    #[test]
    fn risk_result_serde_roundtrip() {
        let engine = RiskEngine::with_defaults();
        let mut input = clean_input();
        input.kyc_verified = false;
        let result = engine.evaluate(&input);
        let json = serde_json::to_string(&result).unwrap();
        let restored: RiskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.level, result.level);
        assert_eq!(restored.score, result.score);
    }
}
