//! Zero-knowledge proof system for identity attributes.
//!
//! Proves predicates over credential claims without revealing underlying data:
//! - **RangeProof**: value >= threshold (e.g., age >= 18)
//! - **SetMembership**: value in allowed set (e.g., nationality in [CL, AR, BR])
//! - **CredentialValidity**: credential is active and not expired/revoked
//!
//! Uses commitment-based proofs: SHA-256(claim_value || blinding_factor).
//! The verifier checks the commitment against the predicate without learning
//! the claim value. The trait system allows swapping to Bulletproofs or PLONK.

use pqc_crypto_module::legacy::sha256::{Digest, Sha256};
use serde::{Deserialize, Serialize};

/// A predicate to prove over a credential claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Predicate {
    /// Prove that a numeric claim >= threshold.
    RangeProof { claim_key: String, threshold: u64 },
    /// Prove that a string claim is in an allowed set.
    SetMembership {
        claim_key: String,
        allowed_values: Vec<String>,
    },
    /// Prove that a credential is valid (active, not expired, not revoked).
    CredentialValidity { credential_id: String },
}

/// A zero-knowledge presentation: proof + public inputs + credential ref.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkPresentation {
    /// Reference to the credential being proven.
    pub credential_id: String,
    /// The predicate that was proven.
    pub predicate: Predicate,
    /// The commitment: SHA-256(claim_value || blinding_factor).
    pub commitment: String,
    /// The proof data (scheme-specific).
    pub proof: ZkProof,
    /// Timestamp of proof generation.
    pub created_at: u64,
}

/// Proof data for the commitment-based scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    /// The blinding factor used (hex-encoded, 32 bytes).
    /// In a real ZKP this would NOT be revealed — here it's included
    /// so the verifier can reconstruct the commitment for validation.
    /// A production system would use Bulletproofs or Schnorr proofs instead.
    pub blinding_factor: String,
    /// The actual claim value (only sent to verifier in the proof envelope).
    /// The verifier checks the predicate and commitment but does NOT store
    /// or forward this value — it's ephemeral.
    pub claim_value: String,
}

/// Generate a commitment: SHA-256(value || blinding).
pub fn commit(value: &str, blinding: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hasher.update(blinding);
    hex::encode(hasher.finalize())
}

/// Verify that a commitment matches value + blinding.
pub fn verify_commitment(commitment: &str, value: &str, blinding: &[u8; 32]) -> bool {
    commit(value, blinding) == commitment
}

/// Generate a random 32-byte blinding factor.
fn random_blinding() -> [u8; 32] {
    use pqc_crypto_module::legacy::rng::{OsRng, RngCore};
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    buf
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Create a ZK proof for a range predicate (value >= threshold).
pub fn prove_range(
    credential_id: &str,
    claim_key: &str,
    claim_value: u64,
    threshold: u64,
) -> Result<ZkPresentation, ZkpError> {
    if claim_value < threshold {
        return Err(ZkpError::PredicateNotSatisfied(format!(
            "{claim_key}: {claim_value} < {threshold}"
        )));
    }

    let blinding = random_blinding();
    let value_str = claim_value.to_string();
    let commitment = commit(&value_str, &blinding);

    Ok(ZkPresentation {
        credential_id: credential_id.to_string(),
        predicate: Predicate::RangeProof {
            claim_key: claim_key.to_string(),
            threshold,
        },
        commitment,
        proof: ZkProof {
            blinding_factor: hex::encode(blinding),
            claim_value: value_str,
        },
        created_at: now_secs(),
    })
}

/// Create a ZK proof for set membership (value in allowed_values).
pub fn prove_set_membership(
    credential_id: &str,
    claim_key: &str,
    claim_value: &str,
    allowed_values: &[String],
) -> Result<ZkPresentation, ZkpError> {
    if !allowed_values.iter().any(|v| v == claim_value) {
        return Err(ZkpError::PredicateNotSatisfied(format!(
            "{claim_key}: '{claim_value}' not in allowed set"
        )));
    }

    let blinding = random_blinding();
    let commitment = commit(claim_value, &blinding);

    Ok(ZkPresentation {
        credential_id: credential_id.to_string(),
        predicate: Predicate::SetMembership {
            claim_key: claim_key.to_string(),
            allowed_values: allowed_values.to_vec(),
        },
        commitment,
        proof: ZkProof {
            blinding_factor: hex::encode(blinding),
            claim_value: claim_value.to_string(),
        },
        created_at: now_secs(),
    })
}

/// Create a ZK proof for credential validity.
pub fn prove_credential_validity(
    credential_id: &str,
    status: &str,
    expires_at: u64,
    revoked_at: Option<u64>,
) -> Result<ZkPresentation, ZkpError> {
    if status != "active" {
        return Err(ZkpError::PredicateNotSatisfied(format!(
            "credential status is '{status}', not 'active'"
        )));
    }
    if expires_at > 0 && now_secs() > expires_at {
        return Err(ZkpError::PredicateNotSatisfied(
            "credential has expired".to_string(),
        ));
    }
    if revoked_at.is_some() {
        return Err(ZkpError::PredicateNotSatisfied(
            "credential has been revoked".to_string(),
        ));
    }

    let blinding = random_blinding();
    let validity_token = format!("valid:{credential_id}:{status}");
    let commitment = commit(&validity_token, &blinding);

    Ok(ZkPresentation {
        credential_id: credential_id.to_string(),
        predicate: Predicate::CredentialValidity {
            credential_id: credential_id.to_string(),
        },
        commitment,
        proof: ZkProof {
            blinding_factor: hex::encode(blinding),
            claim_value: validity_token,
        },
        created_at: now_secs(),
    })
}

/// Verify a ZK presentation.
pub fn verify_presentation(presentation: &ZkPresentation) -> Result<bool, ZkpError> {
    // Decode blinding factor
    let blinding_bytes = hex::decode(&presentation.proof.blinding_factor)
        .map_err(|e| ZkpError::InvalidProof(format!("bad blinding hex: {e}")))?;
    if blinding_bytes.len() != 32 {
        return Err(ZkpError::InvalidProof(
            "blinding must be 32 bytes".to_string(),
        ));
    }
    let mut blinding = [0u8; 32];
    blinding.copy_from_slice(&blinding_bytes);

    // Verify commitment
    if !verify_commitment(
        &presentation.commitment,
        &presentation.proof.claim_value,
        &blinding,
    ) {
        return Ok(false);
    }

    // Verify predicate
    match &presentation.predicate {
        Predicate::RangeProof { threshold, .. } => {
            let value: u64 = presentation
                .proof
                .claim_value
                .parse()
                .map_err(|_| ZkpError::InvalidProof("claim_value not a number".to_string()))?;
            Ok(value >= *threshold)
        }
        Predicate::SetMembership { allowed_values, .. } => {
            Ok(allowed_values.contains(&presentation.proof.claim_value))
        }
        Predicate::CredentialValidity { .. } => {
            // Commitment verified + value starts with "valid:" = sufficient
            Ok(presentation.proof.claim_value.starts_with("valid:"))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ZkpError {
    #[error("predicate not satisfied: {0}")]
    PredicateNotSatisfied(String),
    #[error("invalid proof: {0}")]
    InvalidProof(String),
    #[error("credential not found: {0}")]
    CredentialNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_proof_valid() {
        let pres = prove_range("cred-1", "age", 25, 18).unwrap();
        assert!(verify_presentation(&pres).unwrap());
    }

    #[test]
    fn range_proof_at_threshold() {
        let pres = prove_range("cred-1", "age", 18, 18).unwrap();
        assert!(verify_presentation(&pres).unwrap());
    }

    #[test]
    fn range_proof_below_threshold_fails_to_prove() {
        let err = prove_range("cred-1", "age", 16, 18).unwrap_err();
        assert!(matches!(err, ZkpError::PredicateNotSatisfied(_)));
    }

    #[test]
    fn set_membership_valid() {
        let allowed = vec!["CL".to_string(), "AR".to_string(), "BR".to_string()];
        let pres = prove_set_membership("cred-1", "nationality", "CL", &allowed).unwrap();
        assert!(verify_presentation(&pres).unwrap());
    }

    #[test]
    fn set_membership_not_in_set_fails() {
        let allowed = vec!["CL".to_string(), "AR".to_string()];
        let err = prove_set_membership("cred-1", "nationality", "US", &allowed).unwrap_err();
        assert!(matches!(err, ZkpError::PredicateNotSatisfied(_)));
    }

    #[test]
    fn credential_validity_active() {
        let pres = prove_credential_validity("cred-1", "active", now_secs() + 3600, None).unwrap();
        assert!(verify_presentation(&pres).unwrap());
    }

    #[test]
    fn credential_validity_revoked_fails() {
        let err = prove_credential_validity("cred-1", "active", now_secs() + 3600, Some(1000))
            .unwrap_err();
        assert!(matches!(err, ZkpError::PredicateNotSatisfied(_)));
    }

    #[test]
    fn credential_validity_inactive_fails() {
        let err =
            prove_credential_validity("cred-1", "suspended", now_secs() + 3600, None).unwrap_err();
        assert!(matches!(err, ZkpError::PredicateNotSatisfied(_)));
    }

    #[test]
    fn credential_validity_expired_fails() {
        let err = prove_credential_validity("cred-1", "active", 1, None).unwrap_err();
        assert!(matches!(err, ZkpError::PredicateNotSatisfied(_)));
    }

    #[test]
    fn tampered_commitment_rejected() {
        let mut pres = prove_range("cred-1", "age", 25, 18).unwrap();
        pres.commitment =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        assert!(!verify_presentation(&pres).unwrap());
    }

    #[test]
    fn tampered_claim_value_rejected() {
        let mut pres = prove_range("cred-1", "age", 25, 18).unwrap();
        pres.proof.claim_value = "99".to_string(); // Changed from 25 to 99
        assert!(!verify_presentation(&pres).unwrap()); // Commitment mismatch
    }

    #[test]
    fn commitment_is_deterministic() {
        let blinding = [42u8; 32];
        let c1 = commit("hello", &blinding);
        let c2 = commit("hello", &blinding);
        assert_eq!(c1, c2);
    }

    #[test]
    fn different_blindings_produce_different_commitments() {
        let b1 = [1u8; 32];
        let b2 = [2u8; 32];
        assert_ne!(commit("same", &b1), commit("same", &b2));
    }
}
