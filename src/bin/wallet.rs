//! Cerulean Ledger — Wallet CLI
//!
//! Usage:
//!   wallet generate              Generate a new Ed25519 keypair
//!   wallet generate --pqc        Generate a new ML-DSA-65 keypair
//!   wallet address <pubkey_hex>  Derive address from public key
//!   wallet balance <address> [--node URL]
//!   wallet transfer <from> <to> <amount> --key <privkey_hex> [--fee N] [--chain-id N] [--node URL]

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "generate" | "gen" => cmd_generate(&args[2..]),
        "address" | "addr" => cmd_address(&args[2..]),
        "balance" | "bal" => cmd_balance(&args[2..]),
        "transfer" | "send" => cmd_transfer(&args[2..]),
        "help" | "--help" | "-h" => print_usage(),
        other => {
            eprintln!("Unknown command: {other}");
            print_usage();
        }
    }
}

fn print_usage() {
    println!("Cerulean Ledger — Wallet CLI");
    println!();
    println!("Commands:");
    println!("  generate [--pqc]                 Generate keypair (Ed25519 or ML-DSA-65)");
    println!("  address <pubkey_hex>             Derive address from public key");
    println!("  balance <address> [--node URL]   Query account balance");
    println!("  transfer <from> <to> <amount>    Send tokens");
    println!("    --key <privkey_hex>            Signing key (Ed25519 hex)");
    println!("    --fee <amount>                 Fee (default: 5)");
    println!("    --chain-id <id>                Chain ID (default: 0)");
    println!("    --node <url>                   Node URL (default: http://localhost:8080)");
}

fn cmd_generate(args: &[String]) {
    let pqc = args.iter().any(|a| a == "--pqc");

    if pqc {
        use pqc_crypto_module::legacy::mldsa_raw::mldsa65;
        use pqcrypto_traits::sign::{PublicKey, SecretKey};
        let (pk, sk) = mldsa65::keypair();
        let pk_bytes = pk.as_bytes();
        let sk_bytes = sk.as_bytes();
        let addr = rust_bc::account::address::address_from_pubkey(pk_bytes);

        println!("Algorithm:   ML-DSA-65 (post-quantum)");
        println!("Address:     {addr}");
        println!(
            "Public key:  {} ({} bytes)",
            hex::encode(pk_bytes),
            pk_bytes.len()
        );
        println!(
            "Private key: {} ({} bytes)",
            hex::encode(sk_bytes),
            sk_bytes.len()
        );
    } else {
        use pqc_crypto_module::legacy::rng::OsRng;
        let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key();
        let addr = rust_bc::account::address::address_from_pubkey(pk.as_bytes());

        println!("Algorithm:   Ed25519");
        println!("Address:     {addr}");
        println!("Public key:  {}", hex::encode(pk.as_bytes()));
        println!("Private key: {}", hex::encode(sk.to_bytes()));
    }
    println!();
    eprintln!("Save your private key securely. It cannot be recovered.");
}

fn cmd_address(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: wallet address <pubkey_hex>");
        return;
    }
    match hex::decode(&args[0]) {
        Ok(pk_bytes) => {
            let addr = rust_bc::account::address::address_from_pubkey(&pk_bytes);
            println!("{addr}");
        }
        Err(e) => eprintln!("Invalid hex: {e}"),
    }
}

fn cmd_balance(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: wallet balance <address> [--node URL]");
        return;
    }
    let address = &args[0];
    let node = find_flag(args, "--node").unwrap_or_else(|| "http://localhost:8080".to_string());
    let url = format!("{node}/api/v1/accounts/{address}");

    match ureq::get(&url).call() {
        Ok(resp) => {
            let body: serde_json::Value = resp.into_json().unwrap_or_default();
            if let Some(data) = body.get("data") {
                println!(
                    "Address:  {}",
                    data.get("address")
                        .and_then(|v| v.as_str())
                        .unwrap_or(address)
                );
                println!(
                    "Balance:  {} NOTA",
                    data.get("balance").and_then(|v| v.as_u64()).unwrap_or(0)
                );
                println!(
                    "Nonce:    {}",
                    data.get("nonce").and_then(|v| v.as_u64()).unwrap_or(0)
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&body).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

fn cmd_transfer(args: &[String]) {
    if args.len() < 3 {
        eprintln!("Usage: wallet transfer <from> <to> <amount> --key <privkey_hex>");
        return;
    }
    let from = &args[0];
    let to = &args[1];
    let amount: u64 = match args[2].parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Invalid amount: {e}");
            return;
        }
    };
    let key_hex = match find_flag(args, "--key") {
        Some(k) => k,
        None => {
            eprintln!("--key <privkey_hex> is required");
            return;
        }
    };
    let fee: u64 = find_flag(args, "--fee")
        .and_then(|f| f.parse().ok())
        .unwrap_or(5);
    let chain_id: u64 = find_flag(args, "--chain-id")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let node = find_flag(args, "--node").unwrap_or_else(|| "http://localhost:8080".to_string());

    // Get current nonce
    let nonce_url = format!("{node}/api/v1/accounts/{from}");
    let nonce: u64 = ureq::get(&nonce_url)
        .call()
        .ok()
        .and_then(|r| r.into_json::<serde_json::Value>().ok())
        .and_then(|b| b.get("data")?.get("nonce")?.as_u64())
        .unwrap_or(0);

    // Build and sign transaction
    let mut tx = rust_bc::transaction::native::NativeTransaction::new_transfer_with_chain(
        from.as_str(),
        to.as_str(),
        amount,
        nonce,
        fee,
        chain_id,
    );

    let sk_bytes = match hex::decode(&key_hex) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Invalid key hex: {e}");
            return;
        }
    };

    if sk_bytes.len() == 32 {
        let sk_arr: [u8; 32] = sk_bytes.try_into().unwrap();
        let sk = ed25519_dalek::SigningKey::from_bytes(&sk_arr);
        use pqc_crypto_module::legacy::ed25519::Signer;
        let payload = tx.signing_payload();
        let sig = sk.sign(&payload);
        tx.signature = sig.to_bytes().to_vec();
        tx.signature_algorithm = "ed25519".to_string();
    } else {
        eprintln!("Only Ed25519 (32-byte) private keys supported in CLI");
        return;
    }

    // Submit
    let submit_url = format!("{node}/api/v1/transfer");
    let body = serde_json::json!({
        "from": from,
        "to": to,
        "amount": amount,
        "nonce": nonce,
        "fee": fee,
    });

    match ureq::post(&submit_url)
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(resp) => {
            let result: serde_json::Value = resp.into_json().unwrap_or_default();
            if let Some(data) = result.get("data") {
                println!("Transfer submitted");
                println!(
                    "  TX ID: {}",
                    data.get("tx_id").and_then(|v| v.as_str()).unwrap_or("?")
                );
                println!("  From:  {from}");
                println!("  To:    {to}");
                println!("  Amount: {amount} NOTA");
                println!("  Fee:   {fee}");
                println!("  Nonce: {nonce}");
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            }
        }
        Err(e) => eprintln!("Error submitting: {e}"),
    }
}

fn find_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
