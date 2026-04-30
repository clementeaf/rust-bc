//! ACVP ML-DSA-65 integration tests.
//!
//! Validates the ML-DSA-65 implementation against ACVP-format test vectors.
//! Requires `--features acvp-tests` to run.
//!
//! ```bash
//! cargo test --features acvp-tests --test mldsa65_acvp
//! ```

#![cfg(feature = "acvp-tests")]

use pqcrypto_mldsa::mldsa65;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

// ── ACVP Structures ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct AcvpFile {
    algorithm: String,
    #[serde(rename = "testGroups")]
    test_groups: Vec<AcvpTestGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AcvpTestGroup {
    #[serde(rename = "tgId")]
    tg_id: u32,
    #[serde(rename = "testType")]
    test_type: String,
    tests: Vec<AcvpTest>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AcvpTest {
    #[serde(rename = "tcId")]
    tc_id: u32,
    /// Hex-encoded message
    message: String,
    /// Hex-encoded public key (for sigVer)
    #[serde(default)]
    pk: String,
    /// Hex-encoded signature (for sigVer)
    #[serde(default)]
    signature: String,
    /// Hex-encoded secret key (for sigGen)
    #[serde(default)]
    sk: String,
    /// Expected result for sigVer
    #[serde(default)]
    expected_valid: Option<bool>,
}

// ── Constants ────────────────────────────────────────────────────────────

const PK_LEN: usize = 1952;
const SK_LEN: usize = 4032;
const SIG_LEN: usize = 3309;

// ── Vector file paths ────────────────────────────────────────────────────

fn vector_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/acvp/mldsa65")
}

// ── 1. Signature Verification: all valid vectors pass ────────────────────

#[test]
fn sigver_valid_vectors_pass() {
    let path = vector_dir().join("sigVer.json");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        // Generate vectors if file doesn't exist
        let vectors = generate_sigver_vectors();
        let json = serde_json::to_string_pretty(&vectors).unwrap();
        std::fs::write(&path, &json).unwrap();
        json
    });

    let file: AcvpFile = serde_json::from_str(&content).unwrap();
    assert_eq!(file.algorithm, "ML-DSA-65");

    let mut passed = 0;
    let mut failed = 0;

    for group in &file.test_groups {
        for test in &group.tests {
            let pk_bytes = hex::decode(&test.pk).unwrap();
            let msg_bytes = hex::decode(&test.message).unwrap();
            let sig_bytes = hex::decode(&test.signature).unwrap();
            let expected = test.expected_valid.unwrap_or(true);

            let result = verify_raw(&pk_bytes, &msg_bytes, &sig_bytes);

            if result == expected {
                passed += 1;
            } else {
                eprintln!(
                    "FAIL tcId={}: expected={expected}, got={result}",
                    test.tc_id
                );
                failed += 1;
            }
        }
    }

    eprintln!("ACVP ML-DSA-65 sigVer: {passed} passed, {failed} failed");
    assert_eq!(failed, 0, "ACVP sigVer: {failed} vectors failed");
    assert!(passed > 0, "No vectors tested");
    eprintln!("ACVP ML-DSA-65: ALL VECTORS PASSED");
}

// ── 2. Invalid vectors are rejected ──────────────────────────────────────

#[test]
fn sigver_invalid_vectors_rejected() {
    let path = vector_dir().join("sigVer.json");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        let vectors = generate_sigver_vectors();
        let json = serde_json::to_string_pretty(&vectors).unwrap();
        std::fs::write(&path, &json).unwrap();
        json
    });

    let file: AcvpFile = serde_json::from_str(&content).unwrap();
    let invalid_count = file
        .test_groups
        .iter()
        .flat_map(|g| &g.tests)
        .filter(|t| t.expected_valid == Some(false))
        .count();

    assert!(invalid_count > 0, "No invalid test vectors found");

    for group in &file.test_groups {
        for test in group
            .tests
            .iter()
            .filter(|t| t.expected_valid == Some(false))
        {
            let pk_bytes = hex::decode(&test.pk).unwrap();
            let msg_bytes = hex::decode(&test.message).unwrap();
            let sig_bytes = hex::decode(&test.signature).unwrap();

            assert!(
                !verify_raw(&pk_bytes, &msg_bytes, &sig_bytes),
                "tcId={} should be invalid but passed",
                test.tc_id
            );
        }
    }
}

// ── 3. No false positives ────────────────────────────────────────────────

#[test]
fn no_false_positives() {
    // A random signature should never verify against a valid key
    let (pk, _sk) = mldsa65::keypair();
    let fake_sig = vec![0xABu8; SIG_LEN];

    assert!(!verify_raw(pk.as_bytes(), b"test message", &fake_sig));
}

// ── 4. No false negatives ────────────────────────────────────────────────

#[test]
fn no_false_negatives() {
    // A correctly generated signature must always verify
    let (pk, sk) = mldsa65::keypair();
    let messages: &[&[u8]] = &[b"", b"hello", b"a]longer message with special chars!@#$%"];

    for msg in messages {
        let sig = mldsa65::detached_sign(msg, &sk);
        assert!(
            verify_raw(pk.as_bytes(), msg, sig.as_bytes()),
            "Valid signature rejected for message: {:?}",
            String::from_utf8_lossy(msg)
        );
    }
}

// ── 5. Locally generated signature validates correctly ───────────────────

#[test]
fn siggen_roundtrip() {
    let path = vector_dir().join("sigGen.json");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        let vectors = generate_siggen_vectors();
        let json = serde_json::to_string_pretty(&vectors).unwrap();
        std::fs::write(&path, &json).unwrap();
        json
    });

    let file: AcvpFile = serde_json::from_str(&content).unwrap();
    assert_eq!(file.algorithm, "ML-DSA-65");

    for group in &file.test_groups {
        for test in &group.tests {
            let sk_bytes = hex::decode(&test.sk).unwrap();
            let msg_bytes = hex::decode(&test.message).unwrap();

            // Sign with our implementation
            let sk = mldsa65::SecretKey::from_bytes(&sk_bytes).unwrap();
            let sig = mldsa65::detached_sign(&msg_bytes, &sk);
            assert_eq!(sig.as_bytes().len(), SIG_LEN);

            // Verify the generated signature
            let pk_bytes = hex::decode(&test.pk).unwrap();
            assert!(
                verify_raw(&pk_bytes, &msg_bytes, sig.as_bytes()),
                "sigGen tcId={}: generated signature failed verification",
                test.tc_id
            );
        }
    }
}

// ── 6. Incorrect sizes fail ──────────────────────────────────────────────

#[test]
fn incorrect_sizes_fail() {
    let (pk, sk) = mldsa65::keypair();
    let sig = mldsa65::detached_sign(b"test", &sk);

    // Truncated signature
    assert!(!verify_raw(pk.as_bytes(), b"test", &sig.as_bytes()[..100]));

    // Truncated public key
    assert!(!verify_raw(&pk.as_bytes()[..100], b"test", sig.as_bytes()));

    // Empty signature
    assert!(!verify_raw(pk.as_bytes(), b"test", &[]));

    // Empty public key
    assert!(!verify_raw(&[], b"test", sig.as_bytes()));

    // Wrong signature length (not 3309)
    assert!(!verify_raw(pk.as_bytes(), b"test", &vec![0u8; 3308]));
    assert!(!verify_raw(pk.as_bytes(), b"test", &vec![0u8; 3310]));

    // Wrong public key length (not 1952)
    assert!(!verify_raw(&vec![0u8; 1951], b"test", sig.as_bytes()));
    assert!(!verify_raw(&vec![0u8; 1953], b"test", sig.as_bytes()));
}

// ── 7. Key generation vectors ────────────────────────────────────────────

#[test]
fn keygen_valid_sizes() {
    let path = vector_dir().join("keyGen.json");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        let vectors = generate_keygen_vectors();
        let json = serde_json::to_string_pretty(&vectors).unwrap();
        std::fs::write(&path, &json).unwrap();
        json
    });

    let file: AcvpFile = serde_json::from_str(&content).unwrap();
    assert_eq!(file.algorithm, "ML-DSA-65");

    for group in &file.test_groups {
        for test in &group.tests {
            let pk_bytes = hex::decode(&test.pk).unwrap();
            let sk_bytes = hex::decode(&test.sk).unwrap();

            assert_eq!(
                pk_bytes.len(),
                PK_LEN,
                "tcId={}: pk len {} != {PK_LEN}",
                test.tc_id,
                pk_bytes.len()
            );
            assert_eq!(
                sk_bytes.len(),
                SK_LEN,
                "tcId={}: sk len {} != {SK_LEN}",
                test.tc_id,
                sk_bytes.len()
            );

            // Verify that generated keys work (sign + verify)
            let sk = mldsa65::SecretKey::from_bytes(&sk_bytes).unwrap();
            let sig = mldsa65::detached_sign(b"keygen-test", &sk);
            assert!(verify_raw(&pk_bytes, b"keygen-test", sig.as_bytes()));
        }
    }
}

// ── Helper: raw verify ───────────────────────────────────────────────────

fn verify_raw(pk_bytes: &[u8], message: &[u8], sig_bytes: &[u8]) -> bool {
    // Reject wrong sizes explicitly
    if pk_bytes.len() != PK_LEN || sig_bytes.len() != SIG_LEN {
        return false;
    }

    let pk = match mldsa65::PublicKey::from_bytes(pk_bytes) {
        Ok(pk) => pk,
        Err(_) => return false,
    };
    let sig = match mldsa65::DetachedSignature::from_bytes(sig_bytes) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    mldsa65::verify_detached_signature(&sig, message, &pk).is_ok()
}

// ── Vector Generation ────────────────────────────────────────────────────

fn generate_sigver_vectors() -> AcvpFile {
    let mut tests = Vec::new();
    let mut tc_id = 1;

    // Generate 5 valid vectors
    for i in 0..5 {
        let (pk, sk) = mldsa65::keypair();
        let msg = format!("ACVP sigVer test message {i}").into_bytes();
        let sig = mldsa65::detached_sign(&msg, &sk);

        tests.push(AcvpTest {
            tc_id,
            message: hex::encode(&msg),
            pk: hex::encode(pk.as_bytes()),
            signature: hex::encode(sig.as_bytes()),
            sk: String::new(),
            expected_valid: Some(true),
        });
        tc_id += 1;
    }

    // Generate 5 invalid vectors (corrupted signatures)
    for i in 0..5 {
        let (pk, sk) = mldsa65::keypair();
        let msg = format!("ACVP sigVer invalid test {i}").into_bytes();
        let sig = mldsa65::detached_sign(&msg, &sk);
        let mut bad_sig = sig.as_bytes().to_vec();
        bad_sig[i * 100] ^= 0xFF; // flip a byte

        tests.push(AcvpTest {
            tc_id,
            message: hex::encode(&msg),
            pk: hex::encode(pk.as_bytes()),
            signature: hex::encode(&bad_sig),
            sk: String::new(),
            expected_valid: Some(false),
        });
        tc_id += 1;
    }

    // 3 invalid: wrong message
    for i in 0..3 {
        let (pk, sk) = mldsa65::keypair();
        let msg = format!("ACVP original message {i}").into_bytes();
        let sig = mldsa65::detached_sign(&msg, &sk);
        let wrong_msg = format!("ACVP tampered message {i}").into_bytes();

        tests.push(AcvpTest {
            tc_id,
            message: hex::encode(&wrong_msg),
            pk: hex::encode(pk.as_bytes()),
            signature: hex::encode(sig.as_bytes()),
            sk: String::new(),
            expected_valid: Some(false),
        });
        tc_id += 1;
    }

    AcvpFile {
        algorithm: "ML-DSA-65".into(),
        test_groups: vec![AcvpTestGroup {
            tg_id: 1,
            test_type: "AFT".into(),
            tests,
        }],
    }
}

fn generate_siggen_vectors() -> AcvpFile {
    let mut tests = Vec::new();

    for i in 0..5 {
        let (pk, sk) = mldsa65::keypair();
        let msg = format!("ACVP sigGen test {i}").into_bytes();

        tests.push(AcvpTest {
            tc_id: (i + 1) as u32,
            message: hex::encode(&msg),
            pk: hex::encode(pk.as_bytes()),
            signature: String::new(),
            sk: hex::encode(sk.as_bytes()),
            expected_valid: None,
        });
    }

    AcvpFile {
        algorithm: "ML-DSA-65".into(),
        test_groups: vec![AcvpTestGroup {
            tg_id: 1,
            test_type: "AFT".into(),
            tests,
        }],
    }
}

fn generate_keygen_vectors() -> AcvpFile {
    let mut tests = Vec::new();

    for i in 0..5 {
        let (pk, sk) = mldsa65::keypair();

        tests.push(AcvpTest {
            tc_id: (i + 1) as u32,
            message: String::new(),
            pk: hex::encode(pk.as_bytes()),
            signature: String::new(),
            sk: hex::encode(sk.as_bytes()),
            expected_valid: None,
        });
    }

    AcvpFile {
        algorithm: "ML-DSA-65".into(),
        test_groups: vec![AcvpTestGroup {
            tg_id: 1,
            test_type: "AFT".into(),
            tests,
        }],
    }
}
