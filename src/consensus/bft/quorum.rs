//! Quorum validation logic for BFT consensus.
//!
//! Verifies that a [`QuorumCertificate`] contains enough valid, distinct votes
//! to satisfy the BFT threshold: `2f + 1` out of `n` total validators, where
//! `f = (n - 1) / 3`.

use std::collections::HashSet;

use super::types::{QcError, QuorumCertificate, VoteMessage};

/// Minimum number of validators required for Byzantine fault tolerance.
/// With n < 4, f = 0 and the network has zero Byzantine tolerance.
pub const MIN_BFT_VALIDATORS: usize = 4;

/// Signature verifier trait — abstracts over Ed25519 / ML-DSA-65 / test stubs.
pub trait SignatureVerifier: Send + Sync {
    /// Verify that `signature` is a valid signature of `payload` by `voter_id`.
    fn verify(&self, voter_id: &str, payload: &[u8], signature: &[u8]) -> bool;
}

/// BFT quorum validator.
///
/// Holds the validator set and a signature verifier.  Validates that a QC
/// contains `>= quorum_threshold()` valid, distinct votes from known validators.
pub struct QuorumValidator<V: SignatureVerifier> {
    /// Known validator identities — `HashSet` for O(1) lookup and dedup guarantee.
    validators: HashSet<String>,
    /// Signature verification implementation.
    verifier: V,
}

impl<V: SignatureVerifier> QuorumValidator<V> {
    /// Create a new validator with the given validator set.
    ///
    /// Deduplicates validator IDs. For BFT safety, `validators` should
    /// contain at least [`MIN_BFT_VALIDATORS`] unique entries; use
    /// [`ensure_bft_viable`] to enforce this.
    pub fn new(validators: Vec<String>, verifier: V) -> Self {
        let validators: HashSet<String> = validators.into_iter().collect();
        Self {
            validators,
            verifier,
        }
    }

    /// Return an error if the validator set is too small for BFT.
    ///
    /// With fewer than 4 validators, `f = 0` and the network has zero
    /// Byzantine fault tolerance. Call this at startup to reject unsafe configs.
    pub fn ensure_bft_viable(&self) -> Result<(), QcError> {
        if self.validators.len() < MIN_BFT_VALIDATORS {
            return Err(QcError::InsufficientValidators {
                min: MIN_BFT_VALIDATORS,
                have: self.validators.len(),
            });
        }
        Ok(())
    }

    /// Total number of unique validators (`n`).
    pub fn total_validators(&self) -> usize {
        self.validators.len()
    }

    /// Maximum number of faulty nodes tolerated: `f = (n - 1) / 3`.
    pub fn max_faulty(&self) -> usize {
        let n = self.validators.len();
        if n == 0 {
            return 0;
        }
        (n - 1) / 3
    }

    /// Minimum votes required for quorum: `2f + 1`.
    pub fn quorum_threshold(&self) -> usize {
        if self.validators.is_empty() {
            return 0;
        }
        2 * self.max_faulty() + 1
    }

    /// Validate a single vote: known voter + valid signature.
    pub fn validate_vote(&self, vote: &VoteMessage) -> Result<(), QcError> {
        if !self.validators.contains(&vote.voter_id) {
            return Err(QcError::UnknownVoter(vote.voter_id.clone()));
        }

        let payload = vote.payload();
        if !self
            .verifier
            .verify(&vote.voter_id, &payload, &vote.signature)
        {
            return Err(QcError::InvalidSignature(vote.voter_id.clone()));
        }

        Ok(())
    }

    /// Validate a full quorum certificate.
    ///
    /// Checks:
    /// 1. No duplicate voters
    /// 2. All voters are known validators
    /// 3. All signatures are valid
    /// 4. Number of valid, distinct votes >= `quorum_threshold()`
    pub fn validate_qc(&self, qc: &QuorumCertificate) -> Result<(), QcError> {
        let mut seen = HashSet::new();

        for vote in &qc.votes {
            // Duplicate check.
            if !seen.insert(&vote.voter_id) {
                return Err(QcError::DuplicateVoter(vote.voter_id.clone()));
            }

            self.validate_vote(vote)?;
        }

        let valid_count = seen.len();
        let threshold = self.quorum_threshold();

        if valid_count < threshold {
            return Err(QcError::InsufficientVotes {
                needed: threshold,
                have: valid_count,
            });
        }

        Ok(())
    }
}

/// Test-only signature verifier that accepts any non-empty signature.
#[cfg(test)]
#[derive(Clone)]
pub struct AcceptAllVerifier;

#[cfg(test)]
impl SignatureVerifier for AcceptAllVerifier {
    fn verify(&self, _voter_id: &str, _payload: &[u8], signature: &[u8]) -> bool {
        !signature.is_empty()
    }
}

/// Test-only verifier that rejects signatures from specific voters.
#[cfg(test)]
#[derive(Clone)]
pub struct RejectVoterVerifier {
    pub reject: HashSet<String>,
}

#[cfg(test)]
impl SignatureVerifier for RejectVoterVerifier {
    fn verify(&self, voter_id: &str, _payload: &[u8], signature: &[u8]) -> bool {
        !signature.is_empty() && !self.reject.contains(voter_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::bft::types::{BftPhase, VoteMessage};

    fn validators(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("v{i}")).collect()
    }

    fn vote(hash_id: u8, round: u64, voter: &str) -> VoteMessage {
        let mut block_hash = [0u8; 32];
        block_hash[0] = hash_id;
        VoteMessage {
            block_hash,
            round,
            phase: BftPhase::Prepare,
            voter_id: voter.to_string(),
            signature: vec![1u8; 64],
        }
    }

    fn block_hash(id: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = id;
        h
    }

    // --- ensure_bft_viable ---

    #[test]
    fn bft_viable_with_4_validators() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        assert!(qv.ensure_bft_viable().is_ok());
    }

    #[test]
    fn bft_not_viable_with_3_validators() {
        let qv = QuorumValidator::new(validators(3), AcceptAllVerifier);
        assert!(matches!(
            qv.ensure_bft_viable(),
            Err(QcError::InsufficientValidators { min: 4, have: 3 })
        ));
    }

    #[test]
    fn bft_not_viable_with_0_validators() {
        let qv = QuorumValidator::new(vec![], AcceptAllVerifier);
        assert!(matches!(
            qv.ensure_bft_viable(),
            Err(QcError::InsufficientValidators { min: 4, have: 0 })
        ));
    }

    // --- dedup on construction ---

    #[test]
    fn constructor_deduplicates_validators() {
        let duped = vec!["v0".into(), "v1".into(), "v0".into(), "v2".into()];
        let qv = QuorumValidator::new(duped, AcceptAllVerifier);
        assert_eq!(qv.total_validators(), 3);
    }

    // --- quorum_threshold tests ---

    #[test]
    fn threshold_1_validator() {
        let qv = QuorumValidator::new(validators(1), AcceptAllVerifier);
        assert_eq!(qv.max_faulty(), 0);
        assert_eq!(qv.quorum_threshold(), 1);
    }

    #[test]
    fn threshold_4_validators() {
        // n=4, f=1, threshold=3
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        assert_eq!(qv.max_faulty(), 1);
        assert_eq!(qv.quorum_threshold(), 3);
    }

    #[test]
    fn threshold_7_validators() {
        // n=7, f=2, threshold=5
        let qv = QuorumValidator::new(validators(7), AcceptAllVerifier);
        assert_eq!(qv.max_faulty(), 2);
        assert_eq!(qv.quorum_threshold(), 5);
    }

    #[test]
    fn threshold_10_validators() {
        // n=10, f=3, threshold=7
        let qv = QuorumValidator::new(validators(10), AcceptAllVerifier);
        assert_eq!(qv.max_faulty(), 3);
        assert_eq!(qv.quorum_threshold(), 7);
    }

    #[test]
    fn threshold_empty_validators() {
        let qv = QuorumValidator::new(vec![], AcceptAllVerifier);
        assert_eq!(qv.max_faulty(), 0);
        assert_eq!(qv.quorum_threshold(), 0);
    }

    // --- validate_vote tests ---

    #[test]
    fn validate_vote_known_voter_accepts() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let v = vote(1, 0, "v0");
        assert!(qv.validate_vote(&v).is_ok());
    }

    #[test]
    fn validate_vote_unknown_voter_rejects() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let v = vote(1, 0, "intruder");
        assert!(matches!(
            qv.validate_vote(&v),
            Err(QcError::UnknownVoter(_))
        ));
    }

    #[test]
    fn validate_vote_invalid_signature_rejects() {
        let mut reject_set = HashSet::new();
        reject_set.insert("v0".to_string());
        let verifier = RejectVoterVerifier { reject: reject_set };
        let qv = QuorumValidator::new(validators(4), verifier);
        let v = vote(1, 0, "v0");
        assert!(matches!(
            qv.validate_vote(&v),
            Err(QcError::InvalidSignature(_))
        ));
    }

    // --- validate_qc tests ---

    #[test]
    fn validate_qc_sufficient_votes_accepts() {
        // n=4, threshold=3
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1"), vote(1, 0, "v2")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_ok());
    }

    #[test]
    fn validate_qc_insufficient_votes_rejects() {
        // n=4, threshold=3, only 2 votes
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(matches!(
            qv.validate_qc(&qc),
            Err(QcError::InsufficientVotes { needed: 3, have: 2 })
        ));
    }

    #[test]
    fn validate_qc_duplicate_voter_rejects() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![
            vote(1, 0, "v0"),
            vote(1, 0, "v0"), // duplicate
            vote(1, 0, "v1"),
        ];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(matches!(
            qv.validate_qc(&qc),
            Err(QcError::DuplicateVoter(_))
        ));
    }

    #[test]
    fn validate_qc_unknown_voter_in_set_rejects() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1"), vote(1, 0, "intruder")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(matches!(qv.validate_qc(&qc), Err(QcError::UnknownVoter(_))));
    }

    #[test]
    fn validate_qc_invalid_signature_in_set_rejects() {
        let mut reject_set = HashSet::new();
        reject_set.insert("v2".to_string());
        let verifier = RejectVoterVerifier { reject: reject_set };
        let qv = QuorumValidator::new(validators(4), verifier);
        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1"), vote(1, 0, "v2")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(matches!(
            qv.validate_qc(&qc),
            Err(QcError::InvalidSignature(_))
        ));
    }

    #[test]
    fn validate_qc_all_validators_vote_accepts() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![
            vote(1, 0, "v0"),
            vote(1, 0, "v1"),
            vote(1, 0, "v2"),
            vote(1, 0, "v3"),
        ];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_ok());
    }

    #[test]
    fn validate_qc_one_byzantine_still_passes() {
        // n=4, f=1. Skip v3 (byzantine) — 3 honest votes suffice.
        let mut reject_set = HashSet::new();
        reject_set.insert("v3".to_string());
        let verifier = RejectVoterVerifier { reject: reject_set };
        let qv = QuorumValidator::new(validators(4), verifier);

        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1"), vote(1, 0, "v2")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_ok());
    }

    // --- BFT tolerance boundary tests ---

    #[test]
    fn n4_tolerates_1_missing_voter() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1"), vote(1, 0, "v2")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_ok());
    }

    #[test]
    fn n4_cannot_tolerate_2_missing_voters() {
        let qv = QuorumValidator::new(validators(4), AcceptAllVerifier);
        let votes = vec![vote(1, 0, "v0"), vote(1, 0, "v1")];
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_err());
    }

    #[test]
    fn n7_tolerates_2_missing_voters() {
        let qv = QuorumValidator::new(validators(7), AcceptAllVerifier);
        let votes: Vec<VoteMessage> = (0..5).map(|i| vote(1, 0, &format!("v{i}"))).collect();
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_ok());
    }

    #[test]
    fn n7_cannot_tolerate_3_missing_voters() {
        let qv = QuorumValidator::new(validators(7), AcceptAllVerifier);
        let votes: Vec<VoteMessage> = (0..4).map(|i| vote(1, 0, &format!("v{i}"))).collect();
        let qc = QuorumCertificate::new(BftPhase::Prepare, block_hash(1), 0, votes).unwrap();
        assert!(qv.validate_qc(&qc).is_err());
    }
}
