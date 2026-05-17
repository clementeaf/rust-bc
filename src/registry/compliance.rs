//! Compliance Automation — validate real operations against declared terms.
//!
//! Generic rule engine for comparing actual data (from AssetEvents/telemetry)
//! against contractual terms. Produces signed compliance reports that can
//! trigger downstream actions (payments, alerts, invoicing).

use serde::{Deserialize, Serialize};

/// A compliance rule that defines what to validate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Unique rule identifier.
    pub id: String,
    /// Human-readable name (e.g., "Route deviation check").
    pub name: String,
    /// Asset or scope this rule applies to (asset_id, or "*" for all).
    pub target: String,
    /// Event type to evaluate (e.g., "telemetry", "delivery").
    pub event_type: String,
    /// The field in event.data to check (JSON path, e.g., "km", "fuel_l").
    pub field: String,
    /// Comparison operator.
    pub operator: ComplianceOperator,
    /// Expected value (threshold).
    pub threshold: f64,
    /// Action when rule fails.
    pub on_failure: FailureAction,
    pub created_at: u64,
}

/// Comparison operators for compliance checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceOperator {
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

/// What happens when a compliance check fails.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FailureAction {
    /// Log the violation (default).
    #[default]
    Log,
    /// Send alert via webhook.
    Alert,
    /// Block the operation.
    Block,
}

/// Result of evaluating a compliance rule against an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    /// Unique result identifier.
    pub id: String,
    /// Rule that was evaluated.
    pub rule_id: String,
    /// Asset that was checked.
    pub asset_id: String,
    /// Event that triggered the check.
    pub event_id: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Actual value found in the event data.
    pub actual_value: f64,
    /// Expected threshold from the rule.
    pub expected_value: f64,
    /// Human-readable explanation.
    pub detail: String,
    pub evaluated_at: u64,
}

/// Evaluate a rule against a value.
pub fn evaluate_rule(rule: &ComplianceRule, actual: f64) -> bool {
    match rule.operator {
        ComplianceOperator::LessThan => actual < rule.threshold,
        ComplianceOperator::LessOrEqual => actual <= rule.threshold,
        ComplianceOperator::GreaterThan => actual > rule.threshold,
        ComplianceOperator::GreaterOrEqual => actual >= rule.threshold,
        ComplianceOperator::Equal => (actual - rule.threshold).abs() < f64::EPSILON,
        ComplianceOperator::NotEqual => (actual - rule.threshold).abs() >= f64::EPSILON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rule(op: ComplianceOperator, threshold: f64) -> ComplianceRule {
        ComplianceRule {
            id: "r1".into(),
            name: "test".into(),
            target: "*".into(),
            event_type: "telemetry".into(),
            field: "km".into(),
            operator: op,
            threshold,
            on_failure: FailureAction::Log,
            created_at: 0,
        }
    }

    #[test]
    fn less_than() {
        assert!(evaluate_rule(
            &sample_rule(ComplianceOperator::LessThan, 100.0),
            50.0
        ));
        assert!(!evaluate_rule(
            &sample_rule(ComplianceOperator::LessThan, 100.0),
            150.0
        ));
    }

    #[test]
    fn greater_or_equal() {
        assert!(evaluate_rule(
            &sample_rule(ComplianceOperator::GreaterOrEqual, 10.0),
            10.0
        ));
        assert!(evaluate_rule(
            &sample_rule(ComplianceOperator::GreaterOrEqual, 10.0),
            15.0
        ));
        assert!(!evaluate_rule(
            &sample_rule(ComplianceOperator::GreaterOrEqual, 10.0),
            5.0
        ));
    }

    #[test]
    fn equal() {
        assert!(evaluate_rule(
            &sample_rule(ComplianceOperator::Equal, 42.0),
            42.0
        ));
        assert!(!evaluate_rule(
            &sample_rule(ComplianceOperator::Equal, 42.0),
            43.0
        ));
    }
}
