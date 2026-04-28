//! Equivocation persistence and cross-partition retroactive detection tests.
//!
//! Part 1: Penalty state survives node restart (file-based persistence).
//! Part 2: Cross-partition equivocation detected after healing.

use std::collections::HashSet;
use std::path::Path;

use rust_bc::consensus::equivocation::EquivocationDetector;
use rust_bc::crypto::hasher::{hash_with, HashAlgorithm};
use rust_bc::identity::signing::{MlDsaSigningProvider, SigningAlgorithm, SigningProvider};

// ═══════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════

const EQUIVOCATION_FILE: &str = "equivocation_state.json";

fn sign_block_hash(signer: &dyn SigningProvider, label: &str) -> ([u8; 32], Vec<u8>) {
    let hash = hash_with(HashAlgorithm::Sha3_256, label.as_bytes());
    let sig = signer.sign(&hash).unwrap();
    (hash, sig)
}

/// Persist equivocation detector state to a JSON file in the given directory.
fn save_equivocation_state(dir: &Path, detector: &EquivocationDetector) {
    let file_path = dir.join(EQUIVOCATION_FILE);
    let data = detector.to_bytes();
    std::fs::write(&file_path, &data).expect("persist equivocation state");
}

/// Load equivocation detector state from the JSON file in the given directory.
fn load_equivocation_state(dir: &Path) -> Option<EquivocationDetector> {
    let file_path = dir.join(EQUIVOCATION_FILE);
    let data = std::fs::read(&file_path).ok()?;
    EquivocationDetector::from_bytes(&data)
}

// ═══════════════════════════════════════════════════════════════════
// PART 1: Penalty survives restart
// ═══════════════════════════════════════════════════════════════════

#[test]
fn equivocation_penalty_survives_restart() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let signer = MlDsaSigningProvider::generate();
    let proposer_id = "byzantine-validator";

    // Phase 1: Create equivocation and persist
    {
        let mut detector = EquivocationDetector::new();

        let (hash_a, sig_a) = sign_block_hash(&signer, "persist-block-A");
        let (hash_b, sig_b) = sign_block_hash(&signer, "persist-block-B");

        // Verify both signatures are valid
        assert!(signer.verify(&hash_a, &sig_a).unwrap());
        assert!(signer.verify(&hash_b, &sig_b).unwrap());

        // Submit both — triggers equivocation
        detector.check_proposal(5, 5, proposer_id, hash_a, &sig_a, SigningAlgorithm::MlDsa65);
        let proof = detector
            .check_proposal(5, 5, proposer_id, hash_b, &sig_b, SigningAlgorithm::MlDsa65)
            .expect("equivocation must be detected");

        assert!(proof.is_valid());
        assert!(detector.is_penalized(proposer_id));
        assert_eq!(detector.proof_count_for(proposer_id), 1);

        // Persist state
        save_equivocation_state(dir.path(), &detector);
    }
    // Detector dropped — simulates crash

    // Phase 2: Restart — load from disk
    {
        let restored = load_equivocation_state(dir.path())
            .expect("equivocation state must be loadable after restart");

        // Penalty must survive
        assert!(
            restored.is_penalized(proposer_id),
            "CRITICAL: Equivocation penalty was lost after restart"
        );
        assert_eq!(
            restored.proof_count_for(proposer_id),
            1,
            "proof count must be 1 after restart"
        );

        // The proof itself must be intact
        let proofs = restored.proofs();
        assert_eq!(proofs.len(), 1);
        assert!(proofs[0].is_valid());
        assert_eq!(proofs[0].position.proposer, proposer_id);
        assert_eq!(proofs[0].position.height, 5);

        // Penalized validator's future blocks should be rejectable
        assert!(
            restored.is_penalized(proposer_id),
            "consensus layer must check is_penalized() and reject blocks from this validator"
        );
    }
}

#[test]
fn restart_does_not_create_false_penalty() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let signer = MlDsaSigningProvider::generate();

    // Phase 1: Normal operation — no equivocation
    {
        let mut detector = EquivocationDetector::new();
        let (hash, sig) = sign_block_hash(&signer, "honest-block");
        let result = detector.check_proposal(
            0,
            0,
            "honest-validator",
            hash,
            &sig,
            SigningAlgorithm::MlDsa65,
        );
        assert!(result.is_none());
        assert!(!detector.is_penalized("honest-validator"));
        save_equivocation_state(dir.path(), &detector);
    }

    // Phase 2: Restart
    {
        let restored = load_equivocation_state(dir.path()).expect("state must load");
        assert!(
            !restored.is_penalized("honest-validator"),
            "honest validator must NOT be penalized after restart"
        );
        assert_eq!(restored.proofs().len(), 0);
        assert_eq!(restored.penalized_count(), 0);
    }
}

#[test]
fn equivocation_state_survives_multiple_restarts() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let signer = MlDsaSigningProvider::generate();

    // Create equivocation
    {
        let mut det = EquivocationDetector::new();
        let (ha, sa) = sign_block_hash(&signer, "mr-A");
        let (hb, sb) = sign_block_hash(&signer, "mr-B");
        det.check_proposal(1, 1, "multi-cheater", ha, &sa, SigningAlgorithm::MlDsa65);
        det.check_proposal(1, 1, "multi-cheater", hb, &sb, SigningAlgorithm::MlDsa65);
        save_equivocation_state(dir.path(), &det);
    }

    // Restart 3 times — penalty must persist each time
    for restart_num in 1..=3 {
        let det = load_equivocation_state(dir.path()).expect("load state");
        assert!(
            det.is_penalized("multi-cheater"),
            "penalty lost after restart #{restart_num}"
        );
        assert_eq!(det.proof_count_for("multi-cheater"), 1);
        // Re-persist (simulates node running then shutting down again)
        save_equivocation_state(dir.path(), &det);
    }
}

// ═══════════════════════════════════════════════════════════════════
// PART 2: Cross-partition retroactive equivocation detection
// ═══════════════════════════════════════════════════════════════════

/// Simulated partition node with its own equivocation detector.
struct PartitionNode {
    id: String,
    detector: EquivocationDetector,
    seen_blocks: Vec<([u8; 32], Vec<u8>, String)>,
}

impl PartitionNode {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            detector: EquivocationDetector::new(),
            seen_blocks: Vec::new(),
        }
    }

    fn receive_block(
        &mut self,
        height: u64,
        slot: u64,
        proposer: &str,
        hash: [u8; 32],
        sig: &[u8],
    ) -> Option<rust_bc::consensus::equivocation::EquivocationProof> {
        self.seen_blocks
            .push((hash, sig.to_vec(), proposer.to_string()));
        self.detector
            .check_proposal(height, slot, proposer, hash, sig, SigningAlgorithm::MlDsa65)
    }
}

#[test]
fn equivocation_across_partition_detected_after_healing() {
    let signer = MlDsaSigningProvider::generate();
    let byzantine_id = "byzantine-proposer";

    let (hash_a, sig_a) = sign_block_hash(&signer, "partition-block-A");
    let (hash_b, sig_b) = sign_block_hash(&signer, "partition-block-B");
    assert_ne!(hash_a, hash_b);
    assert!(signer.verify(&hash_a, &sig_a).unwrap());
    assert!(signer.verify(&hash_b, &sig_b).unwrap());

    let height = 10;
    let slot = 10;

    // Group A: nodes 0, 1, 2
    let mut group_a: Vec<PartitionNode> = (0..3)
        .map(|i| PartitionNode::new(&format!("a{i}")))
        .collect();
    // Group B: nodes 3, 4, 5
    let mut group_b: Vec<PartitionNode> = (0..3)
        .map(|i| PartitionNode::new(&format!("b{i}")))
        .collect();

    // Byzantine proposer sends block_A to Group A only
    for node in &mut group_a {
        let result = node.receive_block(height, slot, byzantine_id, hash_a, &sig_a);
        assert!(
            result.is_none(),
            "Group A sees only block_A — no equivocation yet"
        );
    }

    // Byzantine proposer sends block_B to Group B only
    for node in &mut group_b {
        let result = node.receive_block(height, slot, byzantine_id, hash_b, &sig_b);
        assert!(
            result.is_none(),
            "Group B sees only block_B — no equivocation yet"
        );
    }

    // During partition: no equivocation detected
    for node in group_a.iter().chain(group_b.iter()) {
        assert!(
            !node.detector.is_penalized(byzantine_id),
            "no equivocation during partition for {}",
            node.id
        );
    }

    // ── HEAL PARTITION ──
    let a_blocks: Vec<([u8; 32], Vec<u8>, String)> =
        group_a.iter().flat_map(|n| n.seen_blocks.clone()).collect();
    let b_blocks: Vec<([u8; 32], Vec<u8>, String)> =
        group_b.iter().flat_map(|n| n.seen_blocks.clone()).collect();

    let mut all_detected: Vec<String> = Vec::new();

    // Share B's blocks with A
    for node in &mut group_a {
        for (hash, sig, proposer) in &b_blocks {
            if let Some(proof) = node.receive_block(height, slot, proposer, *hash, sig) {
                all_detected.push(node.id.clone());
                assert!(proof.is_valid());
                assert_eq!(proof.position.proposer, byzantine_id);
            }
        }
    }

    // Share A's blocks with B
    for node in &mut group_b {
        for (hash, sig, proposer) in &a_blocks {
            if let Some(proof) = node.receive_block(height, slot, proposer, *hash, sig) {
                all_detected.push(node.id.clone());
                assert!(proof.is_valid());
                assert_eq!(proof.position.proposer, byzantine_id);
            }
        }
    }

    // ALL honest nodes must detect equivocation after healing
    for node in group_a.iter().chain(group_b.iter()) {
        assert!(
            node.detector.is_penalized(byzantine_id),
            "node {} must detect equivocation after healing",
            node.id
        );
        assert_eq!(node.detector.proof_count_for(byzantine_id), 1);
    }

    let detected_groups: HashSet<&str> = all_detected
        .iter()
        .map(|id| if id.starts_with('a') { "A" } else { "B" })
        .collect();
    assert!(detected_groups.contains("A"));
    assert!(detected_groups.contains("B"));
}

#[test]
fn cross_partition_same_block_duplicate_not_equivocation() {
    let signer = MlDsaSigningProvider::generate();
    let (hash, sig) = sign_block_hash(&signer, "same-block-both-partitions");

    let mut node_a = PartitionNode::new("a0");
    let mut node_b = PartitionNode::new("b0");

    node_a.receive_block(5, 5, "honest-proposer", hash, &sig);
    node_b.receive_block(5, 5, "honest-proposer", hash, &sig);

    // After healing: share blocks — same hash = not equivocation
    let b_blocks = node_b.seen_blocks.clone();
    for (h, s, p) in &b_blocks {
        let result = node_a.receive_block(5, 5, p, *h, s);
        assert!(
            result.is_none(),
            "same block in both partitions is NOT equivocation"
        );
    }

    assert!(!node_a.detector.is_penalized("honest-proposer"));
    assert!(!node_b.detector.is_penalized("honest-proposer"));
    assert_eq!(node_a.detector.proofs().len(), 0);
}

#[test]
fn gossip_propagates_proof_after_partition_healing() {
    let signer = MlDsaSigningProvider::generate();
    let byzantine_id = "cross-partition-cheater";

    let (hash_a, sig_a) = sign_block_hash(&signer, "gossip-cross-A");
    let (hash_b, sig_b) = sign_block_hash(&signer, "gossip-cross-B");

    // Node A detects equivocation locally
    let mut detector_a = EquivocationDetector::new();
    detector_a.check_proposal(
        3,
        3,
        byzantine_id,
        hash_a,
        &sig_a,
        SigningAlgorithm::MlDsa65,
    );
    let proof = detector_a
        .check_proposal(
            3,
            3,
            byzantine_id,
            hash_b,
            &sig_b,
            SigningAlgorithm::MlDsa65,
        )
        .expect("equivocation detected");

    // Node B only saw block_B — no local equivocation
    let mut detector_b = EquivocationDetector::new();
    detector_b.check_proposal(
        3,
        3,
        byzantine_id,
        hash_b,
        &sig_b,
        SigningAlgorithm::MlDsa65,
    );
    assert!(!detector_b.is_penalized(byzantine_id));

    // B receives proof via gossip
    assert!(detector_b.receive_proof(&proof));
    assert!(detector_b.is_penalized(byzantine_id));

    // Duplicate gossip ignored
    assert!(!detector_b.receive_proof(&proof));
    assert_eq!(detector_b.proof_count_for(byzantine_id), 1);
}
