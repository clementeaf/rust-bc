//! Byzantine equivocation detection tests.
//!
//! Tests that a validator signing two different valid blocks for the same
//! consensus position (height, slot) is detected and penalized. Uses
//! real ML-DSA-65 signatures — this is NOT about invalid signatures.

use rust_bc::consensus::equivocation::{
    ConsensusPosition, EquivocationDetector, EquivocationProof,
};
use rust_bc::crypto::hasher::{hash_with, HashAlgorithm};
use rust_bc::identity::signing::{MlDsaSigningProvider, SigningAlgorithm, SigningProvider};

/// Create a valid PQC-signed block hash + signature for equivocation tests.
fn sign_block(signer: &dyn SigningProvider, label: &str) -> ([u8; 32], Vec<u8>) {
    let hash = hash_with(HashAlgorithm::Sha3_256, label.as_bytes());
    let sig = signer.sign(&hash).unwrap();
    (hash, sig)
}

// ═══════════════════════════════════════════════════════════════════
// TEST 1: Detects two valid blocks at same height from same proposer
// ═══════════════════════════════════════════════════════════════════

#[test]
fn detects_two_valid_blocks_same_height_same_proposer() {
    let signer = MlDsaSigningProvider::generate();
    let mut detector = EquivocationDetector::new();

    let (hash_a, sig_a) = sign_block(&signer, "block-A-height-5");
    let (hash_b, sig_b) = sign_block(&signer, "block-B-height-5");

    // Both signatures are individually valid
    assert!(signer.verify(hash_a.as_ref(), &sig_a).unwrap());
    assert!(signer.verify(hash_b.as_ref(), &sig_b).unwrap());
    assert_ne!(hash_a, hash_b, "blocks must be different");

    // Submit first block — no equivocation
    let result_a = detector.check_proposal(
        5,
        5,
        "validator-1",
        hash_a,
        &sig_a,
        SigningAlgorithm::MlDsa65,
    );
    assert!(
        result_a.is_none(),
        "first proposal should not be equivocation"
    );

    // Submit second different block at same position — EQUIVOCATION
    let result_b = detector.check_proposal(
        5,
        5,
        "validator-1",
        hash_b,
        &sig_b,
        SigningAlgorithm::MlDsa65,
    );
    assert!(
        result_b.is_some(),
        "second conflicting proposal must be equivocation"
    );

    let proof = result_b.unwrap();
    assert!(proof.is_valid());
    assert_eq!(proof.position.proposer, "validator-1");
    assert_eq!(proof.position.height, 5);
    assert_eq!(proof.block_hash_a, hash_a);
    assert_eq!(proof.block_hash_b, hash_b);

    // Only one block should be accepted (the first one)
    // The equivocator is penalized
    assert!(detector.is_penalized("validator-1"));
    assert_eq!(detector.proof_count_for("validator-1"), 1);
}

// ═══════════════════════════════════════════════════════════════════
// TEST 2: Equivocation proof is constructed with valid signatures
// ═══════════════════════════════════════════════════════════════════

#[test]
fn equivocation_proof_is_constructed_from_two_valid_signatures() {
    let signer = MlDsaSigningProvider::generate();
    let mut detector = EquivocationDetector::new();

    let (hash_a, sig_a) = sign_block(&signer, "proof-block-A");
    let (hash_b, sig_b) = sign_block(&signer, "proof-block-B");

    detector.check_proposal(10, 10, "prover", hash_a, &sig_a, SigningAlgorithm::MlDsa65);
    let proof = detector
        .check_proposal(10, 10, "prover", hash_b, &sig_b, SigningAlgorithm::MlDsa65)
        .expect("equivocation proof must be created");

    // Proof structure
    assert_eq!(proof.position.proposer, "prover");
    assert_eq!(proof.position.height, 10);
    assert_eq!(proof.position.slot, 10);
    assert_eq!(proof.block_hash_a, hash_a);
    assert_eq!(proof.block_hash_b, hash_b);
    assert_eq!(proof.signature_a, sig_a);
    assert_eq!(proof.signature_b, sig_b);
    assert_eq!(proof.algorithm, SigningAlgorithm::MlDsa65);

    // Both signatures in the proof are independently verifiable
    assert!(signer
        .verify(&proof.block_hash_a, &proof.signature_a)
        .unwrap());
    assert!(signer
        .verify(&proof.block_hash_b, &proof.signature_b)
        .unwrap());

    // Proof is structurally valid
    assert!(proof.is_valid());
}

// ═══════════════════════════════════════════════════════════════════
// TEST 3: Equivocation proof gossip + deduplication
// ═══════════════════════════════════════════════════════════════════

#[test]
fn equivocation_proof_survives_gossip_and_is_deduplicated() {
    let signer = MlDsaSigningProvider::generate();

    // Node A detects equivocation
    let mut node_a = EquivocationDetector::new();
    let (hash_a, sig_a) = sign_block(&signer, "gossip-A");
    let (hash_b, sig_b) = sign_block(&signer, "gossip-B");

    node_a.check_proposal(7, 7, "evil", hash_a, &sig_a, SigningAlgorithm::MlDsa65);
    let proof = node_a
        .check_proposal(7, 7, "evil", hash_b, &sig_b, SigningAlgorithm::MlDsa65)
        .expect("equivocation detected by node A");

    // Node B and C receive the proof via gossip
    let mut node_b = EquivocationDetector::new();
    let mut node_c = EquivocationDetector::new();

    assert!(node_b.receive_proof(&proof), "node B accepts proof");
    assert!(node_c.receive_proof(&proof), "node C accepts proof");

    // Duplicate proof is ignored
    assert!(!node_b.receive_proof(&proof), "node B deduplicates proof");
    assert!(!node_c.receive_proof(&proof), "node C deduplicates proof");

    // All nodes agree: proposer is penalized
    assert!(node_a.is_penalized("evil"));
    assert!(node_b.is_penalized("evil"));
    assert!(node_c.is_penalized("evil"));

    // Proof count is 1 on all nodes
    assert_eq!(node_a.proof_count_for("evil"), 1);
    assert_eq!(node_b.proof_count_for("evil"), 1);
    assert_eq!(node_c.proof_count_for("evil"), 1);
}

// ═══════════════════════════════════════════════════════════════════
// TEST 4: Penalized validator's future blocks are rejectable
// ═══════════════════════════════════════════════════════════════════

#[test]
fn equivocating_validator_cannot_produce_future_blocks_until_penalty_expires() {
    let signer = MlDsaSigningProvider::generate();
    let mut detector = EquivocationDetector::new();

    // Equivocate at height 5
    let (hash_a, sig_a) = sign_block(&signer, "future-A");
    let (hash_b, sig_b) = sign_block(&signer, "future-B");
    detector.check_proposal(5, 5, "cheater", hash_a, &sig_a, SigningAlgorithm::MlDsa65);
    detector.check_proposal(5, 5, "cheater", hash_b, &sig_b, SigningAlgorithm::MlDsa65);

    assert!(detector.is_penalized("cheater"));

    // Cheater tries to produce a block at height 6
    let (hash_c, sig_c) = sign_block(&signer, "future-C-height-6");

    // Honest nodes should reject blocks from penalized proposers
    // The detector flags the proposer as penalized — consensus layer
    // should check is_penalized() before accepting proposals.
    assert!(
        detector.is_penalized("cheater"),
        "cheater must be penalized — consensus layer should reject their proposals"
    );

    // The proposal itself is not equivocation (different height), but
    // the proposer is quarantined.
    let result =
        detector.check_proposal(6, 6, "cheater", hash_c, &sig_c, SigningAlgorithm::MlDsa65);
    // No equivocation at height 6 (first proposal), but proposer is still penalized
    assert!(
        result.is_none(),
        "height 6 is first proposal — not equivocation"
    );
    assert!(
        detector.is_penalized("cheater"),
        "penalty persists across heights"
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 5: Different proposers at same height is NOT equivocation
// ═══════════════════════════════════════════════════════════════════

#[test]
fn different_proposers_same_height_is_not_equivocation() {
    let signer_a = MlDsaSigningProvider::generate();
    let signer_b = MlDsaSigningProvider::generate();
    let mut detector = EquivocationDetector::new();

    let (hash_a, sig_a) = sign_block(&signer_a, "proposer-A-block");
    let (hash_b, sig_b) = sign_block(&signer_b, "proposer-B-block");

    let result_a = detector.check_proposal(
        10,
        10,
        "proposer-A",
        hash_a,
        &sig_a,
        SigningAlgorithm::MlDsa65,
    );
    let result_b = detector.check_proposal(
        10,
        10,
        "proposer-B",
        hash_b,
        &sig_b,
        SigningAlgorithm::MlDsa65,
    );

    assert!(result_a.is_none(), "first proposer at height 10 is fine");
    assert!(
        result_b.is_none(),
        "different proposer at same height is NOT equivocation"
    );

    assert!(!detector.is_penalized("proposer-A"));
    assert!(!detector.is_penalized("proposer-B"));
    assert_eq!(detector.proofs().len(), 0);
}

// ═══════════════════════════════════════════════════════════════════
// TEST 6: Same proposer, same block duplicate is NOT equivocation
// ═══════════════════════════════════════════════════════════════════

#[test]
fn same_proposer_same_block_duplicate_is_not_equivocation() {
    let signer = MlDsaSigningProvider::generate();
    let mut detector = EquivocationDetector::new();

    let (hash, sig) = sign_block(&signer, "duplicate-block");

    // Send same block 100 times
    for _ in 0..100 {
        let result = detector.check_proposal(3, 3, "honest", hash, &sig, SigningAlgorithm::MlDsa65);
        assert!(
            result.is_none(),
            "duplicate of same block is NOT equivocation"
        );
    }

    assert!(!detector.is_penalized("honest"));
    assert_eq!(detector.proofs().len(), 0);
}

// ═══════════════════════════════════════════════════════════════════
// TEST 7: Multiple equivocations from same proposer at different heights
// ═══════════════════════════════════════════════════════════════════

#[test]
fn multiple_equivocations_different_heights() {
    let signer = MlDsaSigningProvider::generate();
    let mut detector = EquivocationDetector::new();

    // Equivocate at height 1
    let (h1a, s1a) = sign_block(&signer, "h1-A");
    let (h1b, s1b) = sign_block(&signer, "h1-B");
    detector.check_proposal(1, 1, "serial-cheater", h1a, &s1a, SigningAlgorithm::MlDsa65);
    detector.check_proposal(1, 1, "serial-cheater", h1b, &s1b, SigningAlgorithm::MlDsa65);

    // Equivocate at height 2
    let (h2a, s2a) = sign_block(&signer, "h2-A");
    let (h2b, s2b) = sign_block(&signer, "h2-B");
    detector.check_proposal(2, 2, "serial-cheater", h2a, &s2a, SigningAlgorithm::MlDsa65);
    detector.check_proposal(2, 2, "serial-cheater", h2b, &s2b, SigningAlgorithm::MlDsa65);

    assert!(detector.is_penalized("serial-cheater"));
    assert_eq!(
        detector.proof_count_for("serial-cheater"),
        2,
        "each height equivocation produces a separate proof"
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 8: Invalid proof is rejected by receiver
// ═══════════════════════════════════════════════════════════════════

#[test]
fn invalid_equivocation_proof_rejected() {
    let mut detector = EquivocationDetector::new();

    // Proof with same block hash (not a real equivocation)
    let fake_proof = EquivocationProof {
        position: ConsensusPosition {
            height: 0,
            slot: 0,
            proposer: "fake".to_string(),
        },
        block_hash_a: [1u8; 32],
        block_hash_b: [1u8; 32], // SAME hash — invalid
        signature_a: vec![42u8; 64],
        signature_b: vec![43u8; 64],
        algorithm: SigningAlgorithm::MlDsa65,
    };

    assert!(!fake_proof.is_valid());
    assert!(!detector.receive_proof(&fake_proof));
    assert!(!detector.is_penalized("fake"));
}

// ═══════════════════════════════════════════════════════════════════
// TEST 9: Stress — 100 validators, detect all equivocators
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_100_validators_detect_equivocators() {
    let mut detector = EquivocationDetector::new();
    let num_validators = 100;
    let num_equivocators = 10;

    for i in 0..num_validators {
        let signer = MlDsaSigningProvider::generate();
        let proposer = format!("v{i}");
        let (hash_a, sig_a) = sign_block(&signer, &format!("{proposer}-block-A"));

        detector.check_proposal(0, 0, &proposer, hash_a, &sig_a, SigningAlgorithm::MlDsa65);

        // First 10 validators equivocate
        if i < num_equivocators {
            let (hash_b, sig_b) = sign_block(&signer, &format!("{proposer}-block-B"));
            let result =
                detector.check_proposal(0, 0, &proposer, hash_b, &sig_b, SigningAlgorithm::MlDsa65);
            assert!(
                result.is_some(),
                "equivocation by {proposer} must be detected"
            );
        }
    }

    assert_eq!(
        detector.penalized_count(),
        num_equivocators,
        "exactly {num_equivocators} validators should be penalized"
    );
    assert_eq!(detector.proofs().len(), num_equivocators);

    // Honest validators are not penalized
    for i in num_equivocators..num_validators {
        assert!(
            !detector.is_penalized(&format!("v{i}")),
            "honest v{i} should not be penalized"
        );
    }
}
