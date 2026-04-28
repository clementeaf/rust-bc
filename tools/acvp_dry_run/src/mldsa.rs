//! ML-DSA-65 ACVP vector runner.

use crate::vectors::{AlgorithmReport, TestResult, VectorFile};

pub fn run(vectors: &VectorFile) -> AlgorithmReport {
    let mut results = Vec::new();
    let mut passed = 0u32;
    let mut failed = 0u32;

    for group in &vectors.test_groups {
        for tc in &group.tests {
            let mode = tc.mode.as_deref().unwrap_or("signThenVerify");
            let msg_hex = tc.message_hex.as_deref().unwrap_or("");
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

            match mode {
                "signThenVerify" => match run_sign_then_verify(&msg) {
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
        algorithm: "ml-dsa-65".into(),
        passed,
        failed,
        results,
    }
}

fn run_sign_then_verify(msg: &[u8]) -> Result<String, String> {
    let kp = pqc_crypto_module::api::generate_mldsa_keypair()
        .map_err(|e| format!("keygen failed: {e}"))?;

    let sig = pqc_crypto_module::api::sign_message(&kp.private_key, msg)
        .map_err(|e| format!("sign failed: {e}"))?;

    pqc_crypto_module::api::verify_signature(&kp.public_key, msg, &sig)
        .map_err(|e| format!("verify failed: {e}"))?;

    // Verify corrupted signature is rejected
    let mut bad = sig.as_bytes().to_vec();
    bad[0] ^= 0xff;
    let bad_sig = pqc_crypto_module::types::MldsaSignature(bad);
    if pqc_crypto_module::api::verify_signature(&kp.public_key, msg, &bad_sig).is_ok() {
        return Err("corrupted signature was accepted".into());
    }

    Ok(format!(
        "pk={}B sig={}B",
        kp.public_key.as_bytes().len(),
        sig.as_bytes().len()
    ))
}
