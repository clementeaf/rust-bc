//! Integration tests for the ACVP dry-run harness.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .expect("cargo metadata failed");
    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid metadata json");
    PathBuf::from(meta["workspace_root"].as_str().unwrap())
}

fn cargo_bin() -> Command {
    let root = workspace_root();
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&root);
    cmd.args(["run", "-p", "acvp_dry_run", "--"]);
    cmd
}

#[test]
fn sha3_vectors_pass() {
    let output = cargo_bin()
        .args([
            "--algorithm",
            "sha3-256",
            "--vectors",
            "tools/acvp_dry_run/vectors/sha3_256.json",
        ])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "sha3 failed: {stdout}");
    assert!(stdout.contains("sha3-256: PASS"));
}

#[test]
fn mldsa_sign_then_verify_vectors_pass() {
    let output = cargo_bin()
        .args([
            "--algorithm",
            "ml-dsa-65",
            "--vectors",
            "tools/acvp_dry_run/vectors/mldsa_65.json",
        ])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "mldsa failed: {stdout}");
    assert!(stdout.contains("ml-dsa-65: PASS"));
}

#[test]
fn mlkem_encaps_then_decaps_vectors_pass() {
    let output = cargo_bin()
        .args([
            "--algorithm",
            "ml-kem-768",
            "--vectors",
            "tools/acvp_dry_run/vectors/mlkem_768.json",
        ])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "mlkem failed: {stdout}");
    assert!(stdout.contains("ml-kem-768: PASS"));
}

#[test]
fn invalid_vector_file_fails_cleanly() {
    let output = cargo_bin()
        .args(["--algorithm", "sha3-256", "--vectors", "nonexistent.json"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
}

#[test]
fn unknown_algorithm_rejected() {
    let output = cargo_bin()
        .args(["--algorithm", "aes-256"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
}
