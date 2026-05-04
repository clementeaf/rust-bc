//! End-to-end tests for the minimal testnet network layer.
//!
//! Tests:
//! 1. Two nodes synchronize a block
//! 2. Three nodes converge on the same chain
//! 3. Transaction propagation across nodes
//! 4. Compact block reconstruction from mempool
//! 5. Invalid block rejected by peers
//! 6. Wallet recovery works in a multi-node network

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rust_bc::account::address::address_from_pubkey;
use rust_bc::identity::signing::{SigningProvider, SoftwareSigningProvider};
use rust_bc::network::testnet::client;
use rust_bc::network::testnet::messages::NetworkMessage;
use rust_bc::network::testnet::node::{NodeConfig, NodeHandle};
use rust_bc::network::testnet::server;
use rust_bc::transaction::compact_block::SegWitBlock;
use rust_bc::transaction::native::NativeTransaction;
use rust_bc::transaction::segwit::{TxCore, TxWitness};

const CHAIN_ID: u64 = 9999;

/// Helper: create a signed tx from a specific signer (address matches).
fn make_tx_from_signer(
    signer: &SoftwareSigningProvider,
    to: &str,
    amount: u64,
    nonce: u64,
    fee: u64,
) -> (TxCore, TxWitness) {
    let pk = signer.public_key();
    let from = address_from_pubkey(&pk);
    let native =
        NativeTransaction::new_transfer_with_chain(&from, to, amount, nonce, fee, CHAIN_ID);
    let (core, mut witness) = native.to_segwit(pk);
    let payload = core.signing_payload();
    witness.signature = signer.sign(&payload).unwrap();
    (core, witness)
}

/// Helper: start a node with server in background, return the handle.
fn start_node(port: u16, peers: Vec<SocketAddr>, genesis: &[(&str, u64)]) -> Arc<NodeHandle> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let config = NodeConfig {
        addr,
        peers,
        proposer_address: String::new(),
    };
    let node = Arc::new(NodeHandle::new(config, genesis));

    let node_clone = Arc::clone(&node);
    tokio::spawn(async move {
        if let Err(e) = server::start_server(addr, node_clone).await {
            eprintln!("[testnet] server on {addr} failed: {e}");
        }
    });

    node
}

/// Wait for network operations to settle.
async fn wait() {
    tokio::time::sleep(Duration::from_millis(500)).await;
}

// ── Test 1: Two nodes synchronize a block ──────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_nodes_sync_block() {
    let signer = SoftwareSigningProvider::generate();
    let alice = address_from_pubkey(&signer.public_key());
    let bob = "b".repeat(40);

    let port_a = 19100;
    let port_b = 19101;
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();

    // Bidirectional peering
    let node_a = start_node(port_a, vec![addr_b], &[(&alice, 10_000)]);
    wait().await;
    let node_b = start_node(port_b, vec![addr_a], &[(&alice, 10_000)]);
    wait().await;

    // Submit tx to A, mine on A → broadcasts to B
    let (core, witness) = make_tx_from_signer(&signer, &bob, 500, 0, 1);
    node_a.submit_transaction(core, witness).await;
    wait().await;

    let block = node_a.mine_block().await;
    assert!(block.is_some(), "block must be mined");
    wait().await;

    assert_eq!(node_a.chain_height(), 1);
    assert_eq!(
        node_b.chain_height(),
        1,
        "node B should have synced 1 block"
    );
}

// ── Test 2: Three nodes converge ───────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn three_nodes_converge() {
    let signer = SoftwareSigningProvider::generate();
    let alice = address_from_pubkey(&signer.public_key());
    let recipient = "c".repeat(40);

    let port_a = 19200;
    let port_b = 19201;
    let port_c = 19202;
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();
    let addr_c: SocketAddr = format!("127.0.0.1:{port_c}").parse().unwrap();

    let genesis = vec![(alice.as_str(), 50_000u64)];

    // A knows B and C; B and C know A
    let node_a = start_node(port_a, vec![addr_b, addr_c], &genesis);
    wait().await;
    let node_b = start_node(port_b, vec![addr_a], &genesis);
    wait().await;
    let node_c = start_node(port_c, vec![addr_a], &genesis);
    wait().await;

    let (core, witness) = make_tx_from_signer(&signer, &recipient, 100, 0, 1);
    node_a.submit_transaction(core, witness).await;
    wait().await;

    node_a.mine_block().await;
    wait().await;

    assert_eq!(node_a.chain_height(), 1);
    assert_eq!(node_b.chain_height(), 1, "B should sync");
    assert_eq!(node_c.chain_height(), 1, "C should sync");

    // Balances consistent
    assert_eq!(
        node_b.get_balance(&alice),
        node_c.get_balance(&alice),
        "balance must be consistent across nodes"
    );
}

// ── Test 3: Transaction propagation ────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tx_propagation_to_peers() {
    let signer = SoftwareSigningProvider::generate();
    let alice = address_from_pubkey(&signer.public_key());

    let port_a = 19300;
    let port_b = 19301;
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();

    let genesis = vec![(alice.as_str(), 10_000u64)];

    // A knows B
    let node_a = start_node(port_a, vec![addr_b], &genesis);
    wait().await;
    let node_b = start_node(port_b, vec![addr_a], &genesis);
    wait().await;

    // Submit tx to A → broadcasts to B
    let (core, witness) = make_tx_from_signer(&signer, &"d".repeat(40), 200, 0, 1);
    node_a.submit_transaction(core, witness).await;
    wait().await;

    assert!(
        node_b.mempool_size() >= 1,
        "B should have received the tx in mempool"
    );
}

// ── Test 4: Compact block reconstruction ───────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compact_block_reconstruction() {
    let signer = SoftwareSigningProvider::generate();
    let alice = address_from_pubkey(&signer.public_key());
    let bob = "e".repeat(40);

    let port_a = 19400;
    let port_b = 19401;
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();

    let genesis = vec![(alice.as_str(), 10_000u64)];

    // Both know each other
    let node_a = start_node(port_a, vec![addr_b], &genesis);
    wait().await;
    let node_b = start_node(port_b, vec![addr_a], &genesis);
    wait().await;

    // Submit same tx to both (B has it in mempool for compact reconstruction)
    let (core, witness) = make_tx_from_signer(&signer, &bob, 300, 0, 1);
    node_a
        .submit_transaction(core.clone(), witness.clone())
        .await;
    node_b.submit_transaction(core, witness).await;
    wait().await;

    // Mine on A → broadcasts to B
    node_a.mine_block().await;
    wait().await;

    assert_eq!(node_a.chain_height(), 1);
    assert_eq!(
        node_b.chain_height(),
        1,
        "B should have block via propagation"
    );
}

// ── Test 5: Invalid block rejected ─────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invalid_block_rejected() {
    let port_a = 19500;
    let port_b = 19501;
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();

    let _node_a = start_node(port_a, vec![], &[("alice", 1_000)]);
    wait().await;
    let node_b = start_node(port_b, vec![addr_a], &[("alice", 1_000)]);
    wait().await;

    // Create an invalid block (wrong height — should be 1, not 999)
    use rust_bc::transaction::compact_block::CompactBlockHeader;
    let bad_block = SegWitBlock {
        header: CompactBlockHeader {
            height: 999,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            timestamp: 0,
            proposer: "attacker".to_string(),
        },
        tx_cores: vec![],
        witnesses: vec![],
        tx_root: [0u8; 32],
        witness_root: [0u8; 32],
    };

    // Push invalid block directly to B
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();
    let _ = client::send_to_peer(addr_b, &NetworkMessage::NewBlock(bad_block)).await;
    wait().await;

    assert_eq!(node_b.chain_height(), 0, "invalid block must be rejected");
}

// ── Test 6: Wallet recovery works in network ───────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wallet_recovery_works_in_network() {
    let signer = SoftwareSigningProvider::generate();
    let alice = address_from_pubkey(&signer.public_key());
    let bob = "f".repeat(40);

    let port_a = 19600;
    let port_b = 19601;
    let addr_a: SocketAddr = format!("127.0.0.1:{port_a}").parse().unwrap();
    let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();

    let genesis = vec![(alice.as_str(), 10_000u64)];

    // Bidirectional
    let node_a = start_node(port_a, vec![addr_b], &genesis);
    wait().await;
    let node_b = start_node(port_b, vec![addr_a], &genesis);
    wait().await;

    // Tx 1: alice → bob 500
    let (core1, witness1) = make_tx_from_signer(&signer, &bob, 500, 0, 1);
    node_a.submit_transaction(core1, witness1).await;
    wait().await;
    node_a.mine_block().await;
    wait().await;

    assert_eq!(node_a.get_balance(&alice), 10_000 - 500 - 1);
    assert_eq!(node_a.get_balance(&bob), 500);

    // Tx 2: alice → bob 200 (simulates "recovered wallet" signing again)
    let (core2, witness2) = make_tx_from_signer(&signer, &bob, 200, 1, 1);
    node_a.submit_transaction(core2, witness2).await;
    wait().await;
    node_a.mine_block().await;
    wait().await;

    let expected_alice = 10_000 - 500 - 1 - 200 - 1;
    let expected_bob = 500 + 200;

    assert_eq!(node_a.get_balance(&alice), expected_alice);
    assert_eq!(node_a.get_balance(&bob), expected_bob);

    assert_eq!(node_b.chain_height(), 2, "B should have 2 blocks");
    assert_eq!(
        node_b.get_balance(&alice),
        expected_alice,
        "B balance must match A"
    );
    assert_eq!(
        node_b.get_balance(&bob),
        expected_bob,
        "B balance must match A"
    );
}
