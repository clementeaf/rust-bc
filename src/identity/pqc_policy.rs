//! Post-quantum cryptography enforcement policy.
//!
//! When `REQUIRE_PQC_SIGNATURES=true`, the node rejects any block,
//! endorsement, or alive message that uses a classical (non-PQC)
//! signing algorithm.

use crate::identity::signing::SigningAlgorithm;

/// Returns `true` when the `REQUIRE_PQC_SIGNATURES` env var is set to a truthy value.
pub fn pqc_required() -> bool {
    std::env::var("REQUIRE_PQC_SIGNATURES")
        .map(|v| matches!(v.as_str(), "true" | "1" | "yes"))
        .unwrap_or(false)
}

/// Validate that `algorithm` satisfies the PQC policy.
///
/// Returns `Ok(())` when PQC is not required or when the algorithm is post-quantum.
/// Returns `Err` with a descriptive message when a classical algorithm is used
/// and PQC is mandatory.
pub fn enforce_pqc(algorithm: SigningAlgorithm, context: &str) -> Result<(), String> {
    if pqc_required() && !algorithm.is_post_quantum() {
        Err(format!(
            "PQC policy violation: {context} uses {algorithm}, but REQUIRE_PQC_SIGNATURES is enabled"
        ))
    } else {
        Ok(())
    }
}

/// Validate that the declared `algorithm` is consistent with the actual
/// signature byte length. Prevents tag forgery where an attacker declares
/// `MlDsa65` but provides a 64-byte classical signature.
///
/// Known sizes:
/// - Ed25519: 64 bytes
/// - ML-DSA-65: 3309 bytes
pub fn validate_signature_consistency(
    algorithm: SigningAlgorithm,
    signature: &[u8],
    context: &str,
) -> Result<(), String> {
    let expected_len = match algorithm {
        SigningAlgorithm::Ed25519 => 64,
        SigningAlgorithm::MlDsa65 => 3309,
    };
    if signature.len() != expected_len {
        Err(format!(
            "signature/algorithm mismatch in {context}: declared {algorithm} expects {expected_len} bytes, got {}",
            signature.len()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pqc_not_required_allows_ed25519() {
        // Default: env var not set → allows everything.
        assert!(enforce_pqc(SigningAlgorithm::Ed25519, "test").is_ok());
        assert!(enforce_pqc(SigningAlgorithm::MlDsa65, "test").is_ok());
    }

    #[test]
    fn is_post_quantum_checks() {
        assert!(!SigningAlgorithm::Ed25519.is_post_quantum());
        assert!(SigningAlgorithm::MlDsa65.is_post_quantum());
    }

    #[test]
    fn enforce_pqc_rejects_classical_when_required() {
        std::env::set_var("REQUIRE_PQC_SIGNATURES", "true");
        let result = enforce_pqc(SigningAlgorithm::Ed25519, "block signature");
        std::env::remove_var("REQUIRE_PQC_SIGNATURES");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("PQC policy violation"));
    }

    #[test]
    fn enforce_pqc_allows_mldsa_when_required() {
        std::env::set_var("REQUIRE_PQC_SIGNATURES", "true");
        let result = enforce_pqc(SigningAlgorithm::MlDsa65, "block signature");
        std::env::remove_var("REQUIRE_PQC_SIGNATURES");
        assert!(result.is_ok());
    }

    #[test]
    fn pqc_required_parses_truthy_values() {
        for val in &["true", "1", "yes"] {
            std::env::set_var("REQUIRE_PQC_SIGNATURES", val);
            assert!(pqc_required(), "expected true for '{val}'");
        }
        for val in &["false", "0", "no", ""] {
            std::env::set_var("REQUIRE_PQC_SIGNATURES", val);
            assert!(!pqc_required(), "expected false for '{val}'");
        }
        std::env::remove_var("REQUIRE_PQC_SIGNATURES");
    }
}
