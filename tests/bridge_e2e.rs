//! End-to-end bridge lifecycle tests — full lock→proof→verify→mint→burn→release
//! flow with mock external chain.

use rust_bc::bridge::protocol::BridgeEngine;
use rust_bc::bridge::types::*;
use rust_bc::bridge::verifier;

fn eth_config() -> ChainConfig {
    ChainConfig {
        chain_id: ChainId("ethereum".into()),
        name: "Ethereum".into(),
        protocol: BridgeType::LightClient,
        active: true,
        min_confirmations: 12,
        max_transfer: 10_000_000,
    }
}

fn cosmos_config() -> ChainConfig {
    ChainConfig {
        chain_id: ChainId("cosmos".into()),
        name: "Cosmos Hub".into(),
        protocol: BridgeType::LightClient,
        active: true,
        min_confirmations: 6,
        max_transfer: 5_000_000,
    }
}

fn engine() -> BridgeEngine {
    let e = BridgeEngine::new();
    e.registry.register(eth_config());
    e.registry.register(cosmos_config());
    e
}

fn build_proof_for_payload(payload: &MessagePayload) -> InclusionProof {
    let msg_bytes = serde_json::to_vec(payload).unwrap();
    let (_, proofs) = verifier::build_merkle_tree(&[&msg_bytes]);
    proofs[0].clone()
}

// ── Outbound: rust-bc → Ethereum ────────────────────────────────────────────

#[test]
fn e2e_outbound_lock_and_release() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    // 1. Alice locks 1000 NOTA for transfer to Ethereum.
    let msg = e
        .initiate_transfer("alice", "0xBob", 1000, "NOTA", &eth, 100)
        .unwrap();
    assert_eq!(e.escrow.total_locked(), 1000);
    assert!(matches!(
        msg.payload,
        MessagePayload::TokenTransfer { amount: 1000, .. }
    ));

    // 2. (Off-chain) Ethereum mints wNOTA, user sends back, Ethereum burns.

    // 3. Release: proof verified, tokens unlocked.
    let entry = e.escrow.release(&msg.id, 200).unwrap();
    assert_eq!(entry.status, TransferStatus::Completed);
    assert_eq!(e.escrow.total_locked(), 0);
}

#[test]
fn e2e_outbound_refund_on_failure() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    let msg = e
        .initiate_transfer("alice", "0xBob", 500, "NOTA", &eth, 100)
        .unwrap();
    assert_eq!(e.escrow.total_locked(), 500);

    // Transfer failed — refund.
    let entry = e.escrow.refund(&msg.id, 200).unwrap();
    assert_eq!(entry.status, TransferStatus::Refunded);
    assert_eq!(e.escrow.total_locked(), 0);
}

// ── Inbound: Ethereum → rust-bc ─────────────────────────────────────────────

#[test]
fn e2e_inbound_verify_and_mint() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    // 1. Build an inbound message with valid proof.
    let payload = MessagePayload::TokenTransfer {
        sender: "0xAlice".into(),
        recipient: "bob".into(),
        amount: 2000,
        denom: "wETH".into(),
    };
    let proof = build_proof_for_payload(&payload);

    let message = BridgeMessage {
        id: [1u8; 32],
        source_chain: eth.clone(),
        dest_chain: ChainId::native(),
        sequence: 1,
        payload,
        source_height: 100,
        source_timestamp: 0,
        proof: Some(proof),
    };

    // 2. Process inbound (current_height = 100 + 12 confirmations).
    e.process_inbound(&message, 112).unwrap();

    // 3. Bob now has 2000 wETH on rust-bc.
    assert_eq!(e.escrow.wrapped_balance("bob", &eth, "wETH"), 2000);
    assert_eq!(e.escrow.wrapped_total_supply(&eth, "wETH"), 2000);
}

#[test]
fn e2e_inbound_then_burn_and_return() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    // Mint wETH via inbound.
    let payload = MessagePayload::TokenTransfer {
        sender: "0xAlice".into(),
        recipient: "bob".into(),
        amount: 3000,
        denom: "wETH".into(),
    };
    let proof = build_proof_for_payload(&payload);
    let message = BridgeMessage {
        id: [2u8; 32],
        source_chain: eth.clone(),
        dest_chain: ChainId::native(),
        sequence: 1,
        payload,
        source_height: 50,
        source_timestamp: 0,
        proof: Some(proof),
    };
    e.process_inbound(&message, 62).unwrap();
    assert_eq!(e.escrow.wrapped_balance("bob", &eth, "wETH"), 3000);

    // Bob burns wETH to return to Ethereum.
    e.escrow.burn("bob", 3000, &eth, "wETH").unwrap();
    assert_eq!(e.escrow.wrapped_balance("bob", &eth, "wETH"), 0);
    assert_eq!(e.escrow.wrapped_total_supply(&eth, "wETH"), 0);
}

// ── Multi-chain flows ───────────────────────────────────────────────────────

#[test]
fn e2e_multi_chain_independent_transfers() {
    let e = engine();
    let eth = ChainId("ethereum".into());
    let cosmos = ChainId("cosmos".into());

    // Outbound to Ethereum.
    let msg_eth = e
        .initiate_transfer("alice", "0xBob", 1000, "NOTA", &eth, 100)
        .unwrap();

    // Outbound to Cosmos.
    let msg_cosmos = e
        .initiate_transfer("alice", "cosmosCharlie", 500, "NOTA", &cosmos, 101)
        .unwrap();

    assert_eq!(e.escrow.total_locked(), 1500);

    // Release Ethereum transfer.
    e.escrow.release(&msg_eth.id, 200).unwrap();
    assert_eq!(e.escrow.total_locked(), 500);

    // Release Cosmos transfer.
    e.escrow.release(&msg_cosmos.id, 201).unwrap();
    assert_eq!(e.escrow.total_locked(), 0);
}

#[test]
fn e2e_inbound_from_multiple_chains() {
    let e = engine();
    let eth = ChainId("ethereum".into());
    let cosmos = ChainId("cosmos".into());

    // Inbound wETH from Ethereum.
    let payload1 = MessagePayload::TokenTransfer {
        sender: "0xAlice".into(),
        recipient: "bob".into(),
        amount: 1000,
        denom: "wETH".into(),
    };
    let proof1 = build_proof_for_payload(&payload1);
    e.process_inbound(
        &BridgeMessage {
            id: [10u8; 32],
            source_chain: eth.clone(),
            dest_chain: ChainId::native(),
            sequence: 1,
            payload: payload1,
            source_height: 100,
            source_timestamp: 0,
            proof: Some(proof1),
        },
        112,
    )
    .unwrap();

    // Inbound wATOM from Cosmos.
    let payload2 = MessagePayload::TokenTransfer {
        sender: "cosmos1Alice".into(),
        recipient: "bob".into(),
        amount: 500,
        denom: "wATOM".into(),
    };
    let proof2 = build_proof_for_payload(&payload2);
    e.process_inbound(
        &BridgeMessage {
            id: [11u8; 32],
            source_chain: cosmos.clone(),
            dest_chain: ChainId::native(),
            sequence: 1,
            payload: payload2,
            source_height: 50,
            source_timestamp: 0,
            proof: Some(proof2),
        },
        56,
    )
    .unwrap();

    // Bob has both wrapped tokens.
    assert_eq!(e.escrow.wrapped_balance("bob", &eth, "wETH"), 1000);
    assert_eq!(e.escrow.wrapped_balance("bob", &cosmos, "wATOM"), 500);

    // Independent supplies.
    assert_eq!(e.escrow.wrapped_total_supply(&eth, "wETH"), 1000);
    assert_eq!(e.escrow.wrapped_total_supply(&cosmos, "wATOM"), 500);
}

// ── Security: replay, invalid proof, confirmations ──────────────────────────

#[test]
fn e2e_replay_attack_blocked() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    let payload = MessagePayload::TokenTransfer {
        sender: "0xAlice".into(),
        recipient: "bob".into(),
        amount: 1000,
        denom: "wETH".into(),
    };
    let proof = build_proof_for_payload(&payload);
    let message = BridgeMessage {
        id: [20u8; 32],
        source_chain: eth,
        dest_chain: ChainId::native(),
        sequence: 1,
        payload,
        source_height: 100,
        source_timestamp: 0,
        proof: Some(proof),
    };

    // First: succeeds.
    e.process_inbound(&message, 112).unwrap();
    // Replay: blocked.
    let err = e.process_inbound(&message, 113).unwrap_err();
    assert!(format!("{err}").contains("already processed"));
}

#[test]
fn e2e_insufficient_confirmations_blocked() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    let payload = MessagePayload::TokenTransfer {
        sender: "0xAlice".into(),
        recipient: "bob".into(),
        amount: 500,
        denom: "wETH".into(),
    };
    let proof = build_proof_for_payload(&payload);
    let message = BridgeMessage {
        id: [30u8; 32],
        source_chain: eth,
        dest_chain: ChainId::native(),
        sequence: 1,
        payload,
        source_height: 100,
        source_timestamp: 0,
        proof: Some(proof),
    };

    // Only 5 confirmations (need 12).
    let err = e.process_inbound(&message, 105).unwrap_err();
    assert!(format!("{err}").contains("confirmation"));
}

#[test]
fn e2e_invalid_proof_blocked() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    let payload = MessagePayload::TokenTransfer {
        sender: "0xAlice".into(),
        recipient: "bob".into(),
        amount: 500,
        denom: "wETH".into(),
    };
    // Proof for different data.
    let (_, wrong_proofs) = verifier::build_merkle_tree(&[b"completely wrong data"]);

    let message = BridgeMessage {
        id: [40u8; 32],
        source_chain: eth,
        dest_chain: ChainId::native(),
        sequence: 1,
        payload,
        source_height: 100,
        source_timestamp: 0,
        proof: Some(wrong_proofs[0].clone()),
    };

    let err = e.process_inbound(&message, 200).unwrap_err();
    assert!(format!("{err}").contains("proof"));
}

// ── Stress: 100 sequential transfers ────────────────────────────────────────

#[test]
fn e2e_stress_100_outbound_transfers() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    let mut msg_ids = Vec::new();
    for i in 0..100u64 {
        let msg = e
            .initiate_transfer(
                &format!("sender_{i}"),
                &format!("0xRecv{i}"),
                100 + i,
                "NOTA",
                &eth,
                i,
            )
            .unwrap();
        msg_ids.push(msg.id);
    }

    let expected_total: u64 = (0..100).map(|i| 100 + i).sum();
    assert_eq!(e.escrow.total_locked(), expected_total);

    // Release all.
    for (i, id) in msg_ids.iter().enumerate() {
        e.escrow.release(id, 1000 + i as u64).unwrap();
    }
    assert_eq!(e.escrow.total_locked(), 0);
}

#[test]
fn e2e_stress_100_inbound_transfers() {
    let e = engine();
    let eth = ChainId("ethereum".into());

    for i in 0..100u64 {
        let payload = MessagePayload::TokenTransfer {
            sender: format!("0xSender{i}"),
            recipient: format!("recv_{i}"),
            amount: 100 + i,
            denom: "wETH".into(),
        };
        let proof = build_proof_for_payload(&payload);
        let mut id = [0u8; 32];
        id[..8].copy_from_slice(&i.to_le_bytes());

        e.process_inbound(
            &BridgeMessage {
                id,
                source_chain: eth.clone(),
                dest_chain: ChainId::native(),
                sequence: i + 1,
                payload,
                source_height: i,
                source_timestamp: 0,
                proof: Some(proof),
            },
            i + 12,
        )
        .unwrap();
    }

    let expected_total: u64 = (0..100).map(|i| 100 + i).sum();
    assert_eq!(e.escrow.wrapped_total_supply(&eth, "wETH"), expected_total);
}
