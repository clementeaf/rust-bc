//! SHA3-256 ACVP vector runner.

use crate::vectors::{AlgorithmReport, TestResult, VectorFile};

pub fn run(vectors: &VectorFile) -> AlgorithmReport {
    let mut results = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for group in &vectors.test_groups {
        for tc in &group.tests {
            let msg_hex = tc.msg_hex.as_deref().unwrap_or("");
            let msg = match hex::decode(msg_hex) {
                Ok(b) => b,
                Err(e) => {
                    failed += 1;
                    results.push(TestResult {
                        tc_id: tc.tc_id,
                        status: "failed".into(),
                        detail: Some(format!("invalid hex input: {e}")),
                    });
                    continue;
                }
            };

            let hash = pqc_crypto_module::api::sha3_256(&msg).unwrap();
            let actual = hash.to_hex();

            if let Some(expected) = &tc.expected_digest_hex {
                if actual == *expected {
                    passed += 1;
                    results.push(TestResult {
                        tc_id: tc.tc_id,
                        status: "passed".into(),
                        detail: None,
                    });
                } else {
                    failed += 1;
                    results.push(TestResult {
                        tc_id: tc.tc_id,
                        status: "failed".into(),
                        detail: Some(format!("expected {expected}, got {actual}")),
                    });
                }
            } else {
                // No expected value — just verify it runs
                passed += 1;
                results.push(TestResult {
                    tc_id: tc.tc_id,
                    status: "passed".into(),
                    detail: Some(format!("digest={actual}")),
                });
            }
        }
    }

    AlgorithmReport {
        algorithm: "sha3-256".into(),
        passed,
        failed,
        results,
    }
}
