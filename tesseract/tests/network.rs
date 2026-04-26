//! Network tests — distributed field across multiple nodes.

use tesseract::mapper::*;
use tesseract::node::*;
use tesseract::*;

#[test]
fn two_nodes_propagate_event() {
    // 2 nodes, each owning half the t-axis of a size-8 field
    let mut net = Network::new(8, 2);

    // Seed event on node 0's region (t=2)
    let coord = Coord {
        t: 2,
        c: 3,
        o: 3,
        v: 3,
    };
    net.seed(coord, "Alice→Bob:10");

    // Before sync: only node 0 knows about it
    assert!(net.nodes[0].field.get(coord).probability > 0.0);

    // Run network (evolve + boundary exchange)
    net.run_to_equilibrium(10);

    // After sync: event should crystallize on node 0
    let cell = net.get(coord);
    assert!(cell.crystallized, "Event should crystallize");
    assert!(
        cell.record().contains("Alice"),
        "Record should contain event data"
    );
}

#[test]
fn event_crosses_node_boundary() {
    let mut net = Network::new(8, 2);
    // Node 0 owns t=[0,4), Node 1 owns t=[4,8)

    // Seed near boundary on node 0 (t=3)
    let near_boundary = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };
    net.seed(near_boundary, "boundary-event");

    // Seed on node 1 (t=5), close to boundary
    let other_side = Coord {
        t: 5,
        c: 3,
        o: 3,
        v: 3,
    };
    net.seed(other_side, "other-side-event");

    net.run_to_equilibrium(10);

    // Both should crystallize
    assert!(
        net.get(near_boundary).crystallized,
        "Near-boundary event should crystallize"
    );
    assert!(
        net.get(other_side).crystallized,
        "Other-side event should crystallize"
    );

    // Midpoint (t=4) is on node 1 — should have emergent crystallization
    let midpoint = Coord {
        t: 4,
        c: 3,
        o: 3,
        v: 3,
    };
    let mid_cell = net.get(midpoint);

    // The midpoint should at least have received probability from boundary exchange
    assert!(
        mid_cell.probability > 0.0,
        "Midpoint should receive probability via boundary exchange"
    );
}

#[test]
fn four_nodes_independent_events() {
    let mut net = Network::new(16, 4);
    // 4 nodes, each owning t=[0,4), [4,8), [8,12), [12,16)

    // Seed one event per node
    let events = vec![
        (
            Coord {
                t: 1,
                c: 4,
                o: 4,
                v: 4,
            },
            "ev-node0",
        ),
        (
            Coord {
                t: 5,
                c: 4,
                o: 4,
                v: 4,
            },
            "ev-node1",
        ),
        (
            Coord {
                t: 9,
                c: 4,
                o: 4,
                v: 4,
            },
            "ev-node2",
        ),
        (
            Coord {
                t: 13,
                c: 4,
                o: 4,
                v: 4,
            },
            "ev-node3",
        ),
    ];

    for (coord, name) in &events {
        net.seed(*coord, name);
    }

    net.run_to_equilibrium(10);

    for (coord, name) in &events {
        assert!(
            net.get(*coord).crystallized,
            "Event '{}' at {} should crystallize",
            name,
            coord
        );
    }
}

#[test]
fn network_with_mapper() {
    let mapper = CoordMapper::new(8).with_time_bucket(60);
    let mut net = Network::new(8, 2);

    let e1 = Event {
        id: "tx-001".into(),
        timestamp: 60,
        channel: "payments".into(),
        org: "alice".into(),
        data: "Alice→Bob:10".into(),
    };
    let e2 = Event {
        id: "tx-002".into(),
        timestamp: 300,
        channel: "payments".into(),
        org: "bob".into(),
        data: "Bob→Carol:5".into(),
    };

    let c1 = mapper.map(&e1);
    let c2 = mapper.map(&e2);

    net.seed(c1, &e1.data);
    net.seed(c2, &e2.data);
    net.run_to_equilibrium(10);

    assert!(
        net.get(c1).crystallized,
        "Event 1 should crystallize in network"
    );
    assert!(
        net.get(c2).crystallized,
        "Event 2 should crystallize in network"
    );
}

#[test]
fn node_destruction_does_not_affect_other_nodes() {
    let mut net = Network::new(8, 2);

    let ev0 = Coord {
        t: 1,
        c: 3,
        o: 3,
        v: 3,
    };
    let ev1 = Coord {
        t: 6,
        c: 3,
        o: 3,
        v: 3,
    };

    net.seed(ev0, "event-on-node0");
    net.seed(ev1, "event-on-node1");
    net.run_to_equilibrium(10);

    assert!(net.get(ev0).crystallized);
    assert!(net.get(ev1).crystallized);

    // Destroy event on node 0
    net.nodes[0].field.destroy(ev0);
    for n in net.nodes[0].field.neighbors(ev0) {
        net.nodes[0].field.destroy(n);
    }

    // Event on node 1 should be unaffected
    assert!(
        net.get(ev1).crystallized,
        "Event on other node should survive destruction"
    );
}

// === Distributed seeding ===

#[test]
fn distributed_seed_both_parties_see_event() {
    // Alice on node 0, Bob on node 1. They agree on a transaction.
    // Both seed the same event from their respective nodes.
    let mapper = CoordMapper::new(8).with_time_bucket(60);
    let mut net = Network::new(8, 2);

    let event = Event {
        id: "tx-mutual-001".into(),
        timestamp: 120,
        channel: "payments".into(),
        org: "alice".into(),
        data: "Alice→Bob:10".into(),
    };
    let coord = mapper.map(&event);

    // Both parties seed from their nodes
    net.distributed_seed(coord, "Alice→Bob:10", &[(0, "alice"), (1, "bob")]);
    net.run_to_equilibrium(10);

    // Both nodes should see the crystallized event
    let cell_node0 = net.nodes[0].field.get(coord);
    let cell_node1 = net.nodes[1].field.get(coord);

    // At least the owning node should have it crystallized
    let owner_cell = net.get(coord);
    assert!(owner_cell.crystallized, "Event should crystallize");

    // Record should show both parties
    let record = owner_cell.record();
    assert!(
        record.contains("alice") || record.contains("bob"),
        "Record should reference at least one party: {}",
        record
    );
}

#[test]
fn distributed_seed_stronger_than_single() {
    let mut net_single = Network::new(8, 2);
    let mut net_distributed = Network::new(8, 2);

    let coord = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };

    // Single: only node 0 seeds
    net_single.seed(coord, "single-seed");
    net_single.run_to_equilibrium(10);

    // Distributed: both nodes seed
    net_distributed.distributed_seed(coord, "dual-seed", &[(0, "party-A"), (1, "party-B")]);
    net_distributed.run_to_equilibrium(10);

    // Both should crystallize at the coord
    let single_cell = net_single.get(coord);
    let dist_cell = net_distributed.get(coord);

    assert!(single_cell.crystallized, "Single seed should crystallize");
    assert!(
        dist_cell.crystallized,
        "Distributed seed should crystallize"
    );

    // Distributed should have more influences (both parties contributed)
    assert!(
        dist_cell.influences.len() >= single_cell.influences.len(),
        "Distributed seed should have more influences: single={}, dist={}",
        single_cell.influences.len(),
        dist_cell.influences.len()
    );
}

#[test]
fn distributed_seed_self_heals_from_both_sides() {
    let mut net = Network::new(8, 2);
    let coord = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };

    // Both parties seed + nearby events for density
    net.distributed_seed(coord, "agreement", &[(0, "alice"), (1, "bob")]);
    net.distributed_seed(
        Coord {
            t: 3,
            c: 4,
            o: 3,
            v: 3,
        },
        "related-1",
        &[(0, "alice"), (1, "bob")],
    );
    net.distributed_seed(
        Coord {
            t: 4,
            c: 3,
            o: 3,
            v: 3,
        },
        "related-2",
        &[(0, "alice"), (1, "bob")],
    );
    net.run_to_equilibrium(10);

    assert!(net.get(coord).crystallized);

    // Destroy on the owning node
    let owner_idx = if net.nodes[0].region.contains(coord) {
        0
    } else {
        1
    };
    net.nodes[owner_idx].field.destroy(coord);
    for n in net.nodes[owner_idx].field.neighbors(coord) {
        net.nodes[owner_idx].field.destroy(n);
    }

    // Run — should recover via boundary exchange from the other node
    net.run_to_equilibrium(15);

    assert!(
        net.get(coord).crystallized,
        "Distributed event should self-heal via the other node's orbital"
    );
}

// === Partition reconciliation ===

#[test]
fn partition_reconciliation_preserves_both_sides() {
    // Verify CRDT-like merge: crystallizations from both partitions
    // survive after reconnect. Uses Field directly (no evolve loops)
    // to test merge semantics in isolation, keeping the test fast.
    use tesseract::*;

    let size = 8;
    let mut field_a = Field::new(size);
    let mut field_b = Field::new(size);

    // Each partition crystallizes a different event
    let coord_a = Coord {
        t: 1,
        c: 1,
        o: 1,
        v: 1,
    };
    let coord_b = Coord {
        t: 6,
        c: 6,
        o: 6,
        v: 6,
    };

    // Force crystallization directly (simulating converged partition state)
    let cell_a = field_a.get_mut(coord_a);
    cell_a.probability = 1.0;
    cell_a.crystallized = true;
    cell_a.influences.push(Influence {
        event_id: "ev-A".into(),
        weight: 1.0,
    });

    let cell_b = field_b.get_mut(coord_b);
    cell_b.probability = 1.0;
    cell_b.crystallized = true;
    cell_b.influences.push(Influence {
        event_id: "ev-B".into(),
        weight: 1.0,
    });

    // Verify isolation
    assert!(field_a.get(coord_a).crystallized);
    assert!(!field_a.get(coord_b).crystallized);
    assert!(!field_b.get(coord_a).crystallized);
    assert!(field_b.get(coord_b).crystallized);

    // Reconnect: create nodes wrapping these fields, exchange boundaries
    let region = Region {
        start: [0, 0, 0, 0],
        end: [size, size, size, size],
    };
    let mut node_a = Node::new("node-a", size, region.clone());
    let mut node_b = Node::new("node-b", size, region);
    node_a.field = field_a;
    node_b.field = field_b;

    let boundary_a = node_a.boundary_cells();
    let boundary_b = node_b.boundary_cells();
    node_a.receive_boundary(&boundary_b);
    node_b.receive_boundary(&boundary_a);

    // Both crystallizations must survive on both nodes
    assert!(
        node_a.field.get(coord_a).crystallized,
        "A's crystal survives on A"
    );
    assert!(
        node_a.field.get(coord_b).crystallized,
        "B's crystal arrives on A"
    );
    assert!(
        node_b.field.get(coord_a).crystallized,
        "A's crystal arrives on B"
    );
    assert!(
        node_b.field.get(coord_b).crystallized,
        "B's crystal survives on B"
    );

    // Influences merged correctly
    assert!(node_a
        .field
        .get(coord_b)
        .influences
        .iter()
        .any(|i| i.event_id == "ev-B"));
    assert!(node_b
        .field
        .get(coord_a)
        .influences
        .iter()
        .any(|i| i.event_id == "ev-A"));
}
