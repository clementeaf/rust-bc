//! Endorsement verification logic

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use thiserror::Error;

use super::key_policy::KeyEndorsementStore;
use super::policy::EndorsementPolicy;
use super::registry::OrgRegistry;
use super::types::Endorsement;
use crate::msp::CrlStore;

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
    #[error("signer key revoked (serial {serial}) in MSP {msp_id}")]
    SignerRevoked { serial: String, msp_id: String },
    #[error("policy not satisfied: got {got} matching orgs, need more")]
    PolicyNotSatisfied { got: usize },
}

/// Verify a single endorsement against a known public key.
///
/// Currently supports Ed25519 (32-byte public key, 64-byte signature).
/// Post-quantum signature verification is handled by `SigningProvider::verify()`.
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
/// 3. If `crl_store` is provided, reject endorsements whose signing key serial
///    appears in the MSP's CRL.
/// 4. Collect the unique set of orgs with a valid, non-revoked endorsement.
/// 5. Evaluate the policy against those orgs.
pub fn validate_endorsements(
    endorsements: &[Endorsement],
    policy: &EndorsementPolicy,
    registry: &dyn OrgRegistry,
    crl_store: Option<&dyn CrlStore>,
) -> Result<(), EndorsementError> {
    let mut valid_orgs: Vec<&str> = Vec::new();

    for e in endorsements {
        let org = registry
            .get_org(&e.org_id)
            .map_err(|_| EndorsementError::OrgNotFound(e.org_id.clone()))?;

        // Find the root key that verifies this endorsement.
        let verified_pk = org
            .root_public_keys
            .iter()
            .find(|pk| verify_endorsement(e, pk).is_ok());

        let Some(pk) = verified_pk else { continue };

        // CRL check: reject if the signing key has been revoked.
        if let Some(store) = crl_store {
            let serial = hex::encode(pk);
            let revoked = store.read_crl(&org.msp_id).unwrap_or_default();
            if revoked.contains(&serial) {
                return Err(EndorsementError::SignerRevoked {
                    serial,
                    msp_id: org.msp_id.clone(),
                });
            }
        }

        if !valid_orgs.contains(&e.org_id.as_str()) {
            valid_orgs.push(&e.org_id);
        }
    }

    if policy.evaluate(&valid_orgs) {
        Ok(())
    } else {
        Err(EndorsementError::PolicyNotSatisfied {
            got: valid_orgs.len(),
        })
    }
}

#[allow(dead_code)]
/// Validate endorsements for a set of rwset write keys, applying key-level
/// policy overrides where they exist.
///
/// For each write key the effective policy is determined as:
/// - key-level policy (from `key_policy_store`) if one exists for the key
/// - `chaincode_policy` otherwise
///
/// The endorsements must satisfy **every** distinct effective policy.
/// Priority: key-level > chaincode-level.
pub fn validate_endorsements_for_writes(
    endorsements: &[Endorsement],
    chaincode_policy: &EndorsementPolicy,
    registry: &dyn OrgRegistry,
    crl_store: Option<&dyn CrlStore>,
    write_keys: &[&str],
    key_policy_store: Option<&dyn KeyEndorsementStore>,
) -> Result<(), EndorsementError> {
    // Step 1: resolve the verified set of org IDs (same as validate_endorsements).
    let mut valid_orgs: Vec<&str> = Vec::new();

    for e in endorsements {
        let org = registry
            .get_org(&e.org_id)
            .map_err(|_| EndorsementError::OrgNotFound(e.org_id.clone()))?;

        let verified_pk = org
            .root_public_keys
            .iter()
            .find(|pk| verify_endorsement(e, pk).is_ok());

        let Some(pk) = verified_pk else { continue };

        if let Some(store) = crl_store {
            let serial = hex::encode(pk);
            let revoked = store.read_crl(&org.msp_id).unwrap_or_default();
            if revoked.contains(&serial) {
                return Err(EndorsementError::SignerRevoked {
                    serial,
                    msp_id: org.msp_id.clone(),
                });
            }
        }

        if !valid_orgs.contains(&e.org_id.as_str()) {
            valid_orgs.push(&e.org_id);
        }
    }

    // Step 2: determine effective policies and validate against each.
    //
    // We collect unique effective policies so that multiple keys sharing the
    // same policy are only evaluated once.
    let mut evaluated_chaincode_policy = false;
    let valid_org_refs: Vec<&str> = valid_orgs.clone();

    for key in write_keys {
        let effective_policy = key_policy_store.and_then(|s| s.get_key_policy(key).ok().flatten());

        match effective_policy {
            Some(ref kp) => {
                if !kp.evaluate(&valid_org_refs) {
                    return Err(EndorsementError::PolicyNotSatisfied {
                        got: valid_orgs.len(),
                    });
                }
            }
            None => {
                if !evaluated_chaincode_policy {
                    evaluated_chaincode_policy = true;
                    if !chaincode_policy.evaluate(&valid_org_refs) {
                        return Err(EndorsementError::PolicyNotSatisfied {
                            got: valid_orgs.len(),
                        });
                    }
                }
            }
        }
    }

    // If there were no write keys, fall back to the chaincode-level policy.
    if write_keys.is_empty() && !chaincode_policy.evaluate(&valid_org_refs) {
        return Err(EndorsementError::PolicyNotSatisfied {
            got: valid_orgs.len(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endorsement::org::Organization;
    use crate::endorsement::registry::MemoryOrgRegistry;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn make_keypair() -> (SigningKey, [u8; 32]) {
        let sk = SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key().to_bytes();
        (sk, pk)
    }

    fn make_endorsement(sk: &SigningKey, payload_hash: [u8; 32], org_id: &str) -> Endorsement {
        let sig = sk.sign(&payload_hash).to_bytes().to_vec();
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
            signature: vec![0u8; 64],
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
                format!("{org_id}MSP"),
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

        assert!(validate_endorsements(&endorsements, &policy, &reg, None).is_ok());
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

        assert!(validate_endorsements(&endorsements, &policy, &reg, None).is_err());
    }

    #[test]
    fn validate_endorsements_revoked_signer_rejected() {
        use crate::msp::CrlStore;
        use crate::storage::errors::StorageResult;
        use std::collections::HashMap;
        use std::sync::Mutex;

        // Minimal in-memory CRL store for the test.
        struct MemCrl(Mutex<HashMap<String, Vec<String>>>);
        impl CrlStore for MemCrl {
            fn write_crl(&self, msp_id: &str, serials: &[String]) -> StorageResult<()> {
                self.0
                    .lock()
                    .unwrap()
                    .insert(msp_id.to_string(), serials.to_vec());
                Ok(())
            }
            fn read_crl(&self, msp_id: &str) -> StorageResult<Vec<String>> {
                Ok(self
                    .0
                    .lock()
                    .unwrap()
                    .get(msp_id)
                    .cloned()
                    .unwrap_or_default())
            }
        }

        let (sk1, pk1) = make_keypair();
        let payload = [7u8; 32];
        let reg = setup_registry_with_orgs(&[("org1", pk1)]);

        let crl = MemCrl(Mutex::new(HashMap::new()));
        // Revoke the signing key.
        crl.write_crl("org1MSP", &[hex::encode(pk1)]).unwrap();

        let endorsements = vec![make_endorsement(&sk1, payload, "org1")];
        let policy = EndorsementPolicy::AnyOf(vec!["org1".into()]);

        let result = validate_endorsements(&endorsements, &policy, &reg, Some(&crl));
        assert!(matches!(
            result,
            Err(EndorsementError::SignerRevoked { .. })
        ));
    }

    // ── validate_endorsements_for_writes ─────────────────────────────────────

    #[test]
    fn key_level_policy_overrides_chaincode_policy() {
        use crate::endorsement::key_policy::MemoryKeyEndorsementStore;

        let (sk1, pk1) = make_keypair();
        let (sk2, pk2) = make_keypair();
        let payload = [10u8; 32];

        let reg = setup_registry_with_orgs(&[("org1", pk1), ("org2", pk2)]);
        // Chaincode policy requires only org1.
        let chaincode_policy = EndorsementPolicy::AnyOf(vec!["org1".into()]);
        // Key-level policy for "asset:x" requires BOTH org1 AND org2.
        let kep = MemoryKeyEndorsementStore::new();
        kep.set_key_policy(
            "asset:x",
            &EndorsementPolicy::AllOf(vec!["org1".into(), "org2".into()]),
        )
        .unwrap();

        // Endorsements from org1 only — satisfies chaincode but NOT key-level.
        let endorsements = vec![make_endorsement(&sk1, payload, "org1")];
        let result = validate_endorsements_for_writes(
            &endorsements,
            &chaincode_policy,
            &reg,
            None,
            &["asset:x"],
            Some(&kep),
        );
        assert!(
            result.is_err(),
            "org1-only endorsement must fail key-level AllOf(org1,org2) policy"
        );

        // Endorsements from both org1 and org2 — satisfies key-level policy.
        let endorsements2 = vec![
            make_endorsement(&sk1, payload, "org1"),
            make_endorsement(&sk2, payload, "org2"),
        ];
        assert!(validate_endorsements_for_writes(
            &endorsements2,
            &chaincode_policy,
            &reg,
            None,
            &["asset:x"],
            Some(&kep),
        )
        .is_ok());
    }

    #[test]
    fn key_without_override_uses_chaincode_policy() {
        use crate::endorsement::key_policy::MemoryKeyEndorsementStore;

        let (sk1, pk1) = make_keypair();
        let payload = [11u8; 32];

        let reg = setup_registry_with_orgs(&[("org1", pk1)]);
        let chaincode_policy = EndorsementPolicy::AnyOf(vec!["org1".into()]);
        // No key-level policy registered.
        let kep = MemoryKeyEndorsementStore::new();

        let endorsements = vec![make_endorsement(&sk1, payload, "org1")];
        assert!(validate_endorsements_for_writes(
            &endorsements,
            &chaincode_policy,
            &reg,
            None,
            &["some:key"],
            Some(&kep),
        )
        .is_ok());
    }

    #[test]
    fn no_write_keys_falls_back_to_chaincode_policy() {
        let (sk1, pk1) = make_keypair();
        let payload = [12u8; 32];

        let reg = setup_registry_with_orgs(&[("org1", pk1)]);
        let policy = EndorsementPolicy::AnyOf(vec!["org1".into()]);
        let endorsements = vec![make_endorsement(&sk1, payload, "org1")];

        // Empty write_keys → chaincode policy governs.
        assert!(
            validate_endorsements_for_writes(&endorsements, &policy, &reg, None, &[], None,)
                .is_ok()
        );
    }

    #[test]
    fn no_write_keys_fails_when_chaincode_policy_not_met() {
        let (_, pk1) = make_keypair();
        let _payload = [13u8; 32];

        let reg = setup_registry_with_orgs(&[("org1", pk1)]);
        let policy = EndorsementPolicy::AllOf(vec!["org1".into()]);
        // No endorsements — empty valid_orgs.
        assert!(validate_endorsements_for_writes(&[], &policy, &reg, None, &[], None,).is_err());
    }
}
