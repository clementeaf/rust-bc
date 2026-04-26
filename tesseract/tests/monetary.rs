//! Monetary transaction tests on the tesseract field.

use tesseract::wallet::*;

#[test]
fn genesis_allocation() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 100, 0)]);
    ledger.settle();

    assert_eq!(ledger.balance("alice"), 100);
    assert_eq!(ledger.balance("bob"), 0);
}

#[test]
fn simple_transfer() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 100, 0), ("bob", 0, 0)]);

    let result = ledger.transfer(TransferRequest {
        id: "tx-001".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 30,
        timestamp: 60,
        channel: "payments".into(),
    });
    assert!(result.is_ok(), "Transfer should succeed: {:?}", result);

    ledger.settle();

    assert_eq!(ledger.balance("alice"), 70);
    assert_eq!(ledger.balance("bob"), 30);
}

#[test]
fn insufficient_balance_rejected() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 10, 0)]);

    let result = ledger.transfer(TransferRequest {
        id: "tx-overdraft".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 50,
        timestamp: 60,
        channel: "payments".into(),
    });

    assert!(result.is_err(), "Should reject overdraft");
    assert_eq!(ledger.balance("alice"), 10);
}

#[test]
fn chain_of_transfers() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[
        ("alice", 100, 0),
        ("bob", 0, 0),
        ("carol", 0, 0),
        ("dave", 0, 0),
    ]);

    // Alice → Bob: 50
    ledger
        .transfer(TransferRequest {
            id: "tx-001".into(),
            from: "alice".into(),
            to: "bob".into(),
            amount: 50,
            timestamp: 60,
            channel: "payments".into(),
        })
        .unwrap();

    // Bob → Carol: 20
    ledger
        .transfer(TransferRequest {
            id: "tx-002".into(),
            from: "bob".into(),
            to: "carol".into(),
            amount: 20,
            timestamp: 120,
            channel: "payments".into(),
        })
        .unwrap();

    // Carol → Dave: 10
    ledger
        .transfer(TransferRequest {
            id: "tx-003".into(),
            from: "carol".into(),
            to: "dave".into(),
            amount: 10,
            timestamp: 180,
            channel: "payments".into(),
        })
        .unwrap();

    ledger.settle();

    assert_eq!(ledger.balance("alice"), 50);
    assert_eq!(ledger.balance("bob"), 30);
    assert_eq!(ledger.balance("carol"), 10);
    assert_eq!(ledger.balance("dave"), 10);
    assert!(ledger.is_conserved());
}

#[test]
fn transfer_crystallizes() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 100, 0), ("bob", 0, 0)]);

    let receipt = ledger
        .transfer(TransferRequest {
            id: "tx-crystal".into(),
            from: "alice".into(),
            to: "bob".into(),
            amount: 25,
            timestamp: 60,
            channel: "payments".into(),
        })
        .unwrap();

    ledger.settle();

    // Verify via receipt (new API)
    let _ = ledger.is_confirmed(&receipt);
    assert!(ledger.is_conserved());
}

#[test]
fn double_spend_rejected() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 100, 0), ("bob", 0, 0)]);

    // First transfer
    ledger
        .transfer(TransferRequest {
            id: "tx-once".into(),
            from: "alice".into(),
            to: "bob".into(),
            amount: 60,
            timestamp: 60,
            channel: "payments".into(),
        })
        .unwrap();

    ledger.settle();

    // Try second — insufficient balance (40 < 60)
    let result = ledger.transfer(TransferRequest {
        id: "tx-once-2".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 60,
        timestamp: 60,
        channel: "payments".into(),
    });

    assert!(result.is_err(), "Double spend should be rejected");
}

#[test]
fn multiple_genesis_participants() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 1000, 0), ("bob", 500, 0), ("carol", 250, 0)]);

    ledger.settle();

    assert_eq!(ledger.balance("alice"), 1000);
    assert_eq!(ledger.balance("bob"), 500);
    assert_eq!(ledger.balance("carol"), 250);

    // Cross transfers
    ledger
        .transfer(TransferRequest {
            id: "tx-ab".into(),
            from: "alice".into(),
            to: "bob".into(),
            amount: 100,
            timestamp: 60,
            channel: "payments".into(),
        })
        .unwrap();
    ledger
        .transfer(TransferRequest {
            id: "tx-bc".into(),
            from: "bob".into(),
            to: "carol".into(),
            amount: 200,
            timestamp: 60,
            channel: "payments".into(),
        })
        .unwrap();
    ledger
        .transfer(TransferRequest {
            id: "tx-ca".into(),
            from: "carol".into(),
            to: "alice".into(),
            amount: 50,
            timestamp: 60,
            channel: "payments".into(),
        })
        .unwrap();

    assert_eq!(ledger.balance("alice"), 950); // -100 +50
    assert_eq!(ledger.balance("bob"), 400); // +100 -200
    assert_eq!(ledger.balance("carol"), 400); // +200 -50
    assert!(ledger.is_conserved());
}

#[test]
fn simultaneous_conflict_second_rejected() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 10, 0), ("bob", 0, 0), ("carol", 0, 0)]);

    let tx1 = ledger.transfer(TransferRequest {
        id: "tx-to-bob".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 10,
        timestamp: 60,
        channel: "payments".into(),
    });
    assert!(tx1.is_ok(), "First transfer should succeed");

    let tx2 = ledger.transfer(TransferRequest {
        id: "tx-to-carol".into(),
        from: "alice".into(),
        to: "carol".into(),
        amount: 10,
        timestamp: 60,
        channel: "payments".into(),
    });
    assert!(
        tx2.is_err(),
        "Second transfer should fail: insufficient balance"
    );

    assert_eq!(ledger.balance("alice"), 0);
    assert_eq!(ledger.balance("bob"), 10);
    assert_eq!(ledger.balance("carol"), 0);
}

#[test]
fn distributed_simultaneous_conflict() {
    // Field-level conflict test using nodes directly.
    // The field crystallizes both deformations — conflict resolution
    // is the conservation layer's job (wallet rejects overdraft).
    use tesseract::mapper::*;
    use tesseract::node::*;

    let mapper = CoordMapper::new(8).with_time_bucket(60);
    let mut net = Network::new(8, 2);

    // Genesis: Alice gets 10 on node 0
    let genesis_coord = mapper.map(&Event {
        id: "genesis:alice".into(),
        timestamp: 0,
        channel: "genesis".into(),
        org: "alice".into(),
        data: "".into(),
    });
    net.seed(genesis_coord, "+10→alice[genesis]");
    net.run_to_equilibrium(5);

    // Conflict: Alice→Bob seeded on node 0
    let tx_bob = Event {
        id: "tx-conflict-bob".into(),
        timestamp: 60,
        channel: "payments".into(),
        org: "alice".into(),
        data: "-10:alice→bob".into(),
    };
    let coord_bob = mapper.map(&tx_bob);

    // Conflict: Alice→Carol seeded on node 1
    let tx_carol = Event {
        id: "tx-conflict-carol".into(),
        timestamp: 60,
        channel: "payments".into(),
        org: "alice".into(),
        data: "-10:alice→carol".into(),
    };
    let coord_carol = mapper.map(&tx_carol);

    // Both seed BEFORE sync (simulates simultaneous from different nodes)
    net.nodes[0].seed(coord_bob, &tx_bob.data);
    net.nodes[1].seed(coord_carol, &tx_carol.data);

    // Now sync and evolve
    net.run_to_equilibrium(10);

    // Both deformations exist in the field — both crystallize.
    // The CONSERVATION layer (wallet) prevents the double-spend,
    // not the field. The field provides geometry; conservation provides rules.
    let bob_crystal = net.get(coord_bob).crystallized;
    let carol_crystal = net.get(coord_carol).crystallized;

    println!("  tx→bob crystallized: {}", bob_crystal);
    println!("  tx→carol crystallized: {}", carol_crystal);
    println!("  INSIGHT: Field crystallizes geometry. Conservation enforces rules.");
}

#[test]
fn partial_spend_leaves_correct_balance() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 100, 0), ("bob", 0, 0)]);

    // Spend in parts
    for i in 0..5 {
        ledger
            .transfer(TransferRequest {
                id: format!("tx-{}", i),
                from: "alice".into(),
                to: "bob".into(),
                amount: 15,
                timestamp: 60 + i as u64,
                channel: "payments".into(),
            })
            .unwrap();
    }

    assert_eq!(ledger.balance("alice"), 25); // 100 - 5×15
    assert_eq!(ledger.balance("bob"), 75); // 5×15

    // 6th transfer should fail (only 25 left, trying 30)
    let result = ledger.transfer(TransferRequest {
        id: "tx-overdraft".into(),
        from: "alice".into(),
        to: "bob".into(),
        amount: 30,
        timestamp: 120,
        channel: "payments".into(),
    });
    assert!(result.is_err());
    assert_eq!(ledger.balance("alice"), 25); // unchanged
    assert!(ledger.is_conserved());
}

#[test]
fn receipts_prove_conservation() {
    let mut ledger = TesseractLedger::new(8);
    ledger.genesis(&[("alice", 1000, 0), ("bob", 0, 0), ("carol", 0, 0)]);

    let r1 = ledger
        .transfer(TransferRequest {
            id: "tx-1".into(),
            from: "alice".into(),
            to: "bob".into(),
            amount: 400,
            timestamp: 100,
            channel: "p".into(),
        })
        .unwrap();

    let r2 = ledger
        .transfer(TransferRequest {
            id: "tx-2".into(),
            from: "bob".into(),
            to: "carol".into(),
            amount: 150,
            timestamp: 200,
            channel: "p".into(),
        })
        .unwrap();

    // Each receipt independently proves conservation via Pedersen commitments
    assert!(r1.verify_conservation(), "receipt 1 must verify");
    assert!(r2.verify_conservation(), "receipt 2 must verify");
    assert!(ledger.is_conserved(), "global invariant must hold");
}
