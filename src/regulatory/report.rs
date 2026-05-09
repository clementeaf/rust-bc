//! Compliance report generator — produces structured, exportable reports
//! from regulatory sandbox check results.

use serde::{Deserialize, Serialize};

use super::sandbox::{CheckResult, ComplianceSummary};

/// A full compliance report ready for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: String,
    pub generated_at: String,
    pub platform: String,
    pub version: String,
    pub summary: ComplianceSummary,
    pub checks: Vec<CheckResult>,
    pub content_hash: String,
}

/// Generate a compliance report from check results.
pub fn generate_report(results: Vec<CheckResult>, summary: ComplianceSummary) -> ComplianceReport {
    let now = chrono::Utc::now().to_rfc3339();
    let report_id = format!("CMP-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // Hash the content for integrity verification
    let content = serde_json::to_vec(&(&results, &summary)).unwrap_or_default();
    use pqc_crypto_module::legacy::sha256::{Digest as _, Sha256};
    let hash = Sha256::digest(&content);
    let content_hash = format!("{:x}", hash);

    ComplianceReport {
        report_id,
        generated_at: now,
        platform: "Cerulean Ledger".into(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        summary,
        checks: results,
        content_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regulatory::sandbox;

    #[test]
    fn report_generation() {
        let results = sandbox::run_compliance_checks();
        let summary = sandbox::summarize(&results);
        let report = generate_report(results, summary);

        assert!(report.report_id.starts_with("CMP-"));
        assert_eq!(report.platform, "Cerulean Ledger");
        assert!(!report.content_hash.is_empty());
        assert!(report.checks.len() >= 20);
    }

    #[test]
    fn report_serde_roundtrip() {
        let results = sandbox::run_compliance_checks();
        let summary = sandbox::summarize(&results);
        let report = generate_report(results, summary);

        let json = serde_json::to_string(&report).unwrap();
        let restored: ComplianceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.report_id, report.report_id);
        assert_eq!(restored.content_hash, report.content_hash);
    }
}
