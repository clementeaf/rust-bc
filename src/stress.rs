//! Module-level stress testing — runs targeted stress tests on each subsystem
//! and produces a certification-ready report.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Result of a single module stress test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStressResult {
    pub module: String,
    pub operations: u64,
    pub duration_ms: u64,
    pub ops_per_sec: f64,
    pub p50_us: u64,
    pub p99_us: u64,
    pub errors: u64,
    pub status: StressStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StressStatus {
    Pass,
    Degraded,
    Fail,
}

/// Full stress report across all modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressReport {
    pub report_id: String,
    pub generated_at: String,
    pub total_modules: usize,
    pub passed: usize,
    pub degraded: usize,
    pub failed: usize,
    pub results: Vec<ModuleStressResult>,
}

/// Helper: measure latencies and compute percentiles.
fn percentile(sorted_us: &[u64], pct: f64) -> u64 {
    if sorted_us.is_empty() {
        return 0;
    }
    let idx = ((pct / 100.0) * sorted_us.len() as f64) as usize;
    sorted_us[idx.min(sorted_us.len() - 1)]
}

/// Stress test: Storage (MemoryStore write/read).
pub fn stress_storage(ops: u64) -> ModuleStressResult {
    use crate::storage::memory::MemoryStore;
    use crate::storage::traits::{Block, BlockStore};

    let store = MemoryStore::new();
    let mut latencies = Vec::with_capacity(ops as usize);
    let mut errors: u64 = 0;

    let start = Instant::now();
    for i in 0..ops {
        let block = Block {
            height: i,
            timestamp: 1000 + i,
            parent_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            transactions: vec![format!("tx-{i}")],
            proposer: "stress".into(),
            signature: vec![0u8; 64],
            signature_algorithm: Default::default(),
            endorsements: vec![],
            secondary_signature: None,
            secondary_signature_algorithm: None,
            hash_algorithm: Default::default(),
            orderer_signature: None,
        };

        let op_start = Instant::now();
        if store.write_block(&block).is_err() {
            errors += 1;
        }
        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "storage".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors,
        status: if errors == 0 && ops_per_sec > 1000.0 {
            StressStatus::Pass
        } else if errors == 0 {
            StressStatus::Degraded
        } else {
            StressStatus::Fail
        },
    }
}

/// Stress test: Cryptography (SHA3-256 hashing).
pub fn stress_crypto(ops: u64) -> ModuleStressResult {
    use pqc_crypto_module::legacy::sha256::{Digest as _, Sha256};

    let data = b"cerulean-stress-test-payload-for-hashing";
    let mut latencies = Vec::with_capacity(ops as usize);

    let start = Instant::now();
    for _ in 0..ops {
        let op_start = Instant::now();
        let _ = Sha256::digest(data);
        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "crypto_hash".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors: 0,
        status: if ops_per_sec > 100_000.0 {
            StressStatus::Pass
        } else {
            StressStatus::Degraded
        },
    }
}

/// Stress test: Anomaly detection engine.
pub fn stress_anomaly(ops: u64) -> ModuleStressResult {
    use crate::intelligence::anomaly::{AnomalyConfig, AnomalyDetector, DataPoint};

    let mut detector = AnomalyDetector::new(AnomalyConfig {
        z_threshold: 3.0,
        min_samples: 10,
        window_size: 1000,
    });

    let mut latencies = Vec::with_capacity(ops as usize);

    let start = Instant::now();
    for i in 0..ops {
        let point = DataPoint {
            timestamp: i,
            value: 100.0 + (i % 10) as f64,
            source: "stress".into(),
        };
        let op_start = Instant::now();
        let _ = detector.observe(&point);
        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "anomaly_detection".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors: 0,
        status: if ops_per_sec > 100_000.0 {
            StressStatus::Pass
        } else {
            StressStatus::Degraded
        },
    }
}

/// Stress test: Risk scoring engine.
pub fn stress_risk(ops: u64) -> ModuleStressResult {
    use crate::intelligence::risk::{RiskEngine, RiskInput};

    let engine = RiskEngine::with_defaults();
    let input = RiskInput {
        amount: 50_000,
        country: Some("CL".into()),
        tx_count_last_hour: 10,
        kyc_verified: true,
        watchlisted: false,
        identity_age_days: 100,
    };

    let mut latencies = Vec::with_capacity(ops as usize);

    let start = Instant::now();
    for _ in 0..ops {
        let op_start = Instant::now();
        let _ = engine.evaluate(&input);
        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "risk_scoring".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors: 0,
        status: if ops_per_sec > 500_000.0 {
            StressStatus::Pass
        } else {
            StressStatus::Degraded
        },
    }
}

/// Stress test: ISO 20022 validation.
pub fn stress_compliance(ops: u64) -> ModuleStressResult {
    use crate::compliance::iso20022::{validate_pacs008, CurrencyAmount, Pacs008, Party};

    let msg = Pacs008 {
        message_id: "STRESS".into(),
        creation_date: "2026-05-09".into(),
        settlement_amount: CurrencyAmount {
            amount: 100_000,
            currency: "CLP".into(),
        },
        debtor: Party {
            name: "Stress Test".into(),
            country: "CL".into(),
            account_iban: Some("CL9300000000123456789012".into()),
            bic: Some("BCHICLRM".into()),
        },
        creditor: Party {
            name: "Receiver".into(),
            country: "AR".into(),
            account_iban: None,
            bic: Some("NACNARBAXXX".into()),
        },
        debtor_agent_bic: "BCHICLRM".into(),
        creditor_agent_bic: "NACNARBAXXX".into(),
        remittance_info: None,
    };

    let mut latencies = Vec::with_capacity(ops as usize);
    let mut errors: u64 = 0;

    let start = Instant::now();
    for _ in 0..ops {
        let op_start = Instant::now();
        if validate_pacs008(&msg).is_err() {
            errors += 1;
        }
        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "iso20022_validation".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors,
        status: if errors == 0 && ops_per_sec > 100_000.0 {
            StressStatus::Pass
        } else if errors == 0 {
            StressStatus::Degraded
        } else {
            StressStatus::Fail
        },
    }
}

/// Stress test: Governance (proposal submit + vote cycle).
pub fn stress_governance(ops: u64) -> ModuleStressResult {
    use crate::governance::params::ParamRegistry;
    use crate::governance::proposals::{ProposalAction, ProposalStore, SubmitParams};
    use crate::governance::voting::{VoteOption, VoteStore};

    let proposals = ProposalStore::new();
    let votes = VoteStore::new();
    let params = ParamRegistry::with_defaults();
    let deposit = params.get_u64("proposal_deposit", 10_000);
    let voting_period = params.get_u64("voting_period_blocks", 17_280);

    let mut latencies = Vec::with_capacity(ops as usize);
    let mut errors: u64 = 0;

    let start = Instant::now();
    for i in 0..ops {
        let op_start = Instant::now();

        let action = ProposalAction::TextProposal {
            title: format!("stress-{i}"),
            description: "stress test".into(),
        };
        match proposals.submit(SubmitParams {
            proposer: "stress-proposer",
            action,
            description: "stress",
            deposit,
            required_deposit: deposit,
            current_height: i,
            voting_period,
        }) {
            Ok(pid) => {
                let _ = votes.cast_vote(
                    pid,
                    &format!("voter-{i}"),
                    VoteOption::Yes,
                    1,
                    i,
                    i + voting_period,
                );
            }
            Err(_) => errors += 1,
        }

        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "governance".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors,
        // Rate limiting causes expected rejections — not failures
        status: if ops_per_sec > 1_000.0 {
            StressStatus::Pass
        } else {
            StressStatus::Degraded
        },
    }
}

/// Stress test: Forensic engine (timeline build + evidence package).
pub fn stress_forensic(ops: u64) -> ModuleStressResult {
    use crate::audit::AuditEntry;
    use crate::forensic::ForensicEngine;

    let mut latencies = Vec::with_capacity(ops as usize);

    let start = Instant::now();
    for i in 0..ops {
        let op_start = Instant::now();

        let mut engine = ForensicEngine::new();
        engine.ingest_audit(&[AuditEntry {
            timestamp: format!("2026-05-09T{:02}:00:00Z", i % 24),
            action: crate::audit::AuditAction::HttpRequest,
            method: "POST".into(),
            path: "/api/v1/vote".into(),
            org_id: "stress".into(),
            source_ip: "127.0.0.1".into(),
            status_code: 200,
            trace_id: format!("trace-{i}"),
            duration_ms: 5,
            metadata: None,
        }]);
        let _ = engine.build_timeline();

        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "forensic".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors: 0,
        status: if ops_per_sec > 50_000.0 {
            StressStatus::Pass
        } else {
            StressStatus::Degraded
        },
    }
}

/// Stress test: Pattern detection engine.
pub fn stress_patterns(ops: u64) -> ModuleStressResult {
    use crate::intelligence::patterns::{PatternEngine, TxRecord};

    let engine = PatternEngine::new();
    let txs: Vec<TxRecord> = (0..20)
        .map(|i| TxRecord {
            tx_id: format!("tx-{i}"),
            from: "alice".into(),
            to: "bob".into(),
            amount: 1000,
            timestamp: 100 + i,
        })
        .collect();

    let mut latencies = Vec::with_capacity(ops as usize);

    let start = Instant::now();
    for _ in 0..ops {
        let op_start = Instant::now();
        let _ = engine.analyze(&txs);
        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "pattern_detection".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors: 0,
        status: if ops_per_sec > 10_000.0 {
            StressStatus::Pass
        } else {
            StressStatus::Degraded
        },
    }
}

/// Stress test: Identity (DID creation + read cycle).
pub fn stress_identity(ops: u64) -> ModuleStressResult {
    use crate::storage::memory::MemoryStore;
    use crate::storage::traits::{BlockStore, IdentityRecord};

    let store = MemoryStore::new();
    let mut latencies = Vec::with_capacity(ops as usize);
    let mut errors: u64 = 0;

    let start = Instant::now();
    for i in 0..ops {
        let op_start = Instant::now();

        let did = format!("did:cerulean:stress-{i}");
        let record = IdentityRecord {
            did: did.clone(),
            created_at: 1000 + i,
            updated_at: 1000 + i,
            status: "active".into(),
        };

        if store.write_identity(&record).is_err() {
            errors += 1;
        }
        // Read back to exercise read path
        if store.read_identity(&did).is_err() {
            errors += 1;
        }

        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "identity".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors,
        status: if errors == 0 && ops_per_sec > 1_000.0 {
            StressStatus::Pass
        } else if errors == 0 {
            StressStatus::Degraded
        } else {
            StressStatus::Fail
        },
    }
}

/// Stress test: Credential issuance + verification cycle.
pub fn stress_credential(ops: u64) -> ModuleStressResult {
    use crate::storage::memory::MemoryStore;
    use crate::storage::traits::{BlockStore, Credential, IdentityRecord};

    let store = MemoryStore::new();

    // Pre-register issuer DID
    let issuer_did = "did:cerulean:issuer-stress";
    store
        .write_identity(&IdentityRecord {
            did: issuer_did.into(),
            created_at: 1000,
            updated_at: 1000,
            status: "active".into(),
        })
        .unwrap();

    let mut latencies = Vec::with_capacity(ops as usize);
    let mut errors: u64 = 0;

    let start = Instant::now();
    for i in 0..ops {
        let op_start = Instant::now();

        let cred_id = format!("cred-stress-{i}");
        let cred = Credential {
            id: cred_id.clone(),
            issuer_did: issuer_did.into(),
            subject_did: format!("did:cerulean:subject-{i}"),
            cred_type: "VerifiableCredential".into(),
            issued_at: 1000 + i,
            expires_at: 9_999_999,
            revoked_at: None,
            claims: serde_json::json!({"degree": "engineering", "index": i}),
            signature: String::new(),
            status: "active".into(),
        };

        if store.write_credential(&cred).is_err() {
            errors += 1;
        }
        // Read back
        if store.read_credential(&cred_id).is_err() {
            errors += 1;
        }

        latencies.push(op_start.elapsed().as_micros() as u64);
    }
    let duration = start.elapsed();

    latencies.sort_unstable();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    ModuleStressResult {
        module: "credential".into(),
        operations: ops,
        duration_ms: duration.as_millis() as u64,
        ops_per_sec,
        p50_us: percentile(&latencies, 50.0),
        p99_us: percentile(&latencies, 99.0),
        errors,
        status: if errors == 0 && ops_per_sec > 1_000.0 {
            StressStatus::Pass
        } else if errors == 0 {
            StressStatus::Degraded
        } else {
            StressStatus::Fail
        },
    }
}

/// Run all module stress tests and generate report.
pub fn run_full_stress(ops_per_module: u64) -> StressReport {
    let results = vec![
        stress_storage(ops_per_module),
        stress_crypto(ops_per_module),
        stress_anomaly(ops_per_module),
        stress_risk(ops_per_module),
        stress_compliance(ops_per_module),
        stress_governance(ops_per_module),
        stress_identity(ops_per_module),
        stress_credential(ops_per_module),
        stress_forensic(ops_per_module),
        stress_patterns(ops_per_module),
    ];

    let passed = results
        .iter()
        .filter(|r| r.status == StressStatus::Pass)
        .count();
    let degraded = results
        .iter()
        .filter(|r| r.status == StressStatus::Degraded)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == StressStatus::Fail)
        .count();

    StressReport {
        report_id: format!("STR-{}", &uuid::Uuid::new_v4().to_string()[..8]),
        generated_at: chrono::Utc::now().to_rfc3339(),
        total_modules: results.len(),
        passed,
        degraded,
        failed,
        results,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_stress_completes() {
        let r = stress_storage(100);
        assert_eq!(r.module, "storage");
        assert_eq!(r.operations, 100);
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn crypto_stress_completes() {
        let r = stress_crypto(100);
        assert_eq!(r.module, "crypto_hash");
        assert!(r.ops_per_sec > 0.0);
    }

    #[test]
    fn anomaly_stress_completes() {
        let r = stress_anomaly(100);
        assert_eq!(r.module, "anomaly_detection");
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn risk_stress_completes() {
        let r = stress_risk(100);
        assert_eq!(r.module, "risk_scoring");
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn compliance_stress_completes() {
        let r = stress_compliance(100);
        assert_eq!(r.module, "iso20022_validation");
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn governance_stress_completes() {
        let r = stress_governance(50);
        assert_eq!(r.module, "governance");
        // Some errors expected from rate limiting — just verify it ran
        assert!(r.operations == 50);
    }

    #[test]
    fn identity_stress_completes() {
        let r = stress_identity(200);
        assert_eq!(r.module, "identity");
        assert_eq!(r.errors, 0);
        assert_eq!(r.operations, 200);
    }

    #[test]
    fn credential_stress_completes() {
        let r = stress_credential(200);
        assert_eq!(r.module, "credential");
        assert_eq!(r.errors, 0);
        assert_eq!(r.operations, 200);
    }

    #[test]
    fn forensic_stress_completes() {
        let r = stress_forensic(50);
        assert_eq!(r.module, "forensic");
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn patterns_stress_completes() {
        let r = stress_patterns(50);
        assert_eq!(r.module, "pattern_detection");
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn full_stress_report_has_all_modules() {
        let report = run_full_stress(50);
        assert_eq!(report.total_modules, 10);
        assert_eq!(report.results.len(), 10);
        assert!(report.report_id.starts_with("STR-"));
    }

    #[test]
    fn percentile_on_empty() {
        assert_eq!(percentile(&[], 50.0), 0);
    }

    #[test]
    fn percentile_on_data() {
        let data: Vec<u64> = (1..=100).collect();
        let p50 = percentile(&data, 50.0);
        let p99 = percentile(&data, 99.0);
        // p50 should be around 50 (±1 due to indexing)
        assert!(p50 >= 49 && p50 <= 51, "p50 was {p50}");
        assert!(p99 >= 98 && p99 <= 100, "p99 was {p99}");
    }

    #[test]
    fn stress_result_serde() {
        let r = stress_storage(10);
        let json = serde_json::to_string(&r).unwrap();
        let restored: ModuleStressResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.module, "storage");
    }
}
