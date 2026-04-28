//! Dual-signing support for crypto-agility migration.
//!
//! During a PQC transition, blocks and endorsements can carry both a classical
//! and a post-quantum signature. This module provides helpers to:
//!
//! - Sign data with two providers simultaneously
//! - Verify that at least one (or both) signatures are valid

use crate::identity::signing::{SigningAlgorithm, SigningError, SigningProvider};

/// Result of signing data with two providers.
pub struct DualSignature {
    pub primary_signature: Vec<u8>,
    pub primary_algorithm: SigningAlgorithm,
    pub secondary_signature: Vec<u8>,
    pub secondary_algorithm: SigningAlgorithm,
}

/// Sign `data` with both a primary and secondary provider.
pub fn dual_sign(
    data: &[u8],
    primary: &dyn SigningProvider,
    secondary: &dyn SigningProvider,
) -> Result<DualSignature, SigningError> {
    Ok(DualSignature {
        primary_signature: primary.sign(data)?,
        primary_algorithm: primary.algorithm(),
        secondary_signature: secondary.sign(data)?,
        secondary_algorithm: secondary.algorithm(),
    })
}

/// Verification mode for dual-signed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualVerifyMode {
    /// Accept if either signature is valid (transition period).
    Either,
    /// Require both signatures to be valid (strict migration).
    Both,
}

/// Returns the dual-verify mode from `DUAL_SIGN_VERIFY_MODE` env var.
///
/// - `"both"` → `Both`
/// - anything else or unset → `Either`
pub fn dual_verify_mode() -> DualVerifyMode {
    match std::env::var("DUAL_SIGN_VERIFY_MODE")
        .unwrap_or_default()
        .as_str()
    {
        "both" => DualVerifyMode::Both,
        _ => DualVerifyMode::Either,
    }
}

/// Verify dual signatures according to the given mode.
///
/// `primary_verify` and `secondary_verify` are closures that return `Ok(true)` on
/// successful verification, `Ok(false)` on mismatch, or `Err` on structural errors.
pub fn verify_dual<F1, F2>(
    primary_verify: F1,
    secondary_verify: Option<F2>,
    mode: DualVerifyMode,
) -> Result<bool, SigningError>
where
    F1: FnOnce() -> Result<bool, SigningError>,
    F2: FnOnce() -> Result<bool, SigningError>,
{
    let primary_ok = primary_verify()?;

    match secondary_verify {
        Some(sec_fn) => {
            let secondary_ok = sec_fn()?;
            match mode {
                DualVerifyMode::Either => Ok(primary_ok || secondary_ok),
                DualVerifyMode::Both => Ok(primary_ok && secondary_ok),
            }
        }
        // No secondary signature — accept based on primary only.
        None => Ok(primary_ok),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::signing::{MlDsaSigningProvider, SoftwareSigningProvider};

    #[test]
    fn dual_sign_produces_two_signatures() {
        let ed = SoftwareSigningProvider::generate();
        let pqc = MlDsaSigningProvider::generate();
        let result = dual_sign(b"hello", &ed, &pqc).unwrap();
        assert_eq!(result.primary_algorithm, SigningAlgorithm::Ed25519);
        assert_eq!(result.secondary_algorithm, SigningAlgorithm::MlDsa65);
        assert_eq!(result.primary_signature.len(), 64);
        assert_eq!(result.secondary_signature.len(), 3309);
    }

    #[test]
    fn verify_dual_either_mode_accepts_one_valid() {
        let result = verify_dual(|| Ok(true), Some(|| Ok(false)), DualVerifyMode::Either).unwrap();
        assert!(result);

        let result = verify_dual(|| Ok(false), Some(|| Ok(true)), DualVerifyMode::Either).unwrap();
        assert!(result);
    }

    #[test]
    fn verify_dual_both_mode_requires_all() {
        let result = verify_dual(|| Ok(true), Some(|| Ok(false)), DualVerifyMode::Both).unwrap();
        assert!(!result);

        let result = verify_dual(|| Ok(true), Some(|| Ok(true)), DualVerifyMode::Both).unwrap();
        assert!(result);
    }

    #[test]
    fn verify_dual_no_secondary_accepts_primary() {
        let result = verify_dual(
            || Ok(true),
            None::<fn() -> Result<bool, SigningError>>,
            DualVerifyMode::Both,
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn dual_sign_and_verify_roundtrip() {
        let ed = SoftwareSigningProvider::generate();
        let pqc = MlDsaSigningProvider::generate();
        let data = b"migration test";
        let ds = dual_sign(data, &ed, &pqc).unwrap();

        let ok = verify_dual(
            || ed.verify(data, &ds.primary_signature),
            Some(|| pqc.verify(data, &ds.secondary_signature)),
            DualVerifyMode::Both,
        )
        .unwrap();
        assert!(ok);
    }

    #[test]
    fn dual_verify_mode_defaults_to_either() {
        std::env::remove_var("DUAL_SIGN_VERIFY_MODE");
        assert_eq!(dual_verify_mode(), DualVerifyMode::Either);
    }

    #[test]
    fn dual_verify_mode_parses_both() {
        std::env::set_var("DUAL_SIGN_VERIFY_MODE", "both");
        assert_eq!(dual_verify_mode(), DualVerifyMode::Both);
        std::env::remove_var("DUAL_SIGN_VERIFY_MODE");
    }
}
