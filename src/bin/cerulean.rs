//! Cerulean CLI — digital signature tool for Cerulean Ledger.
//!
//! Commands:
//!   init   — Generate keypair and register DID on the network
//!   sign   — Sign a file and register the credential on-chain
//!   verify — Verify a credential by ID
//!   list   — List credentials issued by this identity

use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── CLI definition ───────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "cerulean", about = "Cerulean Ledger — digital signature CLI")]
struct Cli {
    /// Node URL (default: http://localhost:8080)
    #[arg(long, default_value = "http://localhost:8080")]
    node: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate keypair and register identity on the network
    Init {
        /// Your display name
        name: String,
        /// Organization ID
        #[arg(long, default_value = "default")]
        org: String,
    },
    /// Sign a file and register the credential on-chain
    Sign {
        /// Path to the file to sign
        file: PathBuf,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Verify a credential by its ID
    Verify {
        /// Credential ID
        credential_id: String,
    },
    /// List credentials issued by this identity
    List,
}

// ── Wallet ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Wallet {
    did: String,
    name: String,
    org: String,
    /// Ed25519 secret key (32 bytes, hex-encoded)
    secret_key_hex: String,
    /// Ed25519 public key (32 bytes, hex-encoded)
    public_key_hex: String,
}

impl Wallet {
    fn wallet_dir() -> PathBuf {
        dirs_next().join("wallet.json")
    }

    fn load() -> Result<Self, String> {
        let path = Self::wallet_dir();
        if !path.exists() {
            return Err(format!(
                "No wallet found. Run `cerulean init \"Your Name\"` first.\n  Expected: {}",
                path.display()
            ));
        }
        let data = fs::read_to_string(&path).map_err(|e| format!("read wallet: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("parse wallet: {e}"))
    }

    fn save(&self) -> Result<(), String> {
        let dir = dirs_next();
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write(Self::wallet_dir(), json).map_err(|e| format!("write wallet: {e}"))
    }

    fn signing_key(&self) -> Result<SigningKey, String> {
        let bytes = hex::decode(&self.secret_key_hex).map_err(|e| format!("decode key: {e}"))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "invalid key length".to_string())?;
        Ok(SigningKey::from_bytes(&arr))
    }
}

fn dirs_next() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cerulean")
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("http client")
}

// ── Commands ─────────────────────────────────────────────────────────────────

fn cmd_init(node: &str, name: &str, org: &str) -> Result<(), String> {
    let path = Wallet::wallet_dir();
    if path.exists() {
        return Err(format!(
            "Wallet already exists at {}\n  Delete it first if you want to re-initialize.",
            path.display()
        ));
    }

    // Generate Ed25519 keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key: VerifyingKey = (&signing_key).into();

    let secret_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());
    let did = format!("did:cerulean:{}", &public_hex[..16]);

    // Register DID on the node
    let client = http_client();
    let body = serde_json::json!({
        "did": did,
        "public_key": public_hex,
        "metadata": {
            "name": name,
            "org": org,
            "created": chrono::Utc::now().to_rfc3339(),
        }
    });

    let resp = client
        .post(format!("{node}/api/v1/identity"))
        .header("Content-Type", "application/json")
        .header("X-Org-Id", org)
        .header("X-Msp-Role", "client")
        .json(&body)
        .send()
        .map_err(|e| format!("Failed to connect to node at {node}: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("Node returned {status}: {text}"));
    }

    // Save wallet
    let wallet = Wallet {
        did: did.clone(),
        name: name.to_string(),
        org: org.to_string(),
        secret_key_hex: secret_hex,
        public_key_hex: public_hex,
    };
    wallet.save()?;

    println!("Identity created successfully!\n");
    println!("  DID:    {did}");
    println!("  Name:   {name}");
    println!("  Org:    {org}");
    println!("  Wallet: {}", path.display());
    println!("\nYour private key is stored locally. Never share it.");
    Ok(())
}

fn cmd_sign(node: &str, file: &PathBuf, description: Option<&str>) -> Result<(), String> {
    let wallet = Wallet::load()?;
    let signing_key = wallet.signing_key()?;

    // Read and hash the file
    if !file.exists() {
        return Err(format!("File not found: {}", file.display()));
    }
    let file_bytes = fs::read(file).map_err(|e| format!("read file: {e}"))?;
    let file_hash = Sha256::digest(&file_bytes);
    let hash_hex = hex::encode(file_hash);

    // Sign the hash
    let signature = signing_key.sign(file_hash.as_slice());
    let sig_hex = hex::encode(signature.to_bytes());

    // Build credential
    let file_name = file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let cred_id = format!("sig-{}", &hash_hex[..12]);
    let now = chrono::Utc::now().to_rfc3339();

    let body = serde_json::json!({
        "id": cred_id,
        "issuer_did": wallet.did,
        "subject_did": format!("did:cerulean:doc:{}", &hash_hex[..16]),
        "credential_type": "DigitalSignature",
        "claims": {
            "file_name": file_name,
            "file_hash_sha256": hash_hex,
            "description": description.unwrap_or(""),
            "signed_at": now,
        },
        "signature": sig_hex,
    });

    let client = http_client();
    let resp = client
        .post(format!("{node}/api/v1/credentials"))
        .header("Content-Type", "application/json")
        .header("X-Org-Id", &wallet.org)
        .header("X-Msp-Role", "client")
        .json(&body)
        .send()
        .map_err(|e| format!("Failed to connect to node: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("Node returned {status}: {text}"));
    }

    println!("Document signed and registered!\n");
    println!("  Credential ID: {cred_id}");
    println!("  File:          {file_name}");
    println!("  SHA-256:       {hash_hex}");
    println!("  Signer:        {}", wallet.did);
    println!("  Timestamp:     {now}");
    println!("\nAnyone can verify with: cerulean verify {cred_id}");
    Ok(())
}

fn cmd_verify(node: &str, credential_id: &str) -> Result<(), String> {
    let client = http_client();
    let resp = client
        .get(format!("{node}/api/v1/credentials/{credential_id}"))
        .send()
        .map_err(|e| format!("Failed to connect to node: {e}"))?;

    if resp.status().as_u16() == 404 {
        return Err(format!(
            "Credential '{credential_id}' not found on the network."
        ));
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("Node returned {status}: {text}"));
    }

    let body: serde_json::Value = resp.json().map_err(|e| format!("parse response: {e}"))?;

    // Extract credential from response envelope
    let cred = body.get("data").unwrap_or(&body);

    let issuer = cred["issuer_did"].as_str().unwrap_or("unknown");
    let cred_type = cred["credential_type"].as_str().unwrap_or("unknown");
    let claims = &cred["claims"];
    let sig_hex = cred["signature"].as_str().unwrap_or("");

    // Fetch issuer's public key to verify signature
    let id_resp = client
        .get(format!("{node}/api/v1/identity/{issuer}"))
        .send()
        .map_err(|e| format!("fetch issuer identity: {e}"))?;

    let mut signature_valid = false;
    if id_resp.status().is_success() {
        let id_body: serde_json::Value = id_resp.json().unwrap_or_default();
        let id_data = id_body.get("data").unwrap_or(&id_body);
        if let Some(pk_hex) = id_data["public_key"].as_str() {
            if let Some(file_hash_hex) = claims["file_hash_sha256"].as_str() {
                signature_valid = verify_signature(pk_hex, file_hash_hex, sig_hex);
            }
        }
    }

    println!("Credential: {credential_id}\n");
    println!("  Type:        {cred_type}");
    println!("  Issuer:      {issuer}");
    if let Some(name) = claims.get("file_name").and_then(|v| v.as_str()) {
        println!("  File:        {name}");
    }
    if let Some(hash) = claims.get("file_hash_sha256").and_then(|v| v.as_str()) {
        println!("  SHA-256:     {hash}");
    }
    if let Some(ts) = claims.get("signed_at").and_then(|v| v.as_str()) {
        println!("  Signed at:   {ts}");
    }
    if let Some(desc) = claims.get("description").and_then(|v| v.as_str()) {
        if !desc.is_empty() {
            println!("  Description: {desc}");
        }
    }
    println!();
    if signature_valid {
        println!("  SIGNATURE VALID");
    } else if sig_hex.is_empty() {
        println!("  WARNING: No signature present");
    } else {
        println!("  SIGNATURE COULD NOT BE VERIFIED");
        println!("  (issuer public key not found or signature mismatch)");
    }

    Ok(())
}

fn cmd_list(node: &str) -> Result<(), String> {
    let wallet = Wallet::load()?;
    let client = http_client();

    let resp = client
        .get(format!(
            "{node}/api/v1/credentials?issuer={}",
            urlencoding::encode(&wallet.did)
        ))
        .send()
        .map_err(|e| format!("Failed to connect to node: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("Node returned {status}: {text}"));
    }

    let body: serde_json::Value = resp.json().map_err(|e| format!("parse response: {e}"))?;
    let creds = body["data"]
        .as_array()
        .or_else(|| body.as_array())
        .cloned()
        .unwrap_or_default();

    if creds.is_empty() {
        println!("No credentials found for {}", wallet.did);
        return Ok(());
    }

    println!("Credentials signed by {} ({}):\n", wallet.name, wallet.did);
    println!(
        "{:<16} {:<20} {:<24} Hash (first 16)",
        "ID", "File", "Signed at"
    );
    println!("{}", "-".repeat(80));

    for cred in &creds {
        let id = cred["id"].as_str().unwrap_or("-");
        let claims = &cred["claims"];
        let file = claims["file_name"].as_str().unwrap_or("-");
        let ts = claims["signed_at"].as_str().unwrap_or("-");
        let hash = claims["file_hash_sha256"].as_str().unwrap_or("-");
        let short_hash = if hash.len() >= 16 { &hash[..16] } else { hash };
        println!("{:<16} {:<20} {:<24} {}", id, file, ts, short_hash);
    }
    println!("\nTotal: {}", creds.len());
    Ok(())
}

// ── Crypto helpers ───────────────────────────────────────────────────────────

fn verify_signature(public_key_hex: &str, message_hex: &str, signature_hex: &str) -> bool {
    let Ok(pk_bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(signature_hex) else {
        return false;
    };
    let Ok(msg_bytes) = hex::decode(message_hex) else {
        return false;
    };

    let pk_arr: [u8; 32] = match pk_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return false,
    };

    let Ok(verifying_key) = VerifyingKey::from_bytes(&pk_arr) else {
        return false;
    };
    let signature = ed25519_dalek::Signature::from_bytes(&sig_arr);

    use ed25519_dalek::Verifier;
    verifying_key.verify(&msg_bytes, &signature).is_ok()
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let node = &cli.node;

    let result = match cli.command {
        Commands::Init { ref name, ref org } => cmd_init(node, name, org),
        Commands::Sign {
            ref file,
            ref description,
        } => cmd_sign(node, file, description.as_deref()),
        Commands::Verify { ref credential_id } => cmd_verify(node, credential_id),
        Commands::List => cmd_list(node),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
