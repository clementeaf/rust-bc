//! Approved RNG wrapper using OS-backed randomness.

use crate::errors::CryptoError;

/// Fill `buf` with cryptographically secure random bytes from the OS.
///
/// Uses `getrandom` (OS-backed CSPRNG). Returns explicit error on failure.
pub fn fill_random(buf: &mut [u8]) -> Result<(), CryptoError> {
    use rand::RngCore;
    rand::rngs::OsRng
        .try_fill_bytes(buf)
        .map_err(|e| CryptoError::RngFailure(e.to_string()))
}

/// Generate `n` random bytes.
pub fn random_bytes(n: usize) -> Result<Vec<u8>, CryptoError> {
    let mut buf = vec![0u8; n];
    fill_random(&mut buf)?;
    Ok(buf)
}

/// Continuous RNG test: generate two consecutive outputs and verify they differ.
/// This is a basic FIPS-oriented sanity check (NIST SP 800-90B §4.3).
pub fn continuous_rng_test() -> Result<(), CryptoError> {
    let a = random_bytes(32)?;
    let b = random_bytes(32)?;
    if a == b {
        return Err(CryptoError::RngFailure(
            "continuous RNG test failed: repeated output".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_produces_non_empty_output() {
        let bytes = random_bytes(32).unwrap();
        assert_eq!(bytes.len(), 32);
        assert!(
            bytes.iter().any(|&b| b != 0),
            "RNG output should not be all zeros"
        );
    }

    #[test]
    fn continuous_rng_test_passes() {
        continuous_rng_test().unwrap();
    }

    #[test]
    fn two_random_outputs_differ() {
        let a = random_bytes(32).unwrap();
        let b = random_bytes(32).unwrap();
        assert_ne!(a, b);
    }
}
