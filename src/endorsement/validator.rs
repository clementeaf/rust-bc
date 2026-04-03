//! Endorsement verification logic

use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use thiserror::Error;

use super::policy::EndorsementPolicy;
use super::registry::OrgRegistry;
use super::types::Endorsement;

/// Errors produced during endorsement verification
#[derive(Debug, Error)]
pub enum EndorsementError {
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("organization not found: {0}")]
    OrgNotFound(String),
    #[error("policy not satisfied: got {got} matching orgs, need more")]
    PolicyNotSatisfied { got: usize },
}

/// Verify a single endorsement against a known public key.
///
/// `public_key` must be a valid 32-byte Ed25519 public key.
pub fn verify_endorsement(e: &Endorsement, public_key: &[u8; 32]) -> Result<(), EndorsementError> {
    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|err| EndorsementError::InvalidPublicKey(err.to_string()))?;

    let signature = Signature::from_slice(&e.signature)
        .map_err(|err| EndorsementError::InvalidSignature(err.to_string()))?;

    verifying_key
        .verify(&e.payload_hash, &signature)
        .map_err(|_| EndorsementError::VerificationFailed)
}

/// Validate a set of endorsements against a policy and an org registry.
///
/// Steps:
/// 1. For each endorsement, look up the org from the registry.
/// 2. Try to verify the endorsement against any of the org's root public keys.
/// 3. Collect the unique set of orgs with a valid endorsement.
/// 4. Evaluate the policy against those orgs.
pub fn validate_endorsements(
    endorsements: &[Endorsement],
    policy: &EndorsementPolicy,
    registry: &dyn OrgRegistry,
) -> Result<(), EndorsementError> {
    let mut valid_orgs: Vec<&str> = Vec::new();

    for e in endorsements {
        let org = registry
            .get_org(&e.org_id)
            .map_err(|_| EndorsementError::OrgNotFound(e.org_id.clone()))?;

        let sig_ok = org
            .root_public_keys
            .iter()
            .any(|pk| verify_endorsement(e, pk).is_ok());

        if sig_ok && !valid_orgs.contains(&e.org_id.as_str()) {
            valid_orgs.push(&e.org_id);
        }
    }

    if policy.evaluate(&valid_orgs) {
        Ok(())
    } else {
        Err(EndorsementError::PolicyNotSatisfied { got: valid_orgs.len() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::org::Organization;
    use crate::endorsement::registry::MemoryOrgRegistry;
    use ed25519_dalek::{SigningKey, Signer};
    use rand::rngs::OsRng;

    fn make_keypair() -> (SigningKey, [u8; 32]) {
        let sk = SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key().to_bytes();
        (sk, pk)
    }

    fn make_endorsement(sk: &SigningKey, payload_hash: [u8; 32], org_id: &str) -> Endorsement {
        let sig = sk.sign(&payload_hash).to_bytes();
        Endorsement {
            signer_did: format!("did:bc:{org_id}:signer"),
            org_id: org_id.to_string(),
            signature: sig,
            payload_hash,
            timestamp: 0,
        }
    }

    #[test]
    fn valid_signature_passes() {
        let (sk, pk) = make_keypair();
        let payload = [1u8; 32];
        let e = make_endorsement(&sk, payload, "org1");
        assert!(verify_endorsement(&e, &pk).is_ok());
    }

    #[test]
    fn invalid_signature_fails() {
        let (_, pk) = make_keypair();
        let payload = [1u8; 32];
        // signature bytes all zeros — invalid
        let e = Endorsement {
            signer_did: "did:bc:x".to_string(),
            org_id: "org1".to_string(),
            signature: [0u8; 64],
            payload_hash: payload,
            timestamp: 0,
        };
        assert!(verify_endorsement(&e, &pk).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let (sk, _) = make_keypair();
        let (_, other_pk) = make_keypair();
        let payload = [2u8; 32];
        let e = make_endorsement(&sk, payload, "org1");
        assert!(verify_endorsement(&e, &other_pk).is_err());
    }

    fn setup_registry_with_orgs(orgs: &[(&str, [u8; 32])]) -> MemoryOrgRegistry {
        let reg = MemoryOrgRegistry::new();
        for (org_id, pk) in orgs {
            let org = Organization::new(
                *org_id,
                &format!("{org_id}MSP"),
                vec![format!("did:bc:{org_id}:admin")],
                vec![],
                vec![*pk],
            )
            .unwrap();
            reg.register_org(&org).unwrap();
        }
        reg
    }

    #[test]
    fn validate_endorsements_pass() {
        let (sk1, pk1) = make_keypair();
        let (sk2, pk2) = make_keypair();
        let (sk3, pk3) = make_keypair();
        let payload = [5u8; 32];

        let reg = setup_registry_with_orgs(&[("org1", pk1), ("org2", pk2), ("org3", pk3)]);

        let endorsements = vec![
            make_endorsement(&sk1, payload, "org1"),
            make_endorsement(&sk2, payload, "org2"),
            make_endorsement(&sk3, payload, "org3"),
        ];

        let policy = EndorsementPolicy::NOutOf {
            n: 2,
            orgs: vec!["org1".into(), "org2".into(), "org3".into()],
        };

        assert!(validate_endorsements(&endorsements, &policy, &reg).is_ok());
    }

    #[test]
    fn validate_endorsements_fail_too_few() {
        let (sk1, pk1) = make_keypair();
        let (_, pk2) = make_keypair();
        let payload = [6u8; 32];

        let reg = setup_registry_with_orgs(&[("org1", pk1), ("org2", pk2)]);

        let endorsements = vec![make_endorsement(&sk1, payload, "org1")];

        let policy = EndorsementPolicy::NOutOf {
            n: 2,
            orgs: vec!["org1".into(), "org2".into()],
        };

        assert!(validate_endorsements(&endorsements, &policy, &reg).is_err());
    }
}
