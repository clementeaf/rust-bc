//! Slashing / penalty lifecycle tests.
//!
//! Validates the full lifecycle of validator punishment:
//! creation, active rejection, deterministic expiration,
//! permanent penalties, persistence, and anti-double-slash.

use std::path::Path;

use rust_bc::consensus::slashing::{PenaltyManager, PenaltyPolicy, PenaltyReason, PenaltyStatus};

fn proof_hash(n: u8) -> [u8; 32] {
    let mut h = [0u8; 32];
    h[0] = n;
    h
}

const PENALTY_FILE: &str = "penalty_state.json";

fn save_penalty(dir: &Path, mgr: &PenaltyManager) {
    std::fs::write(dir.join(PENALTY_FILE), mgr.to_bytes()).expect("persist penalty");
}

fn load_penalty(dir: &Path) -> Option<PenaltyManager> {
    let data = std::fs::read(dir.join(PENALTY_FILE)).ok()?;
    PenaltyManager::from_bytes(&data)
}

// ═══════════════════════════════════════════════════════════════════
// 1. Equivocation creates active penalty record
// ═══════════════════════════════════════════════════════════════════

#[test]
fn equivocation_creates_active_penalty_record() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy::default();
    let detection_height = 500;

    let record = mgr
        .penalize_equivocation("evil-val", proof_hash(1), detection_height, &policy)
        .expect("penalty record must be created");

    assert_eq!(record.reason, PenaltyReason::Equivocation);
    assert_eq!(record.status, PenaltyStatus::Active);
    assert_eq!(record.start_height, detection_height);
    assert_eq!(
        record.until_height,
        Some(detection_height + policy.equivocation_penalty_duration_blocks)
    );
    assert_eq!(record.reputation_delta, policy.reputation_penalty);
    assert_eq!(record.proof_hash, Some(proof_hash(1)));
}

// ═══════════════════════════════════════════════════════════════════
// 2. Active penalty rejects validator proposals
// ═══════════════════════════════════════════════════════════════════

#[test]
fn active_penalty_rejects_validator_proposals() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy::default();
    mgr.penalize_equivocation("bad-val", proof_hash(1), 100, &policy);

    // Penalty active from 100 to 100+10000=10100
    assert!(
        mgr.is_active_penalty("bad-val", 100),
        "penalty must be active at start"
    );
    assert!(
        mgr.is_active_penalty("bad-val", 5000),
        "penalty must be active mid-period"
    );
    assert!(
        mgr.is_active_penalty("bad-val", 10099),
        "penalty must be active just before expiry"
    );

    // Honest validator is never penalized
    assert!(!mgr.is_active_penalty("good-val", 5000));
}

// ═══════════════════════════════════════════════════════════════════
// 3. Penalty expires at deterministic height
// ═══════════════════════════════════════════════════════════════════

#[test]
fn penalty_expires_at_deterministic_height() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy::default();
    let start = 1000;
    let duration = policy.equivocation_penalty_duration_blocks;
    let expiry = start + duration;

    mgr.penalize_equivocation("det-val", proof_hash(1), start, &policy);

    // One block before expiry: still active
    assert!(
        mgr.is_active_penalty("det-val", expiry - 1),
        "penalty must be active at height {}-1",
        expiry
    );

    // At expiry: expired
    assert!(
        !mgr.is_active_penalty("det-val", expiry),
        "penalty must expire at height {}",
        expiry
    );

    // Well after expiry
    assert!(!mgr.is_active_penalty("det-val", expiry + 1000));

    // Record still exists (historical)
    let records = mgr.get_records("det-val");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].status_at(expiry), PenaltyStatus::Expired);
}

// ═══════════════════════════════════════════════════════════════════
// 4. Permanent penalty never expires
// ═══════════════════════════════════════════════════════════════════

#[test]
fn permanent_penalty_never_expires() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy {
        equivocation_is_permanent: true,
        ..PenaltyPolicy::default()
    };

    let record = mgr
        .penalize_equivocation("perm-val", proof_hash(1), 0, &policy)
        .unwrap();

    assert_eq!(record.status, PenaltyStatus::Permanent);
    assert_eq!(record.until_height, None);

    // Active at any height
    assert!(mgr.is_active_penalty("perm-val", 0));
    assert!(mgr.is_active_penalty("perm-val", 1_000_000));
    assert!(mgr.is_active_penalty("perm-val", u64::MAX - 1));
}

// ═══════════════════════════════════════════════════════════════════
// 5. Penalty lifecycle survives restart
// ═══════════════════════════════════════════════════════════════════

#[test]
fn penalty_lifecycle_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let policy = PenaltyPolicy::default();
    let start = 500;
    let expiry = start + policy.equivocation_penalty_duration_blocks;

    // Phase 1: Create penalty and persist
    {
        let mut mgr = PenaltyManager::new();
        mgr.penalize_equivocation("restart-val", proof_hash(1), start, &policy);
        assert!(mgr.is_active_penalty("restart-val", start));
        save_penalty(dir.path(), &mgr);
    }

    // Phase 2: Restart — load and verify active
    {
        let restored = load_penalty(dir.path()).expect("load penalty state");
        assert!(
            restored.is_active_penalty("restart-val", start + 100),
            "penalty must be active after restart"
        );
        assert_eq!(restored.reputation("restart-val"), -100);
        assert_eq!(restored.get_records("restart-val").len(), 1);
    }

    // Phase 3: Verify expiration works after restart
    {
        let restored = load_penalty(dir.path()).expect("load penalty state");
        assert!(
            !restored.is_active_penalty("restart-val", expiry),
            "penalty must expire at correct height after restart"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. Expired penalty preserves historical proof
// ═══════════════════════════════════════════════════════════════════

#[test]
fn expired_penalty_does_not_delete_historical_proof() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy::default();
    let start = 100;
    let expiry = start + policy.equivocation_penalty_duration_blocks;

    mgr.penalize_equivocation("hist-val", proof_hash(1), start, &policy);

    // Advance past expiry
    assert!(!mgr.is_active_penalty("hist-val", expiry + 1000));

    // Proof and record must still exist
    let records = mgr.get_records("hist-val");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].reason, PenaltyReason::Equivocation);
    assert_eq!(records[0].proof_hash, Some(proof_hash(1)));
    assert!(mgr.is_proof_processed(&proof_hash(1)));
}

// ═══════════════════════════════════════════════════════════════════
// 7. Repeated equivocation escalates to permanent
// ═══════════════════════════════════════════════════════════════════

#[test]
fn repeated_equivocation_extends_or_escalates_penalty() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy {
        escalate_on_repeat: true,
        ..PenaltyPolicy::default()
    };

    // First equivocation: temporary
    let r1 = mgr
        .penalize_equivocation("repeat-val", proof_hash(1), 100, &policy)
        .unwrap();
    assert_eq!(r1.status, PenaltyStatus::Active);
    assert!(r1.until_height.is_some());

    // Second equivocation while first is active: escalates to permanent
    let r2 = mgr
        .penalize_equivocation("repeat-val", proof_hash(2), 200, &policy)
        .unwrap();
    assert_eq!(
        r2.status,
        PenaltyStatus::Permanent,
        "repeated equivocation must escalate to permanent"
    );
    assert_eq!(r2.until_height, None);

    // Reputation further decreased
    assert_eq!(mgr.reputation("repeat-val"), -200);

    // Two records exist
    assert_eq!(mgr.get_records("repeat-val").len(), 2);

    // Permanent means active at any future height
    assert!(mgr.is_active_penalty("repeat-val", u64::MAX - 1));
}

// ═══════════════════════════════════════════════════════════════════
// 8. Duplicate proof does not double-slash
// ═══════════════════════════════════════════════════════════════════

#[test]
fn slashing_reputation_delta_is_applied_once() {
    let mut mgr = PenaltyManager::new();
    let policy = PenaltyPolicy::default();

    // First application
    let r1 = mgr.penalize_equivocation("dup-val", proof_hash(1), 100, &policy);
    assert!(r1.is_some());

    // Same proof again
    let r2 = mgr.penalize_equivocation("dup-val", proof_hash(1), 100, &policy);
    assert!(
        r2.is_none(),
        "same proof must be rejected (no double-slash)"
    );

    // Reputation only applied once
    assert_eq!(mgr.reputation("dup-val"), -100);
    assert_eq!(mgr.get_records("dup-val").len(), 1);
}

// ═══════════════════════════════════════════════════════════════════
// 9. Honest validator never penalized
// ═══════════════════════════════════════════════════════════════════

#[test]
fn honest_validator_never_penalized_by_expiration_logic() {
    let dir = tempfile::tempdir().unwrap();

    // Phase 1: Normal operation
    {
        let mgr = PenaltyManager::new();
        assert!(!mgr.is_active_penalty("honest-val", 0));
        assert!(!mgr.is_active_penalty("honest-val", 1_000_000));
        assert_eq!(mgr.reputation("honest-val"), 0);
        assert_eq!(mgr.get_records("honest-val").len(), 0);
        save_penalty(dir.path(), &mgr);
    }

    // Phase 2: Restart
    {
        let restored = load_penalty(dir.path()).unwrap();
        assert!(!restored.is_active_penalty("honest-val", 0));
        assert!(!restored.is_active_penalty("honest-val", 1_000_000));
        assert_eq!(restored.reputation("honest-val"), 0);
        assert_eq!(restored.get_records("honest-val").len(), 0);
        assert_eq!(restored.total_records(), 0);
    }
}
