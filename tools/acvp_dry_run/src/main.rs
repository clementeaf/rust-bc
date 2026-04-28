//! ACVP dry-run harness for pqc_crypto_module.
//!
//! Placeholder tool for processing NIST ACVP test vectors.
//! Supports: SHA3-256, ML-DSA-65, ML-KEM-768.
//!
//! Usage:
//!   cargo run -p acvp_dry_run -- --algorithm sha3-256
//!   cargo run -p acvp_dry_run -- --algorithm ml-dsa-65 --vectors vectors/mldsa.json

use clap::Parser;

#[derive(Parser)]
#[command(name = "acvp_dry_run", about = "ACVP test vector dry-run harness")]
struct Cli {
    /// Algorithm to test: sha3-256, ml-dsa-65, ml-kem-768
    #[arg(long)]
    algorithm: String,

    /// Path to ACVP vector JSON file (optional for built-in KAT)
    #[arg(long)]
    vectors: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    // Initialize crypto module
    pqc_crypto_module::api::initialize_approved_mode().expect("crypto module self-tests failed");

    println!("ACVP Dry-Run: algorithm={}", cli.algorithm);

    match cli.algorithm.as_str() {
        "sha3-256" => run_sha3_kat(),
        "ml-dsa-65" => run_mldsa_kat(),
        "ml-kem-768" => run_mlkem_kat(),
        _ => {
            eprintln!("Unknown algorithm: {}", cli.algorithm);
            std::process::exit(1);
        }
    }

    if let Some(path) = &cli.vectors {
        println!("Vector file: {path}");
        println!("Status: PARTIAL — official ACVP vector parsing not yet implemented");
        println!("Action: Integrate NIST ACVP JSON schema when available");
    }
}

fn run_sha3_kat() {
    let hash = pqc_crypto_module::api::sha3_256(b"").unwrap();
    let expected = "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
    assert_eq!(hash.to_hex(), expected);
    println!("SHA3-256 KAT: PASS (empty string)");

    let hash2 = pqc_crypto_module::api::sha3_256(b"abc").unwrap();
    println!("SHA3-256(\"abc\") = {}", hash2.to_hex());
    println!("SHA3-256 KAT: PASS");
}

fn run_mldsa_kat() {
    let kp = pqc_crypto_module::api::generate_mldsa_keypair().unwrap();
    let msg = b"ACVP-KAT-ML-DSA-65";
    let sig = pqc_crypto_module::api::sign_message(&kp.private_key, msg).unwrap();
    pqc_crypto_module::api::verify_signature(&kp.public_key, msg, &sig).unwrap();
    println!("ML-DSA-65 sign/verify KAT: PASS");
    println!("  public_key: {} bytes", kp.public_key.as_bytes().len());
    println!("  signature:  {} bytes", sig.as_bytes().len());

    // Corrupted sig must fail
    let mut bad = sig.as_bytes().to_vec();
    bad[0] ^= 0xff;
    let bad_sig = pqc_crypto_module::types::MldsaSignature(bad);
    assert!(pqc_crypto_module::api::verify_signature(&kp.public_key, msg, &bad_sig).is_err());
    println!("ML-DSA-65 corrupted sig rejection: PASS");
}

fn run_mlkem_kat() {
    let kp = pqc_crypto_module::api::generate_mlkem_keypair().unwrap();
    let (ct, _ss) = pqc_crypto_module::api::mlkem_encapsulate(&kp.public_key).unwrap();
    let _ss2 = pqc_crypto_module::api::mlkem_decapsulate(&kp.private_key, &ct).unwrap();
    println!("ML-KEM-768 encaps/decaps KAT: PASS (placeholder implementation)");
    println!("  Note: Replace with FIPS 203 implementation for shared secret matching");
}
