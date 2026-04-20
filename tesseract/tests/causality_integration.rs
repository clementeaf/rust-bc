use tesseract::*;
use tesseract::causality::{CausalEvent, EventId};

fn coord(t: usize, c: usize, o: usize, v: usize) -> Coord {
    Coord { t, c, o, v }
}

#[test]
fn causal_field_limits_propagation_by_light_cone() {
    let mut field = Field::new(20).with_causality();

    // Genesis event at origin
    let origin = coord(5, 5, 5, 5);
    let eid = field.attest_causal(
        origin, b"genesis", vec![],
        Dimension::Temporal, "validator_t",
    ).unwrap();

    // At time 0 (just inserted, current_time=1), the light cone has radius ~1.
    // The center cell should have probability.
    assert!(field.get(origin).probability > 0.0, "origin should have probability");

    // A cell 3 units away should NOT have been reached yet
    // (cone radius = (1 - 0) * 1.0 = 1.0, distance to (8,5,5,5) = 3.0)
    let far = coord(8, 5, 5, 5);
    let far_p = field.get(far).probability;
    assert!(far_p < EPSILON, "far cell should be outside light cone, got p={far_p}");
}

#[test]
fn light_cone_expands_with_ticks() {
    let mut field = Field::new(20).with_causality();

    let origin = coord(5, 5, 5, 5);
    let eid = field.attest_causal(
        origin, b"event1", vec![],
        Dimension::Temporal, "val_t",
    ).unwrap();

    // Cell 1 unit away on t-axis — just outside initial cone
    let near = coord(6, 5, 5, 5);
    let p_before = field.get(near).probability;

    // Advance time so light cone expands past distance=1
    field.tick();
    field.tick();
    field.tick();

    // Child event at same origin, but now current_time=4, birth_time=3
    // so cone radius = (4-3)*1.0 = 1.0 which barely reaches distance 1.0
    // But more importantly, the FIRST event's cone (birth=0, now=4)
    // has radius=4 — we re-attest from the same origin to observe expansion.
    let _eid2 = field.attest_causal(
        origin, b"event2", vec![eid],
        Dimension::Context, "val_c",
    ).unwrap();

    let p_after = field.get(near).probability;
    assert!(p_after > p_before, "child event's cone should reach near cell, p_before={p_before}, p_after={p_after}");
}

#[test]
fn causal_violation_rejected() {
    let mut field = Field::new(20).with_causality();

    // Try to create an event with a fake parent
    let fake_parent = EventId::from_content(b"nonexistent");
    let result = field.attest_causal(
        coord(0, 0, 0, 0), b"orphan", vec![fake_parent],
        Dimension::Temporal, "val_t",
    );

    assert!(result.is_none(), "event with unknown parent should be rejected");
}

#[test]
fn concurrent_events_coexist_independently() {
    let mut field = Field::new(20).with_causality();

    // Genesis
    let g = field.attest_causal(
        coord(5, 5, 5, 5), b"genesis", vec![],
        Dimension::Temporal, "val_t",
    ).unwrap();

    field.tick();
    field.tick();

    // Two independent events from different origins, both children of genesis
    let a = field.attest_causal(
        coord(5, 5, 2, 5), b"branch_a", vec![g.clone()],
        Dimension::Origin, "val_o_a",
    ).unwrap();

    let b = field.attest_causal(
        coord(5, 5, 8, 5), b"branch_b", vec![g.clone()],
        Dimension::Origin, "val_o_b",
    ).unwrap();

    // Both events should exist in the causal graph
    let graph = field.causality.as_ref().unwrap();
    assert_eq!(
        graph.order(&a, &b),
        tesseract::causality::CausalOrder::Concurrent,
    );

    // Both regions should have probability
    assert!(field.get(coord(5, 5, 2, 5)).probability > 0.0);
    assert!(field.get(coord(5, 5, 8, 5)).probability > 0.0);
}

#[test]
fn classical_field_unaffected() {
    // Field WITHOUT causality should work exactly as before
    let mut field = Field::new(10);
    assert!(field.causality.is_none());

    let center = coord(5, 5, 5, 5);
    field.attest(center, "ev1", Dimension::Temporal, "val_t");
    assert!(field.get(center).probability > 0.0);

    // attest_causal returns None when causality is disabled
    let result = field.attest_causal(
        center, b"ev", vec![],
        Dimension::Temporal, "val_t",
    );
    assert!(result.is_none());
}

#[test]
fn causal_depth_tracks_chain_length() {
    let mut field = Field::new(20).with_causality();

    let e0 = field.attest_causal(
        coord(5, 5, 5, 5), b"e0", vec![],
        Dimension::Temporal, "val_t",
    ).unwrap();
    field.tick();

    let e1 = field.attest_causal(
        coord(6, 5, 5, 5), b"e1", vec![e0.clone()],
        Dimension::Context, "val_c",
    ).unwrap();
    field.tick();

    let e2 = field.attest_causal(
        coord(7, 5, 5, 5), b"e2", vec![e1.clone()],
        Dimension::Origin, "val_o",
    ).unwrap();

    let graph = field.causality.as_ref().unwrap();
    assert_eq!(graph.depth(&e0), 0);
    assert_eq!(graph.depth(&e1), 1);
    assert_eq!(graph.depth(&e2), 2);
}
