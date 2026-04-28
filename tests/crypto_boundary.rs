//! Cryptographic boundary enforcement tests.
//!
//! Ensures that production code does not directly import raw crypto crates.
//! All cryptography should go through `pqc_crypto_module::api`.
//!
//! Files in the LEGACY_ALLOWLIST are pre-existing violations documented for
//! gradual migration. New files adding direct crypto imports will FAIL this test.

use std::path::{Path, PathBuf};

/// Raw crypto crate imports that must not appear in new production code.
const FORBIDDEN_IMPORTS: &[&str] = &[
    "use pqcrypto_mldsa",
    "use pqcrypto_traits",
    "use sha2::",
    "use sha3::",
    "use ed25519_dalek",
    "pqcrypto_mldsa::",
    "use ring::",
    "use openssl::",
    "use k256::",
    "use p256::",
    "use rsa::",
    "use blake",
];

/// Legacy allowlist — EMPTY after full migration.
///
/// All 28 previously-listed files now import crypto exclusively through
/// `pqc_crypto_module::legacy::*` instead of raw crates.
const LEGACY_ALLOWLIST: &[&str] = &[];

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_rs_files(&path));
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            files.push(path);
        }
    }
    files
}

fn is_allowlisted(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    LEGACY_ALLOWLIST
        .iter()
        .any(|allowed| path_str.ends_with(allowed))
}

// ═══════════════════════════════════════════════════════════════════
// TEST 1: No NEW files with raw crypto imports
// ═══════════════════════════════════════════════════════════════════

#[test]
fn no_raw_crypto_imports_outside_crypto_module_and_allowlist() {
    let src_dir = Path::new("src");
    let files = collect_rs_files(src_dir);
    let mut violations: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        if is_allowlisted(file) {
            continue;
        }

        let content = std::fs::read_to_string(file).unwrap();
        for &forbidden in FORBIDDEN_IMPORTS {
            if content.contains(forbidden) {
                violations.push((file.clone(), forbidden.to_string()));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("=== CRYPTO BOUNDARY VIOLATIONS ===");
        for (file, import) in &violations {
            eprintln!("  VIOLATION: '{}' in {:?}", import, file);
        }
        panic!(
            "{} crypto boundary violations found in non-allowlisted files. \
             New production code must use pqc_crypto_module::api instead of raw crypto crates.",
            violations.len()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// TEST 2: Allowlist is complete — no undocumented violations
// ═══════════════════════════════════════════════════════════════════

#[test]
fn allowlist_covers_all_existing_violations() {
    let src_dir = Path::new("src");
    let files = collect_rs_files(src_dir);
    let mut undocumented: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        let content = std::fs::read_to_string(file).unwrap();
        for &forbidden in FORBIDDEN_IMPORTS {
            if content.contains(forbidden) && !is_allowlisted(file) {
                undocumented.push((file.clone(), forbidden.to_string()));
            }
        }
    }

    if !undocumented.is_empty() {
        eprintln!("=== UNDOCUMENTED CRYPTO IMPORTS ===");
        for (file, import) in &undocumented {
            eprintln!("  UNDOCUMENTED: '{}' in {:?}", import, file);
        }
        panic!(
            "{} undocumented crypto imports found. \
             Either migrate to pqc_crypto_module or add to LEGACY_ALLOWLIST with justification.",
            undocumented.len()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// TEST 3: pqc_crypto_module itself doesn't leak outside its boundary
// ═══════════════════════════════════════════════════════════════════

#[test]
fn crypto_module_crate_does_not_exist_in_src() {
    // The pqc_crypto_module code must live ONLY in crates/pqc_crypto_module/,
    // not duplicated in src/.
    let src_dir = Path::new("src");
    let files = collect_rs_files(src_dir);

    for file in &files {
        let content = std::fs::read_to_string(file).unwrap();
        // No file in src/ should re-implement the module's internals
        if content.contains("mod pqc_crypto_module") {
            panic!(
                "pqc_crypto_module must not be defined inside src/: {:?}",
                file
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// TEST 4: Allowlist entries actually exist
// ═══════════════════════════════════════════════════════════════════

#[test]
fn legacy_allowlist_is_empty() {
    assert!(
        LEGACY_ALLOWLIST.is_empty(),
        "LEGACY_ALLOWLIST must be empty — all files migrated to pqc_crypto_module"
    );
}

// ═══════════════════════════════════════════════════════════════════
// TEST 5: Report migration progress
// ═══════════════════════════════════════════════════════════════════

#[test]
fn report_migration_progress() {
    let src_dir = Path::new("src");
    let files = collect_rs_files(src_dir);
    let total_rs_files = files.len();

    let mut files_with_violations = 0;
    let mut files_clean = 0;

    for file in &files {
        let content = std::fs::read_to_string(file).unwrap();
        let has_violation = FORBIDDEN_IMPORTS
            .iter()
            .any(|&forbidden| content.contains(forbidden));
        if has_violation {
            files_with_violations += 1;
        } else {
            files_clean += 1;
        }
    }

    let pct_clean = (files_clean as f64 / total_rs_files as f64) * 100.0;

    eprintln!("=== Crypto Boundary Migration Progress ===");
    eprintln!("  Total .rs files in src/: {}", total_rs_files);
    eprintln!("  Files with direct crypto imports: {files_with_violations}");
    eprintln!("  Files clean (no direct crypto): {files_clean}");
    eprintln!("  Legacy allowlist size: {}", LEGACY_ALLOWLIST.len());
    eprintln!("  Migration progress: {pct_clean:.1}% clean");
}
