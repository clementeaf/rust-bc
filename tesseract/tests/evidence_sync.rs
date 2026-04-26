//! Tests for evidence sync: merkle roots, determinism, and delta protocol.
//!
//! Phase 1 of closing the 3 gaps in LIMITATIONS.md.

use tesseract::*;

// ── Evidence root determinism ────────────────────────────────────────────────

#[test]
fn evidence_root_is_zero_for_empty_cell() {
    let cell = Cell::new();
    assert_eq!(cell.evidence_root, [0u8; 32]);
    assert_eq!(cell.evidence_count, 0);
}

#[test]
fn evidence_root_nonzero_after_seed() {
    let mut field = Field::new(8);
    field.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "test-event",
    );

    let cell = field.get(Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    });
    assert_ne!(
        cell.evidence_root, [0u8; 32],
        "root should be non-zero after seed"
    );
    assert!(cell.evidence_count > 0, "count should be > 0 after seed");
}

#[test]
fn evidence_root_deterministic_same_inputs() {
    // Two fields seeded identically must produce identical roots.
    let mut f1 = Field::new(8);
    let mut f2 = Field::new(8);

    f1.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "event-A",
    );
    f2.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "event-A",
    );

    let c1 = f1.get(Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    });
    let c2 = f2.get(Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    });

    assert_eq!(
        c1.evidence_root, c2.evidence_root,
        "same inputs → same root"
    );
    assert_eq!(c1.evidence_count, c2.evidence_count);
}

#[test]
fn evidence_root_differs_for_different_events() {
    let mut f1 = Field::new(8);
    let mut f2 = Field::new(8);

    f1.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "alice-deal",
    );
    f2.seed_named(
        Coord {
            t: 3,
            c: 3,
            o: 3,
            v: 3,
        },
        "bob-counterclaim",
    );

    let c1 = f1.get(Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    });
    let c2 = f2.get(Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    });

    assert_ne!(
        c1.evidence_root, c2.evidence_root,
        "different events → different roots"
    );
}

#[test]
fn evidence_root_order_independent() {
    // Adding influences in different order should produce same root.
    let mut cell_a = Cell::new();
    cell_a.influences.push(Influence {
        event_id: "ev-1".into(),
        weight: 1.0,
    });
    cell_a.influences.push(Influence {
        event_id: "ev-2".into(),
        weight: 0.5,
    });
    cell_a.update_evidence();

    let mut cell_b = Cell::new();
    cell_b.influences.push(Influence {
        event_id: "ev-2".into(),
        weight: 0.5,
    });
    cell_b.influences.push(Influence {
        event_id: "ev-1".into(),
        weight: 1.0,
    });
    cell_b.update_evidence();

    assert_eq!(
        cell_a.evidence_root, cell_b.evidence_root,
        "order should not affect root"
    );
    assert_eq!(cell_a.evidence_count, cell_b.evidence_count);
}

// ── Attestation evidence ─────────────────────────────────────────────────────

#[test]
fn evidence_root_includes_attestations() {
    let mut field = Field::new(12);
    let center = Coord {
        t: 5,
        c: 5,
        o: 5,
        v: 5,
    };

    field.attest(center, "ev1", Dimension::Temporal, "val_t");

    let cell = field.get(center);
    assert!(
        cell.evidence_count > 0,
        "attestation should count as evidence"
    );
    assert_ne!(cell.evidence_root, [0u8; 32]);
}

#[test]
fn evidence_root_changes_with_additional_attestation() {
    let mut field = Field::new(12);
    let center = Coord {
        t: 5,
        c: 5,
        o: 5,
        v: 5,
    };

    field.attest(center, "ev1", Dimension::Temporal, "val_t");
    let root_1dim = field.get(center).evidence_root;

    field.attest(center, "ev1", Dimension::Context, "val_c");
    let root_2dim = field.get(center).evidence_root;

    assert_ne!(
        root_1dim, root_2dim,
        "adding attestation should change root"
    );
}

// ── Evidence root detects divergence ─────────────────────────────────────────

#[test]
fn divergent_nodes_have_different_evidence_roots() {
    // Simulates the split-brain scenario: two fields, same coord, different events.
    let mut f1 = Field::new(8);
    let mut f2 = Field::new(8);
    let coord = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };

    f1.seed_named(coord, "alice-version");
    f2.seed_named(coord, "bob-version");

    let root1 = f1.get(coord).evidence_root;
    let root2 = f2.get(coord).evidence_root;

    assert_ne!(
        root1, root2,
        "divergent nodes MUST have different evidence roots — this is how we detect split-brain"
    );
}

#[test]
fn converged_nodes_have_same_evidence_roots() {
    // Two fields with identical seeds must converge to same root.
    let mut f1 = Field::new(8);
    let mut f2 = Field::new(8);
    let coord = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };

    f1.seed_named(coord, "shared-event");
    f2.seed_named(coord, "shared-event");

    let root1 = f1.get(coord).evidence_root;
    let root2 = f2.get(coord).evidence_root;

    assert_eq!(
        root1, root2,
        "identical evidence sets must produce identical roots"
    );
}

// ── Evidence count ───────────────────────────────────────────────────────────

#[test]
fn evidence_count_tracks_total_items() {
    let mut cell = Cell::new();

    cell.influences.push(Influence {
        event_id: "a".into(),
        weight: 1.0,
    });
    cell.influences.push(Influence {
        event_id: "b".into(),
        weight: 0.5,
    });
    cell.update_evidence();
    assert_eq!(cell.evidence_count, 2);

    // Add an attestation
    cell.attestations
        .entry(Dimension::Temporal)
        .or_default()
        .push(Attestation {
            dimension: Dimension::Temporal,
            validator_id: "v1".into(),
            event_id: "a".into(),
            weight: 1.0,
        });
    cell.update_evidence();
    assert_eq!(cell.evidence_count, 3);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 2: Deterministic resolution rule
// ═══════════════════════════════════════════════════════════════════════════════

fn make_cell(p: f64, k: bool, event_id: &str) -> Cell {
    let mut cell = Cell::new();
    cell.probability = p;
    // Only allow crystallized if evidence supports it (p >= threshold).
    // This ensures cells are internally consistent with resolve() post-conditions.
    cell.crystallized = k && p >= CRYSTALLIZATION_THRESHOLD;
    if !event_id.is_empty() {
        cell.influences.push(Influence {
            event_id: event_id.into(),
            weight: p,
        });
    }
    cell.update_evidence();
    cell
}

// ── Idempotent ───────────────────────────────────────────────────────────────

#[test]
fn resolve_idempotent() {
    let a = make_cell(0.8, true, "ev-a");
    let result = resolve(&a, &a);
    assert_eq!(result.probability, a.probability);
    assert_eq!(result.crystallized, a.crystallized);
    assert_eq!(result.evidence_root, a.evidence_root);
}

// ── Commutative ──────────────────────────────────────────────────────────────

#[test]
fn resolve_commutative() {
    let a = make_cell(0.9, true, "alice");
    let b = make_cell(0.7, false, "bob");

    let ab = resolve(&a, &b);
    let ba = resolve(&b, &a);

    assert_eq!(ab.probability, ba.probability);
    assert_eq!(ab.crystallized, ba.crystallized);
    assert_eq!(ab.evidence_root, ba.evidence_root);
    assert_eq!(ab.evidence_count, ba.evidence_count);
}

#[test]
fn resolve_commutative_same_p_different_evidence() {
    let a = make_cell(0.8, false, "alice");
    let b = make_cell(0.8, false, "bob");

    let ab = resolve(&a, &b);
    let ba = resolve(&b, &a);

    assert_eq!(ab.evidence_root, ba.evidence_root);
}

// ── Associative ──────────────────────────────────────────────────────────────

#[test]
fn resolve_associative() {
    let a = make_cell(0.9, true, "alice");
    let b = make_cell(0.7, false, "bob");
    let c = make_cell(0.6, false, "carol");

    let ab_c = resolve(&resolve(&a, &b), &c);
    let a_bc = resolve(&a, &resolve(&b, &c));

    assert_eq!(ab_c.probability, a_bc.probability);
    assert_eq!(ab_c.crystallized, a_bc.crystallized);
    assert_eq!(ab_c.evidence_root, a_bc.evidence_root);
}

// ── Crystallized wins ────────────────────────────────────────────────────────

#[test]
fn resolve_crystallized_wins_over_higher_probability() {
    // Crystallized cell needs p >= threshold to be legitimately crystallized.
    let crystallized = make_cell(0.9, true, "solid");
    let high_p = make_cell(0.99, false, "fluid");

    let result = resolve(&crystallized, &high_p);
    assert!(result.crystallized);
}

// ── Higher probability wins (same k) ────────────────────────────────────────

#[test]
fn resolve_higher_probability_wins() {
    let strong = make_cell(0.9, false, "strong");
    let weak = make_cell(0.3, false, "weak");

    let result = resolve(&strong, &weak);
    assert_eq!(result.probability, 0.9);
}

// ── Evidence merge: union ────────────────────────────────────────────────────

#[test]
fn resolve_merges_influences_as_union() {
    let a = make_cell(0.8, true, "alice");
    let b = make_cell(0.6, false, "bob");

    let result = resolve(&a, &b);

    let ids: Vec<&str> = result
        .influences
        .iter()
        .map(|i| i.event_id.as_str())
        .collect();
    assert!(ids.contains(&"alice"), "should have alice: {:?}", ids);
    assert!(ids.contains(&"bob"), "should have bob: {:?}", ids);
}

#[test]
fn resolve_deduplicates_influences() {
    let a = make_cell(0.8, true, "shared-event");
    let b = make_cell(0.6, false, "shared-event");

    let result = resolve(&a, &b);

    let count = result
        .influences
        .iter()
        .filter(|i| i.event_id == "shared-event")
        .count();
    assert_eq!(count, 1, "duplicate influences should be deduplicated");
}

// ── Tiebreak is deterministic ────────────────────────────────────────────────

#[test]
fn resolve_tiebreak_by_evidence_root() {
    // Two cells with identical p, k, evidence_count but different roots.
    let mut a = Cell::new();
    a.probability = 0.5;
    a.influences.push(Influence {
        event_id: "aaa".into(),
        weight: 0.5,
    });
    a.update_evidence();

    let mut b = Cell::new();
    b.probability = 0.5;
    b.influences.push(Influence {
        event_id: "zzz".into(),
        weight: 0.5,
    });
    b.update_evidence();

    assert_ne!(a.evidence_root, b.evidence_root, "setup: roots must differ");

    let ab = resolve(&a, &b);
    let ba = resolve(&b, &a);

    // Must pick the same winner regardless of argument order.
    assert_eq!(ab.probability, ba.probability);
    assert_eq!(ab.evidence_root, ba.evidence_root);
}

// ── Resolve on real field divergence ─────────────────────────────────────────

#[test]
fn resolve_split_brain_produces_unified_state() {
    let mut f1 = Field::new(8);
    let mut f2 = Field::new(8);
    let coord = Coord {
        t: 3,
        c: 3,
        o: 3,
        v: 3,
    };

    f1.seed_named(coord, "alice-deal");
    f2.seed_named(coord, "bob-counterclaim");

    let cell_a = f1.get(coord);
    let cell_b = f2.get(coord);

    let merged = resolve(cell_a, cell_b);

    // Merged cell has evidence from BOTH sides.
    let ids: Vec<&str> = merged
        .influences
        .iter()
        .map(|i| i.event_id.as_str())
        .collect();
    assert!(
        ids.iter().any(|id| id.contains("alice")),
        "merged should have alice's evidence: {:?}",
        ids
    );
    assert!(
        ids.iter().any(|id| id.contains("bob")),
        "merged should have bob's evidence: {:?}",
        ids
    );

    // Evidence root is recomputed over the merged set.
    assert_ne!(merged.evidence_root, cell_a.evidence_root);
    assert_ne!(merged.evidence_root, cell_b.evidence_root);
    assert!(merged.evidence_count > cell_a.evidence_count);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 3: Distributed conservation — double-spend detection
// ═══════════════════════════════════════════════════════════════════════════════

use tesseract::conservation::*;

#[test]
fn spent_nonces_tracked_after_transfer() {
    let mut field = ConservedField::new();
    field.genesis(&[(
        Coord {
            t: 0,
            c: 0,
            o: 0,
            v: 0,
        },
        1000,
    )]);

    let tx = Transfer::new(
        vec![TransferInput {
            coord: Coord {
                t: 0,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 100,
            expected_nonce: 0,
        }],
        vec![TransferOutput {
            coord: Coord {
                t: 1,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 100,
        }],
    )
    .unwrap();

    field.apply(&tx).unwrap();

    // Check: same nonce + same hash → accept (idempotent)
    let result = field.check_remote_transfer(
        Coord {
            t: 0,
            c: 0,
            o: 0,
            v: 0,
        },
        0,
        tx.hash,
    );
    assert!(result.is_ok());
    assert!(result.unwrap()); // accepted
}

#[test]
fn double_spend_detected_different_hash() {
    let mut field = ConservedField::new();
    field.genesis(&[(
        Coord {
            t: 0,
            c: 0,
            o: 0,
            v: 0,
        },
        1000,
    )]);

    let tx = Transfer::new(
        vec![TransferInput {
            coord: Coord {
                t: 0,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 100,
            expected_nonce: 0,
        }],
        vec![TransferOutput {
            coord: Coord {
                t: 1,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 100,
        }],
    )
    .unwrap();

    field.apply(&tx).unwrap();

    // Remote sends a DIFFERENT tx claiming the same (coord, nonce=0)
    let fake_hash = [0xFFu8; 32]; // different hash
    let result = field.check_remote_transfer(
        Coord {
            t: 0,
            c: 0,
            o: 0,
            v: 0,
        },
        0,
        fake_hash,
    );

    assert!(result.is_err(), "should detect double-spend");
    match result.unwrap_err() {
        ConservationError::DoubleSpend {
            nonce, remote_wins, ..
        } => {
            assert_eq!(nonce, 0);
            // remote hash 0xFF... > local hash, so remote does NOT win
            assert!(!remote_wins, "higher hash should lose");
        }
        other => panic!("expected DoubleSpend, got {:?}", other),
    }
}

#[test]
fn double_spend_deterministic_resolution() {
    let mut field_a = ConservedField::new();
    let mut field_b = ConservedField::new();

    let src = Coord {
        t: 0,
        c: 0,
        o: 0,
        v: 0,
    };
    field_a.genesis(&[(src, 1000)]);
    field_b.genesis(&[(src, 1000)]);

    // Two different transfers from same source, same nonce
    let tx_a = Transfer::new(
        vec![TransferInput {
            coord: src,
            amount: 800,
            expected_nonce: 0,
        }],
        vec![TransferOutput {
            coord: Coord {
                t: 1,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 800,
        }],
    )
    .unwrap();

    let tx_b = Transfer::new(
        vec![TransferInput {
            coord: src,
            amount: 900,
            expected_nonce: 0,
        }],
        vec![TransferOutput {
            coord: Coord {
                t: 2,
                c: 0,
                o: 0,
                v: 0,
            },
            amount: 900,
        }],
    )
    .unwrap();

    field_a.apply(&tx_a).unwrap();
    field_b.apply(&tx_b).unwrap();

    // Cross-check: field_a checks field_b's transfer
    let result_a = field_a.check_remote_transfer(src, 0, tx_b.hash);
    assert!(result_a.is_err(), "should detect double-spend");

    // Cross-check: field_b checks field_a's transfer
    let result_b = field_b.check_remote_transfer(src, 0, tx_a.hash);
    assert!(result_b.is_err(), "should detect double-spend");

    // Deterministic: both sides agree on who wins (lower hash)
    let a_wins_from_a = match result_a.unwrap_err() {
        ConservationError::DoubleSpend { remote_wins, .. } => !remote_wins,
        _ => panic!("expected DoubleSpend"),
    };
    let a_wins_from_b = match result_b.unwrap_err() {
        ConservationError::DoubleSpend { remote_wins, .. } => remote_wins,
        _ => panic!("expected DoubleSpend"),
    };

    assert_eq!(
        a_wins_from_a, a_wins_from_b,
        "both sides must agree on winner: a_wins_from_a={}, a_wins_from_b={}",
        a_wins_from_a, a_wins_from_b
    );
}

#[test]
fn unknown_nonce_accepted() {
    let mut field = ConservedField::new();
    field.genesis(&[(
        Coord {
            t: 0,
            c: 0,
            o: 0,
            v: 0,
        },
        1000,
    )]);

    // No local transfer yet — remote's nonce 0 should be accepted
    let remote_hash = [0x42u8; 32];
    let result = field.check_remote_transfer(
        Coord {
            t: 0,
            c: 0,
            o: 0,
            v: 0,
        },
        0,
        remote_hash,
    );
    assert!(result.is_ok());
    assert!(result.unwrap());
}
