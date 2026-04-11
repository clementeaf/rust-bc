//! bcctl — operator CLI for rust-bc blockchain network.
//!
//! Replaces `scripts/bcctl.sh` with a compiled binary.
//! Usage: `bcctl [--node node1] [--format json] <command>`

use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

/// Operator CLI for rust-bc blockchain network.
#[derive(Parser)]
#[command(name = "bcctl", version, about)]
struct Cli {
    /// Target node name (node1, node2, node3, orderer1).
    #[arg(long, default_value = "node1", global = true)]
    node: String,

    /// Output format: "table" (default) or "json".
    #[arg(long, default_value = "table", global = true)]
    format: String,

    /// Skip TLS certificate verification.
    #[arg(long, default_value_t = true, global = true)]
    insecure: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show all nodes health and peer count.
    Status,
    /// Show P2P connectivity (peer list per node).
    Peers,
    /// Show latest block info.
    Blocks,
    /// Mine a block.
    Mine {
        /// Wallet address for coinbase reward.
        #[arg(default_value = "cli-miner")]
        address: String,
    },
    /// Create a new wallet.
    WalletCreate,
    /// List channels.
    Channels,
    /// Create a new channel.
    ChannelCreate {
        /// Channel ID to create.
        channel_id: String,
    },
    /// List organizations.
    Orgs,
    /// Show Prometheus metrics.
    Metrics,
    /// Verify chain integrity.
    Verify,
    /// Compare chain state across all peers.
    Consistency,
    /// Show network configuration.
    Env,
    /// Show node logs (requires docker).
    Logs {
        /// Node name.
        #[arg(default_value = "node1")]
        target: String,
        /// Number of log lines.
        #[arg(default_value = "50")]
        lines: u32,
    },
    /// Restart a node (requires docker).
    Restart {
        /// Node name (or "all").
        #[arg(default_value = "all")]
        target: String,
    },
}

fn port_for(node: &str) -> u16 {
    match node {
        "node1" => 8080,
        "node2" => 8082,
        "node3" => 8084,
        "orderer1" => 8086,
        _ => 8080,
    }
}

fn base_url(node: &str) -> String {
    format!("https://localhost:{}/api/v1", port_for(node))
}

const ALL_NODES: &[&str] = &["node1", "node2", "node3", "orderer1"];
const PEERS: &[&str] = &["node1", "node2", "node3"];

fn build_client(insecure: bool) -> Client {
    Client::builder()
        .danger_accept_invalid_certs(insecure)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client")
}

async fn api_get(client: &Client, node: &str, path: &str) -> Result<Value, String> {
    let url = format!("{}/{}", base_url(node), path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("{node}: {e}"))?;
    resp.json::<Value>()
        .await
        .map_err(|e| format!("{node}: {e}"))
}

async fn api_post(client: &Client, node: &str, path: &str, body: &Value) -> Result<Value, String> {
    let url = format!("{}/{}", base_url(node), path);
    let resp = client
        .post(&url)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("{node}: {e}"))?;
    resp.json::<Value>()
        .await
        .map_err(|e| format!("{node}: {e}"))
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

// ── Commands ─────────────────────────────────────────────────────────────────

async fn cmd_status(client: &Client, json: bool) {
    if !json {
        println!(
            "{:<12} {:<10} {:<8} {:<8} {:<20}",
            "NODE", "STATUS", "BLOCKS", "PEERS", "LATEST HASH"
        );
        println!("{}", "-".repeat(60));
    }

    let mut results = Vec::new();
    for &node in ALL_NODES {
        let health = api_get(client, node, "health")
            .await
            .ok()
            .and_then(|v| v["data"]["status"].as_str().map(String::from))
            .unwrap_or_else(|| "down".to_string());

        let stats = api_get(client, node, "stats").await.ok();
        let blocks = stats
            .as_ref()
            .and_then(|v| v["data"]["blockchain"]["block_count"].as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let peers = stats
            .as_ref()
            .and_then(|v| v["data"]["network"]["connected_peers"].as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let hash = stats
            .as_ref()
            .and_then(|v| v["data"]["blockchain"]["latest_block_hash"].as_str())
            .unwrap_or("-");
        let short_hash = if hash.len() > 16 {
            format!("{}...", &hash[..16])
        } else {
            hash.to_string()
        };

        if json {
            results.push(serde_json::json!({
                "node": node, "status": health, "blocks": blocks, "peers": peers, "latest_hash": hash
            }));
        } else {
            let status_display = if health == "healthy" {
                format!("\x1b[32m{health}\x1b[0m")
            } else {
                format!("\x1b[31m{health}\x1b[0m")
            };
            println!(
                "{:<12} {:<20} {:<8} {:<8} {:<20}",
                node, status_display, blocks, peers, short_hash
            );
        }
    }

    if json {
        print_json(&Value::Array(results));
    }
}

async fn cmd_peers(client: &Client, node: &str, json: bool) {
    match api_get(client, node, "stats").await {
        Ok(resp) => {
            if json {
                print_json(&resp["data"]["network"]);
            } else {
                let peers = resp["data"]["network"]["connected_peers"]
                    .as_u64()
                    .unwrap_or(0);
                println!("{node}: {peers} connected peers");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_blocks(client: &Client, node: &str, json: bool) {
    match api_get(client, node, "stats").await {
        Ok(resp) => {
            if json {
                print_json(&resp["data"]["blockchain"]);
            } else {
                let count = resp["data"]["blockchain"]["block_count"]
                    .as_u64()
                    .unwrap_or(0);
                let hash = resp["data"]["blockchain"]["latest_block_hash"]
                    .as_str()
                    .unwrap_or("-");
                println!("Blocks: {count}");
                println!("Latest hash: {hash}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_mine(client: &Client, node: &str, address: &str, json: bool) {
    // Create wallet first
    let wallet_body = serde_json::json!({"name": address});
    let _ = api_post(client, node, "wallets/create", &wallet_body).await;

    let mine_body = serde_json::json!({
        "miner_address": address,
        "reward": 50
    });
    match api_post(client, node, "blocks", &mine_body).await {
        Ok(resp) => {
            if json {
                print_json(&resp);
            } else {
                let idx = resp["data"]["block"]["index"]
                    .as_u64()
                    .or_else(|| resp["data"]["index"].as_u64())
                    .unwrap_or(0);
                println!("Block mined: #{idx}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_wallet_create(client: &Client, node: &str, json: bool) {
    let body = serde_json::json!({"name": format!("wallet-{}", chrono::Utc::now().timestamp())});
    match api_post(client, node, "wallets/create", &body).await {
        Ok(resp) => {
            if json {
                print_json(&resp);
            } else {
                let addr = resp["data"]["address"].as_str().unwrap_or("-");
                println!("Wallet created: {addr}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_channels(client: &Client, node: &str, json: bool) {
    match api_get(client, node, "channels").await {
        Ok(resp) => {
            if json {
                print_json(&resp["data"]);
            } else if let Some(arr) = resp["data"].as_array() {
                for ch in arr {
                    println!("  {}", ch.as_str().unwrap_or("-"));
                }
                println!("{} channels", arr.len());
            } else {
                println!("No channels");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_channel_create(client: &Client, node: &str, channel_id: &str, json: bool) {
    let body = serde_json::json!({"channel_id": channel_id});
    match api_post(client, node, "channels", &body).await {
        Ok(resp) => {
            if json {
                print_json(&resp);
            } else {
                println!("Channel '{channel_id}' created");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_orgs(client: &Client, node: &str, json: bool) {
    match api_get(client, node, "store/organizations").await {
        Ok(resp) => {
            if json {
                print_json(&resp["data"]);
            } else if let Some(arr) = resp["data"].as_array() {
                for org in arr {
                    let id = org["org_id"].as_str().unwrap_or("-");
                    let msp = org["msp_id"].as_str().unwrap_or("-");
                    println!("  {id} ({msp})");
                }
                println!("{} organizations", arr.len());
            } else {
                println!("No organizations");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_metrics(client: &Client, node: &str, json: bool) {
    match api_get(client, node, "metrics").await {
        Ok(resp) => {
            if json {
                print_json(&resp["data"]);
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&resp["data"]).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_verify(client: &Client, node: &str, json: bool) {
    match api_get(client, node, "chain/verify").await {
        Ok(resp) => {
            if json {
                print_json(&resp);
            } else {
                let valid = resp["data"]["chain_valid"]
                    .as_bool()
                    .or_else(|| resp["data"]["valid"].as_bool())
                    .unwrap_or(false);
                println!("Chain valid: {valid}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

async fn cmd_consistency(client: &Client, json: bool) {
    let mut heights: HashMap<String, u64> = HashMap::new();
    let mut hashes: HashMap<String, String> = HashMap::new();

    for &node in PEERS {
        if let Ok(resp) = api_get(client, node, "stats").await {
            if let Some(h) = resp["data"]["blockchain"]["block_count"].as_u64() {
                heights.insert(node.to_string(), h);
            }
            if let Some(hash) = resp["data"]["blockchain"]["latest_block_hash"].as_str() {
                hashes.insert(node.to_string(), hash.to_string());
            }
        }
    }

    if json {
        print_json(&serde_json::json!({"heights": heights, "hashes": hashes}));
        return;
    }

    let all_same_height = heights
        .values()
        .collect::<std::collections::HashSet<_>>()
        .len()
        <= 1;
    let all_same_hash = hashes
        .values()
        .collect::<std::collections::HashSet<_>>()
        .len()
        <= 1;

    for (node, h) in &heights {
        let hash = hashes.get(node).map(|s| s.as_str()).unwrap_or("-");
        println!("  {node}: height={h}, hash={}", &hash[..hash.len().min(16)]);
    }

    if all_same_height && all_same_hash {
        println!("\x1b[32mConsistent\x1b[0m");
    } else {
        println!("\x1b[31mInconsistent!\x1b[0m");
    }
}

async fn cmd_env() {
    println!("Network configuration:");
    for &node in ALL_NODES {
        println!(
            "  {node}: https://localhost:{} (P2P: {})",
            port_for(node),
            port_for(node) + 1
        );
    }
}

fn cmd_logs(target: &str, lines: u32) {
    let status = std::process::Command::new("docker")
        .args(["compose", "logs", "--tail", &lines.to_string(), target])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => eprintln!("Error: {e}"),
    }
}

fn cmd_restart(target: &str) {
    let nodes: Vec<&str> = if target == "all" {
        ALL_NODES.to_vec()
    } else {
        vec![target]
    };
    for node in nodes {
        println!("Restarting {node}...");
        let _ = std::process::Command::new("docker")
            .args(["compose", "restart", node])
            .status();
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = build_client(cli.insecure);
    let json = cli.format == "json";

    match cli.command {
        Commands::Status => cmd_status(&client, json).await,
        Commands::Peers => cmd_peers(&client, &cli.node, json).await,
        Commands::Blocks => cmd_blocks(&client, &cli.node, json).await,
        Commands::Mine { address } => cmd_mine(&client, &cli.node, &address, json).await,
        Commands::WalletCreate => cmd_wallet_create(&client, &cli.node, json).await,
        Commands::Channels => cmd_channels(&client, &cli.node, json).await,
        Commands::ChannelCreate { channel_id } => {
            cmd_channel_create(&client, &cli.node, &channel_id, json).await
        }
        Commands::Orgs => cmd_orgs(&client, &cli.node, json).await,
        Commands::Metrics => cmd_metrics(&client, &cli.node, json).await,
        Commands::Verify => cmd_verify(&client, &cli.node, json).await,
        Commands::Consistency => cmd_consistency(&client, json).await,
        Commands::Env => cmd_env().await,
        Commands::Logs { target, lines } => cmd_logs(&target, lines),
        Commands::Restart { target } => cmd_restart(&target),
    }
}
