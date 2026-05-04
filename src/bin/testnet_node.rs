//! Minimal testnet node CLI.
//!
//! Usage:
//!   cargo run --bin testnet_node -- node --port 3000
//!   cargo run --bin testnet_node -- node --port 3001 --peers 127.0.0.1:3000
//!   cargo run --bin testnet_node -- send-tx --to ADDR --amount 100 --node 127.0.0.1:3000
//!   cargo run --bin testnet_node -- mine-block --node 127.0.0.1:3000
//!   cargo run --bin testnet_node -- show-balance --addr ADDR --node 127.0.0.1:3000

use std::net::SocketAddr;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use rust_bc::account::address::address_from_pubkey;
use rust_bc::identity::signing::{SigningProvider, SoftwareSigningProvider};
use rust_bc::network::testnet::client;
use rust_bc::network::testnet::messages::NetworkMessage;
use rust_bc::network::testnet::node::{NodeConfig, NodeHandle};
use rust_bc::network::testnet::server;
use rust_bc::transaction::native::NativeTransaction;

const CHAIN_ID: u64 = 9999;
const GENESIS_BALANCE: u64 = 1_000_000;

/// Deterministic testnet key — DO NOT use in production.
/// All nodes and CLI commands share this key so the funded genesis address is consistent.
fn testnet_signer() -> SoftwareSigningProvider {
    let seed: [u8; 32] = [
        0xCE, 0x00, 0x1E, 0xA0, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A,
        0x1B, 0x1C,
    ];
    SoftwareSigningProvider::from_key(ed25519_dalek::SigningKey::from_bytes(&seed))
}

fn testnet_address() -> String {
    address_from_pubkey(&testnet_signer().public_key())
}

#[derive(Parser)]
#[command(name = "testnet_node", about = "Cerulean Ledger — minimal testnet")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a testnet node.
    Node {
        /// TCP port to listen on.
        #[arg(long)]
        port: u16,
        /// Comma-separated peer addresses.
        #[arg(long, default_value = "")]
        peers: String,
    },
    /// Send a transaction from the testnet genesis account.
    SendTx {
        /// Recipient address (40 hex chars).
        #[arg(long)]
        to: String,
        /// Amount to send.
        #[arg(long)]
        amount: u64,
        /// Transaction fee.
        #[arg(long, default_value = "1")]
        fee: u64,
        /// Sender nonce (auto=0 if omitted).
        #[arg(long, default_value = "0")]
        nonce: u64,
        /// Node address to send to.
        #[arg(long)]
        node: String,
    },
    /// Trigger block mining on a running node.
    MineBlock {
        /// Node address.
        #[arg(long)]
        node: String,
    },
    /// Query account balance on a running node.
    ShowBalance {
        /// Address to query. Use "genesis" for the testnet genesis address.
        #[arg(long)]
        addr: String,
        /// Node address.
        #[arg(long)]
        node: String,
    },
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let cli = Cli::parse();

    match cli.command {
        Commands::Node { port, peers } => run_node(port, &peers).await,
        Commands::SendTx {
            to,
            amount,
            fee,
            nonce,
            node,
        } => send_tx(&to, amount, fee, nonce, &node).await,
        Commands::MineBlock { node } => mine_block(&node).await,
        Commands::ShowBalance { addr, node } => show_balance(&addr, &node).await,
    }
}

async fn run_node(port: u16, peers_csv: &str) {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let genesis_addr = testnet_address();

    let peer_addrs: Vec<SocketAddr> = if peers_csv.is_empty() {
        vec![]
    } else {
        peers_csv
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    };

    let config = NodeConfig {
        addr,
        peers: peer_addrs.clone(),
        proposer_address: genesis_addr.clone(),
    };

    let node = Arc::new(NodeHandle::new(config, &[(&genesis_addr, GENESIS_BALANCE)]));

    eprintln!("[node] started on :{port}");
    eprintln!("[node] genesis address: {genesis_addr}");
    eprintln!("[node] genesis balance: {GENESIS_BALANCE} NOTA");
    for p in &peer_addrs {
        eprintln!("[peer] configured: {p}");
    }

    if let Err(e) = server::start_server(addr, node).await {
        eprintln!("[node] server error: {e}");
    }
}

async fn send_tx(to: &str, amount: u64, fee: u64, nonce: u64, node_addr: &str) {
    let addr: SocketAddr = node_addr.parse().expect("invalid node address");
    let signer = testnet_signer();
    let pk = signer.public_key();
    let from = address_from_pubkey(&pk);

    let native =
        NativeTransaction::new_transfer_with_chain(&from, to, amount, nonce, fee, CHAIN_ID);
    let (core, mut witness) = native.to_segwit(pk);
    let payload = core.signing_payload();
    witness.signature = signer.sign(&payload).unwrap();

    let msg = NetworkMessage::NewTransaction(core, witness);
    match client::send_to_peer(addr, &msg).await {
        Ok(mut peer) => {
            let _ = tokio::io::AsyncWriteExt::shutdown(&mut peer.stream).await;
            eprintln!("[tx] sent: {from} -> {to} amount={amount} fee={fee} nonce={nonce}");
        }
        Err(e) => eprintln!("[tx] failed: {e}"),
    }
}

async fn mine_block(node_addr: &str) {
    let addr: SocketAddr = node_addr.parse().expect("invalid node address");
    match client::send_to_peer(addr, &NetworkMessage::MineBlock).await {
        Ok(mut peer) => match peer.recv().await {
            Ok(Some(NetworkMessage::MineBlockResponse { height, tx_count })) => {
                eprintln!("[block] mined height={height} txs={tx_count}");
            }
            Ok(Some(other)) => eprintln!("[block] unexpected response: {other:?}"),
            Ok(None) => eprintln!("[block] no response (connection closed)"),
            Err(e) => eprintln!("[block] error: {e}"),
        },
        Err(e) => eprintln!("[mine] failed: {e}"),
    }
}

async fn show_balance(address: &str, node_addr: &str) {
    let addr: SocketAddr = node_addr.parse().expect("invalid node address");

    // Resolve "genesis" to the actual testnet genesis address
    let resolved = if address == "genesis" {
        testnet_address()
    } else {
        address.to_string()
    };

    let msg = NetworkMessage::QueryBalance {
        address: resolved.clone(),
    };
    match client::send_to_peer(addr, &msg).await {
        Ok(mut peer) => match peer.recv().await {
            Ok(Some(NetworkMessage::BalanceResponse {
                address,
                balance,
                nonce,
            })) => {
                eprintln!("[balance] {address}: {balance} NOTA (nonce={nonce})");
            }
            Ok(Some(other)) => eprintln!("[balance] unexpected: {other:?}"),
            Ok(None) => eprintln!("[balance] no response"),
            Err(e) => eprintln!("[balance] error: {e}"),
        },
        Err(e) => eprintln!("[balance] failed: {e}"),
    }
}
