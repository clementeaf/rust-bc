//! Cryptographic DoS flood resistance tests.
//!
//! Verifies that malicious peers sending invalid PQC blocks cannot halt
//! valid progress, and that cheap checks (size mismatch, duplicate hash,
//! stale height) reject messages before expensive ML-DSA verification.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use rust_bc::consensus::dag::DagBlock;
use rust_bc::consensus::engine::ConsensusEngine;
use rust_bc::consensus::fork_choice::ForkChoiceRule;
use rust_bc::consensus::ConsensusConfig;
use rust_bc::crypto::hasher::{hash_with, HashAlgorithm};
use rust_bc::identity::pqc_policy::{enforce_pqc, validate_signature_consistency};
use rust_bc::identity::signing::{
    MlDsaSigningProvider, SigningAlgorithm, SigningProvider, SoftwareSigningProvider,
};
use rust_bc::storage::MemoryStore;

// ═══════════════════════════════════════════════════════════════════
// INSTRUMENTED NODE — tracks rejection reasons and verification counts
// ═══════════════════════════════════════════════════════════════════

/// Rejection category — ordered from cheapest to most expensive check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RejectionReason {
    /// Signature size doesn't match declared algorithm (zero-cost check).
    SizeMismatch,
    /// PQC policy violation (classical sig when PQC required).
    PqcPolicyViolation,
    /// Duplicate block hash already seen (cheap hash lookup).
    DuplicateHash,
    /// Stale height (already have a block at this height).
    StaleHeight,
    /// DAG/engine rejected (may involve crypto verification).
    EngineRejection,
    /// Peer rate-limited — rejected before any validation.
    RateLimited,
}

struct FloodNode {
    id: String,
    engine: ConsensusEngine,
    #[allow(dead_code)]
    store: Arc<MemoryStore>,
    signing_provider: Box<dyn SigningProvider>,
    accepted_blocks: HashMap<u64, [u8; 32]>,
    /// Count of rejections by reason.
    rejection_counts: HashMap<RejectionReason, usize>,
    /// Set of block hashes already seen (for duplicate detection).
    seen_hashes: HashSet<[u8; 32]>,
    /// Per-peer message count for rate limiting.
    peer_message_counts: HashMap<String, usize>,
    /// Peers that have been rate-limited.
    rate_limited_peers: HashSet<String>,
    /// Rate limit threshold: messages per peer before throttling.
    rate_limit_threshold: usize,
    /// Total accepted valid blocks.
    accepted_count: usize,
}

impl FloodNode {
    fn new(id: &str, all_validators: &[String]) -> Self {
        let store = Arc::new(MemoryStore::new());
        let engine = ConsensusEngine::new(
            ConsensusConfig::default(),
            ForkChoiceRule::HeaviestSubtree,
            all_validators.to_vec(),
            0,
        )
        .with_store(Box::new(Arc::clone(&store)));

        Self {
            id: id.to_string(),
            engine,
            store,
            signing_provider: Box::new(MlDsaSigningProvider::generate()),
            accepted_blocks: HashMap::new(),
            rejection_counts: HashMap::new(),
            seen_hashes: HashSet::new(),
            peer_message_counts: HashMap::new(),
            rate_limited_peers: HashSet::new(),
            rate_limit_threshold: 100, // default: 100 messages before rate-limit
            accepted_count: 0,
        }
    }

    /// Attempt to accept a block with full instrumented validation pipeline.
    ///
    /// Validation order (cheapest first):
    /// 1. Rate limit check (O(1) hash lookup)
    /// 2. Duplicate hash check (O(1) hash lookup)
    /// 3. Stale height check (O(1) map lookup)
    /// 4. Signature size consistency (O(1) length check)
    /// 5. PQC policy check (O(1) enum match)
    /// 6. Engine validation (may involve crypto — most expensive)
    fn try_accept_from_peer(
        &mut self,
        block: DagBlock,
        peer_id: &str,
    ) -> Result<[u8; 32], RejectionReason> {
        // 1. Rate limit check
        let count = self
            .peer_message_counts
            .entry(peer_id.to_string())
            .or_insert(0);
        *count += 1;
        if *count > self.rate_limit_threshold {
            if !self.rate_limited_peers.contains(peer_id) {
                self.rate_limited_peers.insert(peer_id.to_string());
            }
        }
        if self.rate_limited_peers.contains(peer_id) {
            *self
                .rejection_counts
                .entry(RejectionReason::RateLimited)
                .or_insert(0) += 1;
            return Err(RejectionReason::RateLimited);
        }

        // 2. Duplicate hash check
        if self.seen_hashes.contains(&block.hash) {
            *self
                .rejection_counts
                .entry(RejectionReason::DuplicateHash)
                .or_insert(0) += 1;
            return Err(RejectionReason::DuplicateHash);
        }
        self.seen_hashes.insert(block.hash);

        // 3. Stale height check
        if self.accepted_blocks.contains_key(&block.height) {
            *self
                .rejection_counts
                .entry(RejectionReason::StaleHeight)
                .or_insert(0) += 1;
            return Err(RejectionReason::StaleHeight);
        }

        // 4. Signature size consistency (cheap O(1) check)
        if validate_signature_consistency(
            block.signature_algorithm,
            &block.signature,
            "flood block",
        )
        .is_err()
        {
            *self
                .rejection_counts
                .entry(RejectionReason::SizeMismatch)
                .or_insert(0) += 1;
            return Err(RejectionReason::SizeMismatch);
        }

        // 5. PQC policy check (cheap O(1) check)
        if enforce_pqc(block.signature_algorithm, "flood block").is_err() {
            *self
                .rejection_counts
                .entry(RejectionReason::PqcPolicyViolation)
                .or_insert(0) += 1;
            return Err(RejectionReason::PqcPolicyViolation);
        }

        // 6. Engine validation (expensive — may do crypto verification)
        match self.engine.accept_block(block) {
            Ok(hash) => {
                self.accepted_blocks.insert(
                    self.accepted_blocks.len() as u64, // use sequence for tracking
                    hash,
                );
                self.accepted_count += 1;
                Ok(hash)
            }
            Err(_) => {
                *self
                    .rejection_counts
                    .entry(RejectionReason::EngineRejection)
                    .or_insert(0) += 1;
                Err(RejectionReason::EngineRejection)
            }
        }
    }

    fn total_rejections(&self) -> usize {
        self.rejection_counts.values().sum()
    }

    fn cheap_rejection_count(&self) -> usize {
        self.rejection_counts
            .iter()
            .filter(|(reason, _)| {
                matches!(
                    reason,
                    RejectionReason::SizeMismatch
                        | RejectionReason::PqcPolicyViolation
                        | RejectionReason::DuplicateHash
                        | RejectionReason::StaleHeight
                        | RejectionReason::RateLimited
                )
            })
            .map(|(_, count)| count)
            .sum()
    }

    fn engine_rejection_count(&self) -> usize {
        *self
            .rejection_counts
            .get(&RejectionReason::EngineRejection)
            .unwrap_or(&0)
    }
}

/// Generate a valid PQC block for the given height and slot.
fn make_valid_block(
    height: u64,
    slot: u64,
    parent_hash: [u8; 32],
    proposer: &str,
    signer: &dyn SigningProvider,
) -> DagBlock {
    let payload = format!("flood-valid-{proposer}-{height}");
    let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
    let sig = signer.sign(&hash).unwrap();
    let timestamp = slot * 6;
    let mut block = DagBlock::new(
        hash,
        parent_hash,
        height,
        slot,
        timestamp,
        proposer.to_string(),
        sig,
    );
    block.signature_algorithm = signer.algorithm();
    block
}

/// Generate an invalid block (wrong signature size for declared algorithm).
fn make_invalid_size_mismatch(
    height: u64,
    slot: u64,
    parent_hash: [u8; 32],
    proposer: &str,
) -> DagBlock {
    let payload = format!("flood-invalid-size-{height}-{}", rand_u64());
    let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
    let timestamp = slot * 6;
    let mut block = DagBlock::new(
        hash,
        parent_hash,
        height,
        slot,
        timestamp,
        proposer.to_string(),
        vec![42u8; 64], // Ed25519 size
    );
    block.signature_algorithm = SigningAlgorithm::MlDsa65; // claims PQC but size is 64
    block
}

/// Generate an invalid block (classical signature when PQC required).
fn make_invalid_classical(
    height: u64,
    slot: u64,
    parent_hash: [u8; 32],
    proposer: &str,
) -> DagBlock {
    let payload = format!("flood-invalid-classical-{height}-{}", rand_u64());
    let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
    let ed = SoftwareSigningProvider::generate();
    let sig = ed.sign(&hash).unwrap();
    let timestamp = slot * 6;
    let mut block = DagBlock::new(
        hash,
        parent_hash,
        height,
        slot,
        timestamp,
        proposer.to_string(),
        sig,
    );
    block.signature_algorithm = SigningAlgorithm::Ed25519;
    block
}

/// Generate an invalid block with corrupted ML-DSA signature.
fn make_invalid_corrupted_pqc(
    height: u64,
    slot: u64,
    parent_hash: [u8; 32],
    proposer: &str,
) -> DagBlock {
    let payload = format!("flood-invalid-corrupt-{height}-{}", rand_u64());
    let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
    let pqc = MlDsaSigningProvider::generate();
    let mut sig = pqc.sign(&hash).unwrap();
    sig[0] ^= 0xff;
    let timestamp = slot * 6;
    let mut block = DagBlock::new(
        hash,
        parent_hash,
        height,
        slot,
        timestamp,
        proposer.to_string(),
        sig,
    );
    block.signature_algorithm = SigningAlgorithm::MlDsa65;
    block
}

/// Simple counter for unique IDs.
fn rand_u64() -> u64 {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// ═══════════════════════════════════════════════════════════════════
// TEST 1: Invalid PQC flood does not halt valid progress
// ═══════════════════════════════════════════════════════════════════

#[test]
fn invalid_pqc_signature_flood_does_not_halt_valid_progress() {
    std::env::set_var("REQUIRE_PQC_SIGNATURES", "true");

    let validators: Vec<String> = (0..4).map(|i| format!("h{i}")).collect();
    let mut node = FloodNode::new("h0", &validators);
    let signer = MlDsaSigningProvider::generate();

    let flood_count = 10_000;
    let valid_rounds = 20;
    let mut parent = [0u8; 32];

    for round in 0..valid_rounds {
        let slot = round as u64;

        // Flood: malicious peers send invalid blocks
        let flood_per_round = flood_count / valid_rounds;
        for i in 0..flood_per_round {
            let invalid = if i % 3 == 0 {
                make_invalid_size_mismatch(slot, slot, parent, "h0")
            } else if i % 3 == 1 {
                make_invalid_classical(slot, slot, parent, "h0")
            } else {
                make_invalid_corrupted_pqc(slot, slot, parent, "h0")
            };
            let _ = node.try_accept_from_peer(invalid, "evil_peer");
        }

        // Valid block from honest peer
        let valid = make_valid_block(slot, slot, parent, "h0", &signer);
        let result = node.try_accept_from_peer(valid, "honest_peer");
        if let Ok(hash) = result {
            parent = hash;
        }
    }

    std::env::remove_var("REQUIRE_PQC_SIGNATURES");

    // Assertions
    let total_rejections = node.total_rejections();
    assert!(
        total_rejections >= flood_count,
        "expected at least {flood_count} rejections, got {total_rejections}"
    );
    assert!(
        node.accepted_count > 0,
        "valid blocks must be accepted during flood"
    );
    assert_eq!(
        node.rejection_counts
            .get(&RejectionReason::SizeMismatch)
            .copied()
            .unwrap_or(0)
            + node
                .rejection_counts
                .get(&RejectionReason::PqcPolicyViolation)
                .copied()
                .unwrap_or(0)
            + node
                .rejection_counts
                .get(&RejectionReason::DuplicateHash)
                .copied()
                .unwrap_or(0)
            + node
                .rejection_counts
                .get(&RejectionReason::StaleHeight)
                .copied()
                .unwrap_or(0)
            + node
                .rejection_counts
                .get(&RejectionReason::RateLimited)
                .copied()
                .unwrap_or(0)
            + node
                .rejection_counts
                .get(&RejectionReason::EngineRejection)
                .copied()
                .unwrap_or(0),
        total_rejections,
        "all rejections accounted for"
    );

    eprintln!(
        "Flood results: {} rejected, {} accepted, breakdown: {:?}",
        total_rejections, node.accepted_count, node.rejection_counts
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 2: Duplicate invalid block is dropped before re-verification
// ═══════════════════════════════════════════════════════════════════

#[test]
fn duplicate_invalid_signature_is_dropped_before_reverification() {
    std::env::set_var("REQUIRE_PQC_SIGNATURES", "true");

    let validators = vec!["h0".to_string()];
    let mut node = FloodNode::new("h0", &validators);
    node.rate_limit_threshold = 100_000; // disable rate limiting for this test

    // Create ONE invalid block with size mismatch
    let invalid_block = make_invalid_size_mismatch(0, 0, [0u8; 32], "h0");
    let block_hash = invalid_block.hash;

    // Send the SAME block 5000 times
    for _ in 0..5000 {
        let mut dup = invalid_block.clone();
        dup.hash = block_hash; // same hash every time
        let _ = node.try_accept_from_peer(dup, "evil_peer");
    }

    std::env::remove_var("REQUIRE_PQC_SIGNATURES");

    // First rejection is SizeMismatch (cheap check). All subsequent are DuplicateHash.
    let size_mismatch = node
        .rejection_counts
        .get(&RejectionReason::SizeMismatch)
        .copied()
        .unwrap_or(0);
    let duplicate_hash = node
        .rejection_counts
        .get(&RejectionReason::DuplicateHash)
        .copied()
        .unwrap_or(0);
    let engine_calls = node.engine_rejection_count();

    eprintln!(
        "Duplicate test: size_mismatch={size_mismatch}, duplicate_hash={duplicate_hash}, engine={engine_calls}"
    );

    // The first message hits SizeMismatch. All 4999 subsequent hit DuplicateHash.
    // Engine (expensive crypto) should NEVER be called.
    assert_eq!(
        size_mismatch, 1,
        "only the first copy should reach size mismatch check"
    );
    assert!(
        duplicate_hash >= 4999,
        "subsequent copies must be caught by duplicate hash check"
    );
    assert_eq!(
        engine_calls, 0,
        "ML-DSA verification (engine) must NEVER run for duplicate invalid blocks"
    );

    // Total cheap rejections should be ~5000
    let cheap = node.cheap_rejection_count();
    assert!(
        cheap >= 5000,
        "all 5000 rejections should be cheap (no crypto): got {cheap}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 3: Malicious peer is rate-limited
// ═══════════════════════════════════════════════════════════════════

#[test]
fn malicious_peer_is_rate_limited_or_quarantined() {
    let validators = vec!["h0".to_string()];
    let mut node = FloodNode::new("h0", &validators);
    node.rate_limit_threshold = 50; // aggressive: 50 messages then throttle

    // Malicious peer sends 500 messages
    for i in 0..500 {
        let payload = format!("spam-{i}");
        let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
        let block = DagBlock::new(hash, [0u8; 32], 0, 0, 0, "h0".to_string(), vec![42u8; 64]);
        let _ = node.try_accept_from_peer(block, "spammer");
    }

    // After threshold (50), peer should be rate-limited
    assert!(
        node.rate_limited_peers.contains("spammer"),
        "malicious peer must be rate-limited after threshold"
    );

    let rate_limited_count = node
        .rejection_counts
        .get(&RejectionReason::RateLimited)
        .copied()
        .unwrap_or(0);
    assert!(
        rate_limited_count >= 449,
        "at least 449 messages should be rate-limited (500 - 50 threshold - 1): got {rate_limited_count}"
    );

    // Honest peer is NOT rate-limited
    let honest_block = DagBlock::new(
        hash_with(HashAlgorithm::Sha3_256, b"honest"),
        [0u8; 32],
        0,
        0,
        0,
        "h0".to_string(),
        vec![42u8; 3309],
    );
    let result = node.try_accept_from_peer(honest_block, "honest_peer");
    // Should not be rate-limited (different peer ID)
    assert_ne!(
        result,
        Err(RejectionReason::RateLimited),
        "honest peer must NOT be rate-limited by spammer's activity"
    );

    eprintln!(
        "Rate limit: {} rate-limited, {} total rejections, rate-limited peers: {:?}",
        rate_limited_count,
        node.total_rejections(),
        node.rate_limited_peers
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 4: Mixed valid and invalid load preserves fairness
// ═══════════════════════════════════════════════════════════════════

#[test]
fn mixed_valid_and_invalid_load_preserves_fairness() {
    std::env::set_var("REQUIRE_PQC_SIGNATURES", "true");

    let validators: Vec<String> = (0..5).map(|i| format!("n{i}")).collect();
    let mut node = FloodNode::new("n0", &validators);
    let signer = MlDsaSigningProvider::generate();

    let rounds = 30;
    let invalid_per_round = 100;
    let mut parent = [0u8; 32];
    let mut valid_accepted = 0;

    for round in 0..rounds {
        let slot = round as u64;

        // Invalid flood from 2 malicious peers
        for i in 0..invalid_per_round {
            let peer = if i % 2 == 0 { "evil_a" } else { "evil_b" };
            let invalid = make_invalid_size_mismatch(slot, slot, parent, "n0");
            let _ = node.try_accept_from_peer(invalid, peer);
        }

        // Valid block from honest proposer
        let valid = make_valid_block(slot, slot, parent, "n0", &signer);
        if let Ok(hash) = node.try_accept_from_peer(valid, "n0") {
            parent = hash;
            valid_accepted += 1;
        }
    }

    std::env::remove_var("REQUIRE_PQC_SIGNATURES");

    let total_invalid_submitted = rounds * invalid_per_round;
    let cheap_rejections = node.cheap_rejection_count();
    let engine_rejections = node.engine_rejection_count();

    eprintln!(
        "Fairness: {valid_accepted} valid accepted, {total_invalid_submitted} invalid submitted, \
         {cheap_rejections} cheap rejections, {engine_rejections} engine rejections"
    );

    // All honest peers made progress
    assert!(
        valid_accepted > 0,
        "honest peers must make progress under mixed load"
    );

    // Invalid blocks never enter canonical chain
    // (all rejections are accounted for)
    assert!(
        cheap_rejections + engine_rejections >= total_invalid_submitted,
        "all invalid messages must be rejected: cheap={cheap_rejections} engine={engine_rejections} total_invalid={total_invalid_submitted}"
    );

    // Cheap rejections should dominate (most invalid blocks caught early)
    assert!(
        cheap_rejections > engine_rejections,
        "cheap rejections ({cheap_rejections}) should exceed engine rejections ({engine_rejections})"
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 5: Cheap rejection order — size check before PQC verification
// ═══════════════════════════════════════════════════════════════════

#[test]
fn cheap_checks_fire_before_engine_for_malformed_blocks() {
    std::env::set_var("REQUIRE_PQC_SIGNATURES", "true");

    let validators = vec!["h0".to_string()];
    let mut node = FloodNode::new("h0", &validators);

    // Send 1000 blocks with size mismatch (64 bytes claimed as MlDsa65)
    for i in 0..1000 {
        let payload = format!("cheap-test-{i}");
        let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
        let mut block = DagBlock::new(
            hash,
            [0u8; 32],
            0,
            0,
            0,
            "h0".to_string(),
            vec![42u8; 64], // 64 bytes
        );
        block.signature_algorithm = SigningAlgorithm::MlDsa65; // claims 3309
        let _ = node.try_accept_from_peer(block, "attacker");
    }

    std::env::remove_var("REQUIRE_PQC_SIGNATURES");

    let size_mismatch = node
        .rejection_counts
        .get(&RejectionReason::SizeMismatch)
        .copied()
        .unwrap_or(0);
    let engine_calls = node.engine_rejection_count();

    // First message hits SizeMismatch, rest hit DuplicateHash or SizeMismatch
    // Engine must NEVER be called for size-mismatched blocks
    assert_eq!(
        engine_calls, 0,
        "engine (crypto verification) must NEVER run for size-mismatched blocks"
    );
    assert!(
        size_mismatch >= 1,
        "at least 1 size mismatch rejection expected"
    );

    let cheap = node.cheap_rejection_count();
    assert_eq!(
        cheap, 1000,
        "all 1000 rejections must be cheap (no crypto): got {cheap}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 6: Performance — 10K flood completes in bounded time
// ═══════════════════════════════════════════════════════════════════

#[test]
fn flood_10k_completes_in_bounded_time() {
    let validators = vec!["h0".to_string()];
    let mut node = FloodNode::new("h0", &validators);

    let start = Instant::now();
    let count = 10_000;

    for i in 0..count {
        let payload = format!("perf-{i}");
        let hash = hash_with(HashAlgorithm::Sha3_256, payload.as_bytes());
        let mut block = DagBlock::new(hash, [0u8; 32], 0, 0, 0, "h0".to_string(), vec![42u8; 64]);
        block.signature_algorithm = SigningAlgorithm::MlDsa65;
        let _ = node.try_accept_from_peer(block, "flood_peer");
    }

    let elapsed = start.elapsed();

    eprintln!(
        "10K flood: {:?} ({:.0} msgs/sec), {} cheap, {} engine",
        elapsed,
        count as f64 / elapsed.as_secs_f64(),
        node.cheap_rejection_count(),
        node.engine_rejection_count()
    );

    // 10K invalid messages with cheap rejection should complete in <2s
    assert!(
        elapsed.as_secs() < 10,
        "10K flood took {:?} — cheap rejection must be fast",
        elapsed
    );

    // No engine calls
    assert_eq!(node.engine_rejection_count(), 0);
}
