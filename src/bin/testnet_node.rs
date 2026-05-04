//! Minimal testnet node CLI.
//!
//! Usage:
//!   cargo run --bin testnet_node -- node --port 3000
//!   cargo run --bin testnet_node -- node --port 3001 --peers 127.0.0.1:3000
//!   cargo run --bin testnet_node -- send-tx --from ADDR --to ADDR --amount 100 --fee 1 --node 127.0.0.1:3000
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
        /// Comma-separated peer addresses (e.g. 127.0.0.1:3000,127.0.0.1:3001).
        #[arg(long, default_value = "")]
        peers: String,
        /// Genesis balance for a test address (addr:amount, e.g. abc...def:10000).
        #[arg(long)]
        genesis: Vec<String>,
    },
    /// Send a transaction to a running node.
    SendTx {
        /// Sender address.
        #[arg(long)]
        from: String,
        /// Recipient address.
        #[arg(long)]
        to: String,
        /// Amount to send.
        #[arg(long)]
        amount: u64,
        /// Transaction fee.
        #[arg(long, default_value = "1")]
        fee: u64,
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
        /// Address to query.
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
        Commands::Node {
            port,
            peers,
            genesis,
        } => run_node(port, &peers, &genesis).await,
        Commands::SendTx {
            from,
            to,
            amount,
            fee,
            node,
        } => send_tx(&from, &to, amount, fee, &node).await,
        Commands::MineBlock { node } => mine_block(&node).await,
        Commands::ShowBalance { addr, node } => show_balance(&addr, &node).await,
    }
}

async fn run_node(port: u16, peers_csv: &str, genesis_args: &[String]) {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

    let peer_addrs: Vec<SocketAddr> = if peers_csv.is_empty() {
        vec![]
    } else {
        peers_csv
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    };

    // Parse genesis: "address:amount"
    let genesis_allocs: Vec<(String, u64)> = genesis_args
        .iter()
        .filter_map(|g| {
            let parts: Vec<&str> = g.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].parse().ok()?))
            } else {
                None
            }
        })
        .collect();

    let genesis_refs: Vec<(&str, u64)> = genesis_allocs
        .iter()
        .map(|(a, b)| (a.as_str(), *b))
        .collect();

    let config = NodeConfig {
        addr,
        peers: peer_addrs.clone(),
        proposer_address: String::new(),
    };

    let node = Arc::new(NodeHandle::new(config, &genesis_refs));

    eprintln!("[node] started on :{port}");
    for p in &peer_addrs {
        eprintln!("[peer] configured: {p}");
    }
    for (a, b) in &genesis_allocs {
        eprintln!("[genesis] {a} = {b}");
    }

    // Run server — blocks forever
    if let Err(e) = server::start_server(addr, node).await {
        eprintln!("[node] server error: {e}");
    }
}

async fn send_tx(from: &str, to: &str, amount: u64, fee: u64, node_addr: &str) {
    let addr: SocketAddr = node_addr.parse().expect("invalid node address");

    // Generate a stub signer for the tx (in real use, cerulean-wallet signs)
    let signer = SoftwareSigningProvider::generate();
    let pk = signer.public_key();
    let signer_addr = address_from_pubkey(&pk);

    // Use the signer's address if --from matches, otherwise warn
    let effective_from = if from == "auto" { &signer_addr } else { from };

    let native =
        NativeTransaction::new_transfer_with_chain(effective_from, to, amount, 0, fee, CHAIN_ID);
    let (core, mut witness) = native.to_segwit(pk);
    let payload = core.signing_payload();
    witness.signature = signer.sign(&payload).unwrap();

    let msg = NetworkMessage::NewTransaction(core, witness);
    match client::send_to_peer(addr, &msg).await {
        Ok(mut peer) => {
            let _ = tokio::io::AsyncWriteExt::shutdown(&mut peer.stream).await;
            eprintln!(
                "[tx] sent to {node_addr}: {effective_from} → {to} amount={amount} fee={fee}"
            );
        }
        Err(e) => eprintln!("[tx] failed to send: {e}"),
    }
}

async fn mine_block(node_addr: &str) {
    let addr: SocketAddr = node_addr.parse().expect("invalid node address");

    match client::send_to_peer(addr, &NetworkMessage::MineBlock).await {
        Ok(mut peer) => {
            // Wait for response
            match peer.recv().await {
                Ok(Some(NetworkMessage::MineBlockResponse { height, tx_count })) => {
                    eprintln!("[block] mined height={height} txs={tx_count}");
                }
                Ok(Some(other)) => eprintln!("[block] unexpected response: {other:?}"),
                Ok(None) => eprintln!("[block] no response (connection closed)"),
                Err(e) => eprintln!("[block] recv error: {e}"),
            }
        }
        Err(e) => eprintln!("[mine] failed to connect: {e}"),
    }
}

async fn show_balance(address: &str, node_addr: &str) {
    let addr: SocketAddr = node_addr.parse().expect("invalid node address");

    let msg = NetworkMessage::QueryBalance {
        address: address.to_string(),
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
            Ok(Some(other)) => eprintln!("[balance] unexpected response: {other:?}"),
            Ok(None) => eprintln!("[balance] no response"),
            Err(e) => eprintln!("[balance] recv error: {e}"),
        },
        Err(e) => eprintln!("[balance] failed to connect: {e}"),
    }
}
