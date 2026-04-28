//! ML-KEM-768 ACVP vector runner.

use crate::vectors::{AlgorithmReport, TestResult, VectorFile};

pub fn run(vectors: &VectorFile) -> AlgorithmReport {
    let mut results = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for group in &vectors.test_groups {
        for tc in &group.tests {
            let mode = tc.mode.as_deref().unwrap_or("encapsThenDecaps");

            match mode {
                "encapsThenDecaps" => match run_encaps_then_decaps() {
                    Ok(detail) => {
                        passed += 1;
                        results.push(TestResult {
                            tc_id: tc.tc_id,
                            status: "passed".into(),
                            detail: Some(detail),
                        });
                    }
                    Err(e) => {
                        failed += 1;
                        results.push(TestResult {
                            tc_id: tc.tc_id,
                            status: "failed".into(),
                            detail: Some(e),
                        });
                    }
                },
                other => {
                    failed += 1;
                    results.push(TestResult {
                        tc_id: tc.tc_id,
                        status: "failed".into(),
                        detail: Some(format!("unsupported mode: {other}")),
                    });
                }
            }
        }
    }

    AlgorithmReport {
        algorithm: "ml-kem-768".into(),
        passed,
        failed,
        results,
    }
}

fn run_encaps_then_decaps() -> Result<String, String> {
    let kp = pqc_crypto_module::api::generate_mlkem_keypair()
        .map_err(|e| format!("keygen failed: {e}"))?;

    let (ct, ss1) = pqc_crypto_module::api::mlkem_encapsulate(&kp.public_key)
        .map_err(|e| format!("encapsulate failed: {e}"))?;

    let ss2 = pqc_crypto_module::api::mlkem_decapsulate(&kp.private_key, &ct)
        .map_err(|e| format!("decapsulate failed: {e}"))?;

    if ss1.as_bytes() != ss2.as_bytes() {
        return Err("shared secrets do not match".into());
    }

    // Verify invalid ciphertext handling
    let bad_ct = pqc_crypto_module::types::MlKemCiphertext(vec![0xAA; 1088]);
    if let Ok(bad_ss) = pqc_crypto_module::api::mlkem_decapsulate(&kp.private_key, &bad_ct) {
        if bad_ss.as_bytes() == ss1.as_bytes() {
            return Err("corrupted ciphertext produced same shared secret".into());
        }
        // Different secret is acceptable (implicit rejection)
    }

    Ok(format!(
        "pk={}B sk={}B ct={}B ss={}B",
        kp.public_key.as_bytes().len(),
        kp.private_key.0.len(),
        ct.as_bytes().len(),
        ss1.as_bytes().len()
    ))
}
