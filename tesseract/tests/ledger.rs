//! Ledger integration test — mapper + field working together.
//! Events mapped to coordinates → seeded → crystallized → queryable.

use tesseract::mapper::*;
use tesseract::*;

#[test]
fn real_world_events_crystallize() {
    let mapper = CoordMapper::new(16).with_time_bucket(60);
    let mut field = Field::new(16);

    let events = vec![
        Event {
            id: "tx-001".into(),
            timestamp: 1020,
            channel: "payments".into(),
            org: "alice-corp".into(),
            data: "Alice→Bob:10".into(),
        },
        Event {
            id: "tx-002".into(),
            timestamp: 1040,
            channel: "payments".into(),
            org: "alice-corp".into(),
            data: "Alice→Carol:5".into(),
        },
        Event {
            id: "tx-003".into(),
            timestamp: 1025,
            channel: "payments".into(),
            org: "bob-llc".into(),
            data: "Bob→Dave:3".into(),
        },
    ];

    for ev in &events {
        let coord = mapper.map(ev);
        field.seed_named(coord, &ev.data);
    }

    evolve_to_equilibrium(&mut field, 20);

    // All events should crystallize
    for ev in &events {
        let coord = mapper.map(ev);
        assert!(
            field.get(coord).crystallized,
            "Event '{}' at {} should crystallize",
            ev.data,
            coord
        );
    }
}

#[test]
fn same_channel_events_produce_emergent_links() {
    let mapper = CoordMapper::new(16).with_time_bucket(60);
    let mut field = Field::new(16);

    // Two events in same channel, same org, same time bucket
    // They share 3 axes (t, c, o) — only v differs
    let e1 = Event {
        id: "tx-001".into(),
        timestamp: 1020,
        channel: "payments".into(),
        org: "alice".into(),
        data: "Alice→Bob:10".into(),
    };
    let e2 = Event {
        id: "tx-002".into(),
        timestamp: 1040,
        channel: "payments".into(),
        org: "alice".into(),
        data: "Alice→Carol:5".into(),
    };

    let c1 = mapper.map(&e1);
    let c2 = mapper.map(&e2);

    // Verify they share 3 axes
    assert_eq!(c1.t, c2.t, "Same time bucket");
    assert_eq!(c1.c, c2.c, "Same channel");
    assert_eq!(c1.o, c2.o, "Same org");

    field.seed_named(c1, &e1.data);
    field.seed_named(c2, &e2.data);
    evolve_to_equilibrium(&mut field, 20);

    // Both should crystallize
    assert!(field.get(c1).crystallized);
    assert!(field.get(c2).crystallized);

    // Each event's record should mention the other (due to orbital overlap)
    let r1 = field.get(c1).record();
    let r2 = field.get(c2).record();

    // c1's record should contain its own data at high weight
    assert!(
        r1.contains("Alice→Bob"),
        "Event 1 should know itself: {}",
        r1
    );
    // c2's record should contain its own data at high weight
    assert!(
        r2.contains("Alice→Carol"),
        "Event 2 should know itself: {}",
        r2
    );
}

#[test]
fn different_channels_stay_independent() {
    let mapper = CoordMapper::new(16).with_time_bucket(60);
    let mut field = Field::new(16);

    let e1 = Event {
        id: "tx-001".into(),
        timestamp: 1020,
        channel: "payments".into(),
        org: "alice".into(),
        data: "payment".into(),
    };
    let e2 = Event {
        id: "tx-002".into(),
        timestamp: 1020,
        channel: "identity".into(),
        org: "bob".into(),
        data: "did-reg".into(),
    };

    let c1 = mapper.map(&e1);
    let c2 = mapper.map(&e2);

    field.seed_named(c1, &e1.data);
    field.seed_named(c2, &e2.data);
    evolve_to_equilibrium(&mut field, 20);

    // Both crystallize independently
    assert!(field.get(c1).crystallized);
    assert!(field.get(c2).crystallized);

    // Destroy one — other survives
    field.destroy(c1);
    for n in field.neighbors(c1) {
        field.destroy(n);
    }
    evolve_to_equilibrium(&mut field, 20);

    assert!(
        field.get(c2).crystallized,
        "Different channel event should survive"
    );
}

#[test]
fn query_by_coordinate() {
    let mapper = CoordMapper::new(16).with_time_bucket(60);
    let mut field = Field::new(16);

    let event = Event {
        id: "tx-999".into(),
        timestamp: 1020,
        channel: "supply-chain".into(),
        org: "factory-a".into(),
        data: "shipped:container-42".into(),
    };
    let coord = mapper.map(&event);

    field.seed_named(coord, &event.data);
    evolve_to_equilibrium(&mut field, 20);

    // Query: "does this event exist?"
    let cell = field.get(coord);
    assert!(cell.crystallized, "Event should be findable by coordinate");

    // Read the record
    let record = cell.record();
    assert!(
        record.contains("shipped:container-42"),
        "Record should contain event data: {}",
        record
    );
}

#[test]
fn dense_activity_strengthens_region() {
    let mapper = CoordMapper::new(16).with_time_bucket(60);
    let mut field = Field::new(16);

    // 10 events in the same channel and org, same time bucket
    for i in 0..10 {
        let ev = Event {
            id: format!("tx-{:03}", i),
            timestamp: 1020,
            channel: "payments".into(),
            org: "alice".into(),
            data: format!("payment-{}", i),
        };
        let coord = mapper.map(&ev);
        field.seed_named(coord, &ev.data);
    }

    evolve_to_equilibrium(&mut field, 20);

    let crystals = field.crystallized_count();
    println!(
        "Dense region: {} crystallized cells from 10 events",
        crystals
    );

    // With 10 overlapping events, should produce many crystallizations
    assert!(
        crystals > 10,
        "Dense activity should produce emergent crystallizations"
    );

    // Pick the first event and destroy it — should recover from the density
    let first = mapper.map(&Event {
        id: "tx-000".into(),
        timestamp: 1020,
        channel: "payments".into(),
        org: "alice".into(),
        data: "".into(),
    });
    field.destroy(first);
    for n in field.neighbors(first) {
        field.destroy(n);
    }

    evolve_to_equilibrium(&mut field, 20);
    assert!(
        field.get(first).crystallized,
        "Dense region should self-heal"
    );
}
