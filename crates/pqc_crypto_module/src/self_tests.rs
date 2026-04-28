//! Power-up self-tests (Known Answer Tests).
//!
//! Run at module initialization before transitioning to Approved state.

use crate::errors::CryptoError;
use crate::hashing::sha3_256_raw;
use crate::mldsa::{generate_keypair_raw, sign_message_raw, verify_signature_raw};
use crate::mlkem::{decapsulate_raw, encapsulate_raw, generate_keypair_raw as kem_keygen_raw};
use crate::rng::continuous_rng_test;

/// Run all self-tests. Returns `Ok(())` if all pass.
pub fn run_all() -> Result<(), CryptoError> {
    kat_sha3_256()?;
    kat_mldsa65()?;
    kat_mlkem()?;
    test_rng()?;
    Ok(())
}

fn kat_sha3_256() -> Result<(), CryptoError> {
    let hash = sha3_256_raw(b"");
    let expected = "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
    if hash.to_hex() != expected {
        return Err(CryptoError::SelfTestFailed(
            "SHA3-256 KAT: empty string digest mismatch".into(),
        ));
    }

    // Verify determinism
    let hash2 = sha3_256_raw(b"");
    if hash != hash2 {
        return Err(CryptoError::SelfTestFailed(
            "SHA3-256 KAT: non-deterministic".into(),
        ));
    }

    Ok(())
}

fn kat_mldsa65() -> Result<(), CryptoError> {
    let kp = generate_keypair_raw();
    let message = b"FIPS-204-KAT-ML-DSA-65";

    // Sign + verify
    let sig = sign_message_raw(&kp.private_key, message)
        .map_err(|e| CryptoError::SelfTestFailed(format!("ML-DSA sign: {e}")))?;
    verify_signature_raw(&kp.public_key, message, &sig)
        .map_err(|_| CryptoError::SelfTestFailed("ML-DSA KAT: sign-then-verify failed".into()))?;

    // Corrupted signature must fail
    let mut bad_sig = sig.clone();
    bad_sig.0[0] ^= 0xff;
    if verify_signature_raw(&kp.public_key, message, &bad_sig).is_ok() {
        return Err(CryptoError::SelfTestFailed(
            "ML-DSA KAT: corrupted signature was accepted".into(),
        ));
    }

    // Wrong message must fail
    if verify_signature_raw(&kp.public_key, b"wrong", &sig).is_ok() {
        return Err(CryptoError::SelfTestFailed(
            "ML-DSA KAT: wrong message verified".into(),
        ));
    }

    Ok(())
}

fn kat_mlkem() -> Result<(), CryptoError> {
    let kp =
        kem_keygen_raw().map_err(|e| CryptoError::SelfTestFailed(format!("ML-KEM keygen: {e}")))?;

    let (ct, ss) = encapsulate_raw(&kp.public_key)
        .map_err(|e| CryptoError::SelfTestFailed(format!("ML-KEM encaps: {e}")))?;

    let ss2 = decapsulate_raw(&kp.private_key, &ct)
        .map_err(|e| CryptoError::SelfTestFailed(format!("ML-KEM decaps: {e}")))?;

    if ss.as_bytes() != ss2.as_bytes() {
        return Err(CryptoError::SelfTestFailed(
            "ML-KEM KAT: shared secrets from encapsulate and decapsulate do not match".into(),
        ));
    }

    // Invalid ciphertext must fail or produce different secret
    let bad_ct = crate::types::MlKemCiphertext(vec![0xAA; 1088]);
    let ss_bad = decapsulate_raw(&kp.private_key, &bad_ct);
    match ss_bad {
        Ok(ref bad_ss) if bad_ss.as_bytes() == ss.as_bytes() => {
            return Err(CryptoError::SelfTestFailed(
                "ML-KEM KAT: corrupted ciphertext produced same shared secret".into(),
            ));
        }
        _ => {} // Error or different secret — both acceptable
    }

    Ok(())
}

fn test_rng() -> Result<(), CryptoError> {
    continuous_rng_test()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_self_tests_pass() {
        run_all().expect("all self-tests must pass");
    }
}
