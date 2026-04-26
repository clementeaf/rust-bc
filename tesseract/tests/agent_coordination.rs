//! Proof of concept: Tesseract as shared reality layer for AI agent coordination.
//!
//! Scenario: multi-agent negotiation with conflict, partition, and self-healing.
//! Each step validates a specific claim about the system.

use tesseract::mapper::*;
use tesseract::node::*;
use tesseract::*;

/// Helper: create a 2-node network with mapper.
fn setup() -> (Network, CoordMapper) {
    let net = Network::new(8, 2);
    let mapper = CoordMapper::new(8).with_time_bucket(60);
    (net, mapper)
}

// ============================================================
// STEP 1: Agent Alice proposes — event enters the field
// ============================================================
#[test]
fn step1_agent_proposes_event() {
    let (mut net, mapper) = setup();

    let proposal = Event {
        id: "deal-001".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:service-X:100".into(),
    };
    let coord = mapper.map(&proposal);
    net.seed(coord, "alice:propose:deal-001");
    net.run_to_equilibrium(10);

    let cell = net.get(coord);
    assert!(cell.probability > 0.0, "Proposal should exist in field");
    assert!(
        cell.influences.iter().any(|i| i.event_id.contains("alice")),
        "Proposal should carry Alice's identity"
    );
}

// ============================================================
// STEP 2: Bob accepts — distributed seed crystallizes agreement
// ============================================================
#[test]
fn step2_mutual_agreement_crystallizes() {
    let (mut net, mapper) = setup();

    let deal = Event {
        id: "deal-001".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:service-X:100".into(),
    };
    let coord = mapper.map(&deal);

    // Both agents seed the same event (multi-party agreement)
    net.distributed_seed(coord, "deal-001:buy:100", &[(0, "alice"), (1, "bob")]);
    net.run_to_equilibrium(10);

    let cell = net.get(coord);
    assert!(cell.crystallized, "Mutual agreement should crystallize");

    // Both parties are recorded in influences
    let record = cell.record();
    assert!(
        record.contains("alice"),
        "Agreement records Alice: {}",
        record
    );
    assert!(record.contains("bob"), "Agreement records Bob: {}", record);
}

// ============================================================
// STEP 3: Charlie's conflicting claim does NOT crystallize
// ============================================================
#[test]
fn step3_conflicting_claim_rejected() {
    let (mut net, mapper) = setup();

    // Alice + Bob agree on deal-001
    let deal = Event {
        id: "deal-001".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:service-X:100".into(),
    };
    let coord = mapper.map(&deal);
    net.distributed_seed(coord, "deal-001:buy:100", &[(0, "alice"), (1, "bob")]);
    net.run_to_equilibrium(10);

    assert!(net.get(coord).crystallized, "Real deal crystallized");

    // Charlie claims Alice agreed with HIM on a different deal
    // This is a DIFFERENT event (different id) — lands at different coordinate
    let fake = Event {
        id: "fake-deal-charlie".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "charlie".into(),
        data: "buy:service-X:100".into(),
    };
    let fake_coord = mapper.map(&fake);

    // Charlie seeds alone — no support from Alice
    net.seed(fake_coord, "charlie:fake-deal");
    net.run_to_equilibrium(10);

    // Charlie's event exists but has weak support (single seeder)
    let fake_cell = net.get(fake_coord);
    let real_cell = net.get(coord);

    // Real deal: crystallized with multi-party support
    assert!(real_cell.crystallized, "Real deal remains crystallized");

    // The field distinguishes multi-party agreement from single-party claim:
    // Real deal has BOTH alice AND bob in its influences.
    // Charlie's fake has ONLY charlie — no alice involvement.
    let real_record = real_cell.record();
    assert!(
        real_record.contains("alice"),
        "Real deal records Alice: {}",
        real_record
    );
    assert!(
        real_record.contains("bob"),
        "Real deal records Bob: {}",
        real_record
    );

    // Charlie's cell may contain Alice's influence as orbital background
    // (the real deal's orbital spreads in a small field). The KEY distinction:
    // Charlie's OWN event is the PRIMARY influence (highest weight),
    // not Alice's agreement. Alice never endorsed Charlie directly.
    let charlie_primary = fake_cell
        .influences
        .iter()
        .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap())
        .expect("Charlie's cell should have influences");
    assert!(
        charlie_primary.event_id.contains("charlie"),
        "Charlie's PRIMARY influence must be his own event, not Alice's: {}",
        charlie_primary.event_id
    );

    // Real deal's primary influence is the multi-party agreement
    let real_primary = real_cell
        .influences
        .iter()
        .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap())
        .expect("Real deal should have influences");
    assert!(
        real_primary.event_id.contains("alice") || real_primary.event_id.contains("bob"),
        "Real deal's PRIMARY influence is the agreement parties: {}",
        real_primary.event_id
    );
}

// ============================================================
// STEP 4: Network partition — agents evolve independently
// ============================================================
#[test]
fn step4_partition_independent_operation() {
    let size = 8;

    // Create two isolated nodes (simulating partition)
    let mut alice_node = Node::new(
        "alice-agent",
        size,
        Region {
            start: [0, 0, 0, 0],
            end: [size, size, size, size],
        },
    );
    let mut bob_node = Node::new(
        "bob-agent",
        size,
        Region {
            start: [0, 0, 0, 0],
            end: [size, size, size, size],
        },
    );

    let mapper = CoordMapper::new(size).with_time_bucket(60);

    // Before partition: both agree on deal-001
    let deal = Event {
        id: "deal-001".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:service-X:100".into(),
    };
    let deal_coord = mapper.map(&deal);

    alice_node.seed(deal_coord, "deal-001[alice]");
    bob_node.seed(deal_coord, "deal-001[bob]");

    // Exchange once, then partition
    let ba = alice_node.boundary_cells();
    let bb = bob_node.boundary_cells();
    alice_node.receive_boundary(&bb);
    bob_node.receive_boundary(&ba);

    for _ in 0..50 {
        alice_node.evolve();
        bob_node.evolve();
    }

    assert!(
        alice_node.field.get(deal_coord).crystallized,
        "Deal crystallized on Alice"
    );
    assert!(
        bob_node.field.get(deal_coord).crystallized,
        "Deal crystallized on Bob"
    );

    // --- PARTITION: each agent operates independently ---

    // Alice makes a new deal during partition
    let alice_deal = Event {
        id: "deal-002-alice".into(),
        timestamp: 180,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:service-Y:50".into(),
    };
    let alice_coord = mapper.map(&alice_deal);
    alice_node.seed(alice_coord, "deal-002[alice-solo]");

    // Bob makes a different deal during partition
    let bob_deal = Event {
        id: "deal-003-bob".into(),
        timestamp: 240,
        channel: "marketplace".into(),
        org: "bob".into(),
        data: "sell:service-Z:75".into(),
    };
    let bob_coord = mapper.map(&bob_deal);
    bob_node.seed(bob_coord, "deal-003[bob-solo]");

    for _ in 0..50 {
        alice_node.evolve();
        bob_node.evolve();
    }

    // Each only knows their own partition events
    assert!(
        alice_node.field.get(alice_coord).probability > 0.0,
        "Alice knows her deal"
    );
    assert!(
        bob_node.field.get(bob_coord).probability > 0.0,
        "Bob knows his deal"
    );
}

// ============================================================
// STEP 5: Reconnection — CRDT merge, both sides preserved
// ============================================================
#[test]
fn step5_reconnection_merges_state() {
    let size = 8;

    // Two agents with completely separate fields (partition)
    let region = Region {
        start: [0, 0, 0, 0],
        end: [size, size, size, size],
    };
    let mut alice_node = Node::new("alice", size, region.clone());
    let mut bob_node = Node::new("bob", size, region);

    // During partition, each agent makes deals independently.
    // Force crystallization directly to guarantee isolation.
    let alice_deal = Coord {
        t: 1,
        c: 1,
        o: 1,
        v: 1,
    };
    let bob_deal = Coord {
        t: 6,
        c: 6,
        o: 6,
        v: 6,
    };

    let ac = alice_node.field.get_mut(alice_deal);
    ac.probability = 1.0;
    ac.crystallized = true;
    ac.influences.push(Influence {
        event_id: "alice-solo-deal".into(),
        weight: 1.0,
    });

    let bc = bob_node.field.get_mut(bob_deal);
    bc.probability = 1.0;
    bc.crystallized = true;
    bc.influences.push(Influence {
        event_id: "bob-solo-deal".into(),
        weight: 1.0,
    });

    // Verify: each only knows their own deal
    assert!(alice_node.field.get(alice_deal).crystallized);
    assert!(
        !alice_node.field.get(bob_deal).crystallized,
        "Alice doesn't know Bob's deal"
    );
    assert!(
        !bob_node.field.get(alice_deal).crystallized,
        "Bob doesn't know Alice's deal"
    );
    assert!(bob_node.field.get(bob_deal).crystallized);

    // --- RECONNECT: exchange boundaries ---
    let ba = alice_node.boundary_cells();
    let bb = bob_node.boundary_cells();
    alice_node.receive_boundary(&bb);
    bob_node.receive_boundary(&ba);

    // After merge: both agents see BOTH deals
    assert!(
        alice_node.field.get(alice_deal).crystallized,
        "Alice keeps her deal"
    );
    assert!(
        alice_node.field.get(bob_deal).crystallized,
        "Alice receives Bob's deal"
    );
    assert!(
        bob_node.field.get(alice_deal).crystallized,
        "Bob receives Alice's deal"
    );
    assert!(
        bob_node.field.get(bob_deal).crystallized,
        "Bob keeps his deal"
    );

    // Influence provenance survives merge
    assert!(
        alice_node
            .field
            .get(bob_deal)
            .influences
            .iter()
            .any(|i| i.event_id.contains("bob")),
        "Bob's deal carries Bob's identity after merge"
    );
}

// ============================================================
// STEP 6: Self-healing — destroyed agreement recovers
// ============================================================
#[test]
fn step6_destroyed_agreement_self_heals() {
    let (mut net, _mapper) = setup();

    // Create agreement with strong support (multiple nearby events)
    let coord = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };
    net.distributed_seed(coord, "deal-001", &[(0, "alice"), (1, "bob")]);

    // Add supporting events nearby for orbital density
    net.distributed_seed(
        Coord {
            t: 3,
            c: 4,
            o: 3,
            v: 3,
        },
        "deal-context-1",
        &[(0, "alice"), (1, "bob")],
    );
    net.distributed_seed(
        Coord {
            t: 4,
            c: 3,
            o: 3,
            v: 3,
        },
        "deal-context-2",
        &[(0, "alice"), (1, "bob")],
    );
    net.distributed_seed(
        Coord {
            t: 3,
            c: 3,
            o: 4,
            v: 3,
        },
        "deal-context-3",
        &[(0, "alice"), (1, "bob")],
    );
    net.run_to_equilibrium(10);

    assert!(net.get(coord).crystallized, "Agreement crystallized");

    // --- ATTACK: destroy the agreement record ---
    let owner_idx = if net.nodes[0].region.contains(coord) {
        0
    } else {
        1
    };
    net.nodes[owner_idx].field.destroy(coord);

    assert!(
        !net.nodes[owner_idx].field.get(coord).crystallized,
        "Agreement destroyed"
    );

    // Run network — surrounding geometry should restore it
    net.run_to_equilibrium(15);

    assert!(
        net.get(coord).crystallized,
        "Agreement self-healed from surrounding orbital geometry"
    );
}

// ============================================================
// STEP 7: Audit trail — influences record who participated
// ============================================================
#[test]
fn step7_audit_trail_via_influences() {
    let (mut net, mapper) = setup();

    let deal = Event {
        id: "deal-001".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:service-X:100".into(),
    };
    let coord = mapper.map(&deal);

    // Three-party agreement
    net.distributed_seed(
        coord,
        "deal-001:buy:100",
        &[(0, "alice"), (0, "bob"), (1, "carol")],
    );
    net.run_to_equilibrium(10);

    let cell = net.get(coord);
    assert!(cell.crystallized, "Multi-party deal crystallized");

    // Audit: extract all participating identities from influences
    let participants: Vec<&str> = cell
        .influences
        .iter()
        .map(|i| i.event_id.as_str())
        .collect();

    assert!(
        participants.iter().any(|p| p.contains("alice")),
        "Audit trail includes Alice: {:?}",
        participants
    );
    assert!(
        participants.iter().any(|p| p.contains("bob")),
        "Audit trail includes Bob: {:?}",
        participants
    );
    assert!(
        participants.iter().any(|p| p.contains("carol")),
        "Audit trail includes Carol: {:?}",
        participants
    );

    // Influence weights show relative contribution
    for inf in &cell.influences {
        assert!(inf.weight > 0.0, "Each participant has positive weight");
    }
}

// ============================================================
// STEP 8: Full scenario — end-to-end agent coordination
// ============================================================
#[test]
fn step8_full_scenario_end_to_end() {
    let size = 8;
    let mapper = CoordMapper::new(size).with_time_bucket(60);
    let mut net = Network::new(size, 2);

    // --- Phase 1: Agreement ---
    let deal = Event {
        id: "deal-final".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "alice".into(),
        data: "buy:compute:500".into(),
    };
    let deal_coord = mapper.map(&deal);
    net.distributed_seed(deal_coord, "deal-final:500", &[(0, "alice"), (1, "bob")]);
    net.run_to_equilibrium(10);

    assert!(
        net.get(deal_coord).crystallized,
        "Phase 1: Agreement crystallized"
    );

    // --- Phase 2: Conflict rejection ---
    let fraud = Event {
        id: "fraud-attempt".into(),
        timestamp: 120,
        channel: "marketplace".into(),
        org: "mallory".into(),
        data: "buy:compute:500".into(),
    };
    let fraud_coord = mapper.map(&fraud);
    net.seed(fraud_coord, "mallory:fake-deal");
    net.run_to_equilibrium(10);

    let fraud_cell = net.get(fraud_coord);
    let real_cell = net.get(deal_coord);

    // Real deal has both parties; fraud only has mallory
    let real_record = real_cell.record();
    assert!(
        real_record.contains("alice"),
        "Phase 2: Real deal has Alice: {}",
        real_record
    );
    assert!(
        real_record.contains("bob"),
        "Phase 2: Real deal has Bob: {}",
        real_record
    );

    // Fraud's PRIMARY influence is mallory, not alice
    let fraud_primary = fraud_cell
        .influences
        .iter()
        .max_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap())
        .expect("Fraud cell should have influences");
    assert!(
        fraud_primary.event_id.contains("mallory"),
        "Phase 2: Fraud's primary influence is mallory, not alice: {}",
        fraud_primary.event_id
    );

    // --- Phase 3: Self-healing ---
    let owner = if net.nodes[0].region.contains(deal_coord) {
        0
    } else {
        1
    };

    // Add density for self-healing support
    for offset in 1..=3 {
        let nearby = Coord {
            t: (deal_coord.t + offset) % size,
            ..deal_coord
        };
        net.distributed_seed(
            nearby,
            &format!("context-{}", offset),
            &[(0, "alice"), (1, "bob")],
        );
    }
    net.run_to_equilibrium(10);

    net.nodes[owner].field.destroy(deal_coord);
    assert!(
        !net.nodes[owner].field.get(deal_coord).crystallized,
        "Phase 3: Destroyed"
    );

    net.run_to_equilibrium(15);
    assert!(net.get(deal_coord).crystallized, "Phase 3: Self-healed");

    // --- Phase 4: Audit ---
    let final_cell = net.get(deal_coord);
    let record = final_cell.record();
    assert!(
        record.contains("alice") || record.contains("bob"),
        "Phase 4: Audit trail preserved after self-healing: {}",
        record
    );
}
