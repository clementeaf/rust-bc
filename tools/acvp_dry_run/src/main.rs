//! ACVP dry-run harness for pqc_crypto_module.
//!
//! Processes ACVP-inspired test vectors for SHA3-256, ML-DSA-65, and ML-KEM-768.
//!
//! Usage:
//!   cargo run -p acvp_dry_run -- --algorithm sha3-256 --vectors vectors/sha3_256.json
//!   cargo run -p acvp_dry_run -- --algorithm ml-dsa-65 --vectors vectors/mldsa_65.json
//!   cargo run -p acvp_dry_run -- --algorithm ml-kem-768 --vectors vectors/mlkem_768.json
//!   cargo run -p acvp_dry_run -- --all

mod mldsa;
mod mlkem;
mod report;
mod sha3;
mod vectors;

use clap::Parser;
use report::FullReport;
use vectors::AlgorithmReport;

#[derive(Parser)]
#[command(name = "acvp_dry_run", about = "ACVP test vector dry-run harness")]
struct Cli {
    /// Algorithm to test: sha3-256, ml-dsa-65, ml-kem-768
    #[arg(long)]
    algorithm: Option<String>,

    /// Path to ACVP vector JSON file
    #[arg(long)]
    vectors: Option<String>,

    /// Run all algorithms with default vector files
    #[arg(long)]
    all: bool,
}

const DEFAULT_SHA3_VECTORS: &str = "tools/acvp_dry_run/vectors/sha3_256.json";
const DEFAULT_MLDSA_VECTORS: &str = "tools/acvp_dry_run/vectors/mldsa_65.json";
const DEFAULT_MLKEM_VECTORS: &str = "tools/acvp_dry_run/vectors/mlkem_768.json";

fn main() {
    let cli = Cli::parse();

    // Initialize crypto module
    pqc_crypto_module::api::initialize_approved_mode().expect("crypto module self-tests failed");

    let mut reports: Vec<AlgorithmReport> = Vec::new();

    if cli.all {
        reports.push(run_algorithm("sha3-256", DEFAULT_SHA3_VECTORS));
        reports.push(run_algorithm("ml-dsa-65", DEFAULT_MLDSA_VECTORS));
        reports.push(run_algorithm("ml-kem-768", DEFAULT_MLKEM_VECTORS));
    } else if let Some(ref alg) = cli.algorithm {
        let vectors_path = cli.vectors.as_deref().unwrap_or(match alg.as_str() {
            "sha3-256" => DEFAULT_SHA3_VECTORS,
            "ml-dsa-65" => DEFAULT_MLDSA_VECTORS,
            "ml-kem-768" => DEFAULT_MLKEM_VECTORS,
            _ => {
                eprintln!("Unknown algorithm: {alg}");
                eprintln!("Supported: sha3-256, ml-dsa-65, ml-kem-768");
                std::process::exit(1);
            }
        });
        reports.push(run_algorithm(alg, vectors_path));
    } else {
        eprintln!("Error: specify --algorithm <name> or --all");
        std::process::exit(1);
    }

    let report = FullReport::new(reports);
    report.print_summary();

    // Write JSON report
    let json = report.to_json();
    if let Err(e) = std::fs::write("tools/acvp_dry_run/report.json", &json) {
        eprintln!("Warning: could not write report.json: {e}");
        // Still print to stdout as fallback
        println!("{json}");
    }

    if report.total_failed > 0 {
        std::process::exit(1);
    }
}

fn run_algorithm(algorithm: &str, vectors_path: &str) -> AlgorithmReport {
    println!("Running {algorithm} vectors from {vectors_path}...");

    let vectors = match vectors::load_vectors(vectors_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error loading vectors: {e}");
            return AlgorithmReport {
                algorithm: algorithm.into(),
                passed: 0,
                failed: 1,
                results: vec![vectors::TestResult {
                    tc_id: 0,
                    status: "failed".into(),
                    detail: Some(e),
                }],
            };
        }
    };

    match algorithm {
        "sha3-256" => sha3::run(&vectors),
        "ml-dsa-65" => mldsa::run(&vectors),
        "ml-kem-768" => mlkem::run(&vectors),
        _ => AlgorithmReport {
            algorithm: algorithm.into(),
            passed: 0,
            failed: 1,
            results: vec![vectors::TestResult {
                tc_id: 0,
                status: "failed".into(),
                detail: Some(format!("unknown algorithm: {algorithm}")),
            }],
        },
    }
}
