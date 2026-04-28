//! Validator slashing / penalty economics and expiration rules.
//!
//! Defines the lifecycle of validator punishment:
//! - Penalty creation (from equivocation proof or protocol violation)
//! - Active status (proposals rejected)
//! - Deterministic expiration (at `start_height + duration`)
//! - Permanent penalties (never expire)
//! - Reputation tracking
//! - Persistence (serialize/deserialize for restart survival)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Why the validator was penalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenaltyReason {
    Equivocation,
    InvalidPqcSignatureFlood,
    ProtocolViolation,
}

/// Current penalty status, computed from `start_height`, `until_height`, and current height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenaltyStatus {
    Active,
    Expired,
    Permanent,
}

/// A single penalty record for a validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyRecord {
    pub validator_id: String,
    pub reason: PenaltyReason,
    /// Hash of the equivocation proof (for linking).
    pub proof_hash: Option<[u8; 32]>,
    pub start_height: u64,
    /// `None` = permanent penalty.
    pub until_height: Option<u64>,
    pub reputation_delta: i64,
    pub status: PenaltyStatus,
}

impl PenaltyRecord {
    /// Recompute status based on the current chain height.
    pub fn status_at(&self, current_height: u64) -> PenaltyStatus {
        match self.until_height {
            None => PenaltyStatus::Permanent,
            Some(until) => {
                if current_height >= until {
                    PenaltyStatus::Expired
                } else {
                    PenaltyStatus::Active
                }
            }
        }
    }
}

/// Configurable penalty policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenaltyPolicy {
    pub equivocation_penalty_duration_blocks: u64,
    pub equivocation_is_permanent: bool,
    /// Basis points (1/100th of percent). 500 = 5%.
    pub slash_percent_bps: u16,
    pub min_slash_amount: u128,
    pub reputation_penalty: i64,
    /// If true, repeated equivocation escalates to permanent.
    pub escalate_on_repeat: bool,
}

impl Default for PenaltyPolicy {
    fn default() -> Self {
        Self {
            equivocation_penalty_duration_blocks: 10_000,
            equivocation_is_permanent: false,
            slash_percent_bps: 500,
            min_slash_amount: 1,
            reputation_penalty: -100,
            escalate_on_repeat: true,
        }
    }
}

/// Manages validator penalties with deterministic expiration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PenaltyManager {
    /// All penalty records per validator (append-only for history).
    records: HashMap<String, Vec<PenaltyRecord>>,
    /// Reputation scores per validator (starts at 0, decremented on penalty).
    reputation: HashMap<String, i64>,
    /// Set of proof hashes already processed (prevents double-slashing).
    processed_proofs: Vec<[u8; 32]>,
}

impl PenaltyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a penalty for equivocation.
    ///
    /// Returns the created `PenaltyRecord`, or `None` if the proof was already processed.
    pub fn penalize_equivocation(
        &mut self,
        validator_id: &str,
        proof_hash: [u8; 32],
        detection_height: u64,
        policy: &PenaltyPolicy,
    ) -> Option<PenaltyRecord> {
        // Prevent double-slashing for same proof
        if self.processed_proofs.contains(&proof_hash) {
            return None;
        }
        self.processed_proofs.push(proof_hash);

        let existing_records = self.records.entry(validator_id.to_string()).or_default();

        // Check for escalation: if already penalized and policy escalates
        let has_active = existing_records.iter().any(|r| {
            r.status_at(detection_height) == PenaltyStatus::Active
                || r.status_at(detection_height) == PenaltyStatus::Permanent
        });

        let (until_height, status) =
            if policy.equivocation_is_permanent || (policy.escalate_on_repeat && has_active) {
                (None, PenaltyStatus::Permanent)
            } else {
                let until = detection_height + policy.equivocation_penalty_duration_blocks;
                (Some(until), PenaltyStatus::Active)
            };

        let record = PenaltyRecord {
            validator_id: validator_id.to_string(),
            reason: PenaltyReason::Equivocation,
            proof_hash: Some(proof_hash),
            start_height: detection_height,
            until_height,
            reputation_delta: policy.reputation_penalty,
            status,
        };

        existing_records.push(record.clone());

        // Apply reputation delta (once per proof)
        *self.reputation.entry(validator_id.to_string()).or_default() += policy.reputation_penalty;

        Some(record)
    }

    /// Check if a validator has an active penalty at the given height.
    pub fn is_active_penalty(&self, validator_id: &str, current_height: u64) -> bool {
        self.records
            .get(validator_id)
            .map(|records| {
                records.iter().any(|r| {
                    let s = r.status_at(current_height);
                    s == PenaltyStatus::Active || s == PenaltyStatus::Permanent
                })
            })
            .unwrap_or(false)
    }

    /// Get all penalty records for a validator.
    pub fn get_records(&self, validator_id: &str) -> &[PenaltyRecord] {
        self.records
            .get(validator_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the reputation score for a validator (0 = no history).
    pub fn reputation(&self, validator_id: &str) -> i64 {
        self.reputation.get(validator_id).copied().unwrap_or(0)
    }

    /// Check if a proof hash has already been processed.
    pub fn is_proof_processed(&self, proof_hash: &[u8; 32]) -> bool {
        self.processed_proofs.contains(proof_hash)
    }

    /// Total penalty records across all validators.
    pub fn total_records(&self) -> usize {
        self.records.values().map(|v| v.len()).sum()
    }

    // ── Persistence ─────────────────────────────────────────────────

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof_hash(n: u8) -> [u8; 32] {
        let mut h = [0u8; 32];
        h[0] = n;
        h
    }

    #[test]
    fn default_policy_values() {
        let p = PenaltyPolicy::default();
        assert_eq!(p.equivocation_penalty_duration_blocks, 10_000);
        assert!(!p.equivocation_is_permanent);
        assert_eq!(p.slash_percent_bps, 500);
        assert_eq!(p.reputation_penalty, -100);
    }

    #[test]
    fn penalty_record_status_at() {
        let r = PenaltyRecord {
            validator_id: "v1".to_string(),
            reason: PenaltyReason::Equivocation,
            proof_hash: None,
            start_height: 100,
            until_height: Some(200),
            reputation_delta: -100,
            status: PenaltyStatus::Active,
        };
        assert_eq!(r.status_at(99), PenaltyStatus::Active);
        assert_eq!(r.status_at(100), PenaltyStatus::Active);
        assert_eq!(r.status_at(199), PenaltyStatus::Active);
        assert_eq!(r.status_at(200), PenaltyStatus::Expired);
        assert_eq!(r.status_at(300), PenaltyStatus::Expired);
    }

    #[test]
    fn permanent_penalty_never_expires() {
        let r = PenaltyRecord {
            validator_id: "v1".to_string(),
            reason: PenaltyReason::Equivocation,
            proof_hash: None,
            start_height: 0,
            until_height: None,
            reputation_delta: -100,
            status: PenaltyStatus::Permanent,
        };
        assert_eq!(r.status_at(0), PenaltyStatus::Permanent);
        assert_eq!(r.status_at(u64::MAX), PenaltyStatus::Permanent);
    }

    #[test]
    fn serde_roundtrip() {
        let mut mgr = PenaltyManager::new();
        let policy = PenaltyPolicy::default();
        mgr.penalize_equivocation("v1", proof_hash(1), 100, &policy);

        let bytes = mgr.to_bytes();
        let restored = PenaltyManager::from_bytes(&bytes).unwrap();
        assert!(restored.is_active_penalty("v1", 100));
        assert_eq!(restored.reputation("v1"), -100);
    }
}
