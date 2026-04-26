//! Persistence E2E: what happens when you turn off and on again?
//! Two recovery paths:
//!   1. Local cache: replay events from disk
//!   2. Network recovery: neighbors rebuild your field

use std::fs;
use tesseract::node::*;
use tesseract::*;

#[test]
fn local_cache_survives_restart() {
    let tmp = std::env::temp_dir().join("tesseract_e2e_node.log");
    let path = tmp.to_str().unwrap().to_string();
    let _ = fs::remove_file(&path);

    let region = Region {
        start: [0, 0, 0, 0],
        end: [4, 4, 4, 4],
    };
    let coord = Coord {
        t: 1,
        c: 1,
        o: 1,
        v: 1,
    };

    // Session 1: seed events, they persist to disk
    {
        let mut node = Node::with_persistence("alice", 4, region.clone(), &path);
        node.seed(coord, "tx-001");
        node.seed(
            Coord {
                t: 2,
                c: 1,
                o: 1,
                v: 1,
            },
            "tx-002",
        );
        node.seed(
            Coord {
                t: 1,
                c: 2,
                o: 1,
                v: 1,
            },
            "tx-003",
        );
        evolve_to_equilibrium(&mut node.field, 10);

        assert!(node.field.get(coord).crystallized);
        assert_eq!(node.event_count(), 3);
    }
    // Node "dies" here — RAM gone

    // Session 2: new node, same file — recovers from cache
    {
        let node = Node::with_persistence("alice", 4, region.clone(), &path);
        assert_eq!(node.event_count(), 3);
        assert!(
            node.field.crystallized_count() > 0,
            "Field should be reconstructed from cached events"
        );
    }

    let _ = fs::remove_file(&path);
}

#[test]
fn network_recovery_without_local_cache() {
    // 2-node network. Both seed events. Equilibrium reached.
    let mut net = Network::new(8, 2);

    net.seed(
        Coord {
            t: 1,
            c: 3,
            o: 3,
            v: 3,
        },
        "ev-node0",
    );
    net.seed(
        Coord {
            t: 5,
            c: 3,
            o: 3,
            v: 3,
        },
        "ev-node1",
    );
    net.run_to_equilibrium(10);

    let ev0 = Coord {
        t: 1,
        c: 3,
        o: 3,
        v: 3,
    };
    let ev1 = Coord {
        t: 5,
        c: 3,
        o: 3,
        v: 3,
    };
    assert!(net.get(ev0).crystallized);
    assert!(net.get(ev1).crystallized);

    // Node 0 dies completely — no cache, no disk, nothing
    let crystals_before = net.nodes[1].field.crystallized_count();

    net.simulate_node_recovery(0);

    // Node 1 should be unaffected
    assert!(
        net.nodes[1].field.get(ev1).crystallized,
        "Node 1 should be unaffected by node 0's death"
    );

    // Node 0 should have recovered some state from node 1's boundaries
    let recovered_cells = net.nodes[0].field.active_cells();
    assert!(
        recovered_cells > 0,
        "Recovered node should have cells from boundary exchange"
    );
}

#[test]
fn four_nodes_one_dies_recovers() {
    let mut net = Network::new(8, 4);

    // Each node seeds an event
    net.seed(
        Coord {
            t: 0,
            c: 3,
            o: 3,
            v: 3,
        },
        "ev-0",
    );
    net.seed(
        Coord {
            t: 2,
            c: 3,
            o: 3,
            v: 3,
        },
        "ev-1",
    );
    net.seed(
        Coord {
            t: 4,
            c: 3,
            o: 3,
            v: 3,
        },
        "ev-2",
    );
    net.seed(
        Coord {
            t: 6,
            c: 3,
            o: 3,
            v: 3,
        },
        "ev-3",
    );
    net.run_to_equilibrium(10);

    // All crystallized
    assert!(
        net.get(Coord {
            t: 0,
            c: 3,
            o: 3,
            v: 3
        })
        .crystallized
    );
    assert!(
        net.get(Coord {
            t: 6,
            c: 3,
            o: 3,
            v: 3
        })
        .crystallized
    );

    // Node 2 dies
    net.simulate_node_recovery(2);

    // Other nodes' events should survive
    assert!(
        net.get(Coord {
            t: 0,
            c: 3,
            o: 3,
            v: 3
        })
        .crystallized,
        "Node 0's event should survive node 2's death"
    );
    assert!(
        net.get(Coord {
            t: 6,
            c: 3,
            o: 3,
            v: 3
        })
        .crystallized,
        "Node 3's event should survive node 2's death"
    );

    // Recovered node should have received boundary data
    assert!(
        net.nodes[2].field.active_cells() > 0,
        "Recovered node should have data from neighbors"
    );
}
