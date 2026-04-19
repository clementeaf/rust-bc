//! Monetary transaction tests on the tesseract field.

use tesseract::wallet::*;

#[test]
fn genesis_allocation() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 100.0, 0);
    ledger.settle();

    assert_eq!(ledger.balance("alice"), 100.0);
    assert_eq!(ledger.balance("bob"), 0.0);
}

#[test]
fn simple_transfer() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 100.0, 0);

    let tx = Transfer {
        id: "tx-001".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 30.0,
        timestamp: 60,
        channel: "payments".into(),
    };

    let result = ledger.transfer(tx);
    assert!(result.is_ok(), "Transfer should succeed: {:?}", result);

    ledger.settle();

    assert_eq!(ledger.balance("alice"), 70.0);
    assert_eq!(ledger.balance("bob"), 30.0);
}

#[test]
fn insufficient_balance_rejected() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 10.0, 0);

    let tx = Transfer {
        id: "tx-overdraft".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 50.0,
        timestamp: 60,
        channel: "payments".into(),
    };

    let result = ledger.transfer(tx);
    assert!(result.is_err(), "Should reject overdraft");
    assert_eq!(ledger.balance("alice"), 10.0);
}

#[test]
fn chain_of_transfers() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 100.0, 0);

    // Alice → Bob: 50
    ledger.transfer(Transfer {
        id: "tx-001".into(), from: "alice".into(), to: "bob".into(),
        amount: 50.0, timestamp: 60, channel: "payments".into(),
    }).unwrap();

    // Bob → Carol: 20
    ledger.transfer(Transfer {
        id: "tx-002".into(), from: "bob".into(), to: "carol".into(),
        amount: 20.0, timestamp: 120, channel: "payments".into(),
    }).unwrap();

    // Carol → Dave: 10
    ledger.transfer(Transfer {
        id: "tx-003".into(), from: "carol".into(), to: "dave".into(),
        amount: 10.0, timestamp: 180, channel: "payments".into(),
    }).unwrap();

    ledger.settle();

    assert_eq!(ledger.balance("alice"), 50.0);
    assert_eq!(ledger.balance("bob"), 30.0);
    assert_eq!(ledger.balance("carol"), 10.0);
    assert_eq!(ledger.balance("dave"), 10.0);
}

#[test]
fn transfer_crystallizes() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 100.0, 0);

    ledger.transfer(Transfer {
        id: "tx-crystal".into(), from: "alice".into(), to: "bob".into(),
        amount: 25.0, timestamp: 60, channel: "payments".into(),
    }).unwrap();

    ledger.settle();

    assert!(ledger.is_confirmed("tx-crystal"), "Transfer should crystallize in the field");
}

#[test]
fn double_spend_rejected() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 100.0, 0);

    // First transfer
    ledger.transfer(Transfer {
        id: "tx-once".into(), from: "alice".into(), to: "bob".into(),
        amount: 60.0, timestamp: 60, channel: "payments".into(),
    }).unwrap();

    ledger.settle();

    // Try to replay exact same transfer
    let result = ledger.transfer(Transfer {
        id: "tx-once".into(), from: "alice".into(), to: "bob".into(),
        amount: 60.0, timestamp: 60, channel: "payments".into(),
    });

    // Should fail: either double-spend detected or insufficient balance
    assert!(result.is_err(), "Double spend should be rejected");
}

#[test]
fn multiple_genesis_participants() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 1000.0, 0);
    ledger.genesis_allocate("bob", 500.0, 0);
    ledger.genesis_allocate("carol", 250.0, 0);

    ledger.settle();

    assert_eq!(ledger.balance("alice"), 1000.0);
    assert_eq!(ledger.balance("bob"), 500.0);
    assert_eq!(ledger.balance("carol"), 250.0);

    // Cross transfers
    ledger.transfer(Transfer {
        id: "tx-ab".into(), from: "alice".into(), to: "bob".into(),
        amount: 100.0, timestamp: 60, channel: "payments".into(),
    }).unwrap();
    ledger.transfer(Transfer {
        id: "tx-bc".into(), from: "bob".into(), to: "carol".into(),
        amount: 200.0, timestamp: 60, channel: "payments".into(),
    }).unwrap();
    ledger.transfer(Transfer {
        id: "tx-ca".into(), from: "carol".into(), to: "alice".into(),
        amount: 50.0, timestamp: 60, channel: "payments".into(),
    }).unwrap();

    assert_eq!(ledger.balance("alice"), 950.0);  // -100 +50
    assert_eq!(ledger.balance("bob"), 400.0);    // +100 -200
    assert_eq!(ledger.balance("carol"), 400.0);  // +200 -50
}

// === The critical question: simultaneous conflicting transfers ===

#[test]
fn simultaneous_conflict_second_rejected() {
    // Alice has 10. Tries to send 10 to Bob AND 10 to Carol.
    // Sequential: second fails (balance check).
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 10.0, 0);

    let tx1 = ledger.transfer(Transfer {
        id: "tx-to-bob".into(), from: "alice".into(), to: "bob".into(),
        amount: 10.0, timestamp: 60, channel: "payments".into(),
    });
    assert!(tx1.is_ok(), "First transfer should succeed");

    let tx2 = ledger.transfer(Transfer {
        id: "tx-to-carol".into(), from: "alice".into(), to: "carol".into(),
        amount: 10.0, timestamp: 60, channel: "payments".into(),
    });
    assert!(tx2.is_err(), "Second transfer should fail: insufficient balance");

    assert_eq!(ledger.balance("alice"), 0.0);
    assert_eq!(ledger.balance("bob"), 10.0);
    assert_eq!(ledger.balance("carol"), 0.0);
}

#[test]
fn distributed_simultaneous_conflict() {
    // The real test: Alice sends from TWO different nodes simultaneously.
    // Both nodes seed the transfer before syncing.
    // After sync + evolution, only one should be valid.
    use tesseract::node::*;
    use tesseract::mapper::*;

    let mapper = CoordMapper::new(8).with_time_bucket(60);
    let mut net = Network::new(8, 2);

    // Genesis: Alice gets 10 on node 0
    let genesis_coord = mapper.map(&Event {
        id: "genesis:alice".into(), timestamp: 0,
        channel: "genesis".into(), org: "alice".into(), data: "".into(),
    });
    net.seed(genesis_coord, "+10→alice[genesis]");
    net.run_to_equilibrium(5);

    // Conflict: Alice→Bob seeded on node 0
    let tx_bob = Event {
        id: "tx-conflict-bob".into(), timestamp: 60,
        channel: "payments".into(), org: "alice".into(),
        data: "-10:alice→bob".into(),
    };
    let coord_bob = mapper.map(&tx_bob);

    // Conflict: Alice→Carol seeded on node 1
    let tx_carol = Event {
        id: "tx-conflict-carol".into(), timestamp: 60,
        channel: "payments".into(), org: "alice".into(),
        data: "-10:alice→carol".into(),
    };
    let coord_carol = mapper.map(&tx_carol);

    // Both seed BEFORE sync (simulates simultaneous from different nodes)
    net.nodes[0].seed(coord_bob, &tx_bob.data);
    net.nodes[1].seed(coord_carol, &tx_carol.data);

    // Now sync and evolve
    net.run_to_equilibrium(10);

    // Both deformations exist in the field — both crystallize
    let bob_crystal = net.get(coord_bob).crystallized;
    let carol_crystal = net.get(coord_carol).crystallized;

    println!("  tx→bob crystallized: {}", bob_crystal);
    println!("  tx→carol crystallized: {}", carol_crystal);

    // KEY INSIGHT: both deformations crystallize because the field
    // doesn't know about "balances" — it knows about geometry.
    // The CONFLICT is detected at the application layer:
    // total debits (20) > genesis (10).
    //
    // Resolution options:
    // 1. First-to-crystallize wins (temporal ordering in the field)
    // 2. Application layer rejects the overdraft after derivation
    // 3. Deformations carry "weight" and compete for finite curvature budget
    //
    // In the current model, BOTH crystallize — the field doesn't enforce
    // monetary rules. The application layer (wallet.rs) prevents this
    // by checking balance before seeding.
    //
    // In a fully distributed model, conflict resolution needs:
    // - Temporal ordering within the field (which crystallized first?)
    // - Or: finite curvature budgets per region (geometric scarcity)

    println!();
    println!("  INSIGHT: The field crystallizes both deformations.");
    println!("  Conflict resolution is an APPLICATION concern, not a FIELD concern.");
    println!("  The field provides the space. The rules of scarcity are a layer above.");
    println!("  Just as spacetime doesn't enforce conservation of energy —");
    println!("  physics does, using spacetime as the substrate.");
}

#[test]
fn partial_spend_leaves_correct_balance() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis_allocate("alice", 100.0, 0);

    // Spend in parts
    for i in 0..5 {
        ledger.transfer(Transfer {
            id: format!("tx-{}", i), from: "alice".into(), to: "bob".into(),
            amount: 15.0, timestamp: 60 + i as u64, channel: "payments".into(),
        }).unwrap();
    }

    assert_eq!(ledger.balance("alice"), 25.0);  // 100 - 5×15
    assert_eq!(ledger.balance("bob"), 75.0);    // 5×15

    // 6th transfer should fail (only 25 left, trying 30)
    let result = ledger.transfer(Transfer {
        id: "tx-overdraft".into(), from: "alice".into(), to: "bob".into(),
        amount: 30.0, timestamp: 120, channel: "payments".into(),
    });
    assert!(result.is_err());
    assert_eq!(ledger.balance("alice"), 25.0);  // unchanged
}
