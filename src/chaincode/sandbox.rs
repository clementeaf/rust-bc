//! Chaincode sandbox validator — static analysis gate before deployment.
//!
//! Validates Wasm bytes without executing them:
//! - Well-formedness (valid Wasm binary)
//! - Import whitelist (only allowed host functions)
//! - Memory limits (initial pages ≤ max)
//!
//! Produces a `SandboxReport` that can be stored and queried.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Allowed imports: `(module, name)` pairs that chaincode may reference.
const ALLOWED_IMPORTS: &[(&str, &str)] = &[
    ("env", "put_state"),
    ("env", "get_state"),
    ("env", "set_event"),
    ("env", "set_key_endorsement_policy"),
    ("env", "get_history_for_key"),
    ("env", "invoke_chaincode"),
];

/// Maximum initial memory pages allowed (1 page = 64 KB).
const MAX_INITIAL_MEMORY_PAGES: u64 = 16; // 1 MB

/// Result of a single validation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Aggregate sandbox validation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxReport {
    pub chaincode_id: String,
    pub version: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,
    pub wasm_size_bytes: usize,
    pub duration_ms: u64,
}

/// Validate Wasm bytes (binary or WAT text) and produce a `SandboxReport`.
pub fn validate(chaincode_id: &str, version: &str, wasm_bytes: &[u8]) -> SandboxReport {
    let start = Instant::now();
    let mut checks = Vec::new();

    // Convert WAT to binary if needed (wasmparser only accepts binary Wasm).
    let binary = match to_binary(wasm_bytes) {
        Ok(b) => b,
        Err(e) => {
            checks.push(CheckResult {
                name: "well_formedness".to_string(),
                passed: false,
                detail: format!("invalid Wasm: {e}"),
            });
            return SandboxReport {
                chaincode_id: chaincode_id.to_string(),
                version: version.to_string(),
                passed: false,
                checks,
                wasm_size_bytes: wasm_bytes.len(),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    // 1. Well-formedness
    checks.push(check_well_formedness(&binary));

    // Only run further checks if Wasm is valid
    if checks[0].passed {
        // 2. Import whitelist
        checks.push(check_imports(&binary));

        // 3. Memory limits
        checks.push(check_memory(&binary));
    }

    let passed = checks.iter().all(|c| c.passed);

    SandboxReport {
        chaincode_id: chaincode_id.to_string(),
        version: version.to_string(),
        passed,
        checks,
        wasm_size_bytes: wasm_bytes.len(),
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Convert WAT text to binary Wasm. If already binary, returns as-is.
fn to_binary(input: &[u8]) -> Result<Vec<u8>, String> {
    // Wasm binary magic: \0asm
    if input.len() >= 4 && input[0..4] == [0x00, 0x61, 0x73, 0x6d] {
        return Ok(input.to_vec());
    }
    // Try WAT → binary conversion
    wat::parse_bytes(input)
        .map(|cow| cow.into_owned())
        .map_err(|e| e.to_string())
}

fn check_well_formedness(wasm_bytes: &[u8]) -> CheckResult {
    match wasmparser::Validator::new().validate_all(wasm_bytes) {
        Ok(_) => CheckResult {
            name: "well_formedness".to_string(),
            passed: true,
            detail: "valid Wasm binary".to_string(),
        },
        Err(e) => CheckResult {
            name: "well_formedness".to_string(),
            passed: false,
            detail: format!("invalid Wasm: {e}"),
        },
    }
}

fn check_imports(wasm_bytes: &[u8]) -> CheckResult {
    let parser = wasmparser::Parser::new(0);
    let mut forbidden = Vec::new();

    for payload in parser.parse_all(wasm_bytes) {
        let payload = match payload {
            Ok(p) => p,
            Err(_) => continue,
        };
        if let wasmparser::Payload::ImportSection(reader) = payload {
            for import in reader {
                let import = match import {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let module = import.module;
                let name = import.name;
                // Allow memory imports (they're checked separately)
                if matches!(import.ty, wasmparser::TypeRef::Memory(_)) {
                    continue;
                }
                if !ALLOWED_IMPORTS.contains(&(module, name)) {
                    forbidden.push(format!("{module}::{name}"));
                }
            }
        }
    }

    if forbidden.is_empty() {
        CheckResult {
            name: "import_whitelist".to_string(),
            passed: true,
            detail: "all imports are allowed".to_string(),
        }
    } else {
        CheckResult {
            name: "import_whitelist".to_string(),
            passed: false,
            detail: format!("forbidden imports: {}", forbidden.join(", ")),
        }
    }
}

fn check_memory(wasm_bytes: &[u8]) -> CheckResult {
    let parser = wasmparser::Parser::new(0);
    let mut max_initial: u64 = 0;

    for payload in parser.parse_all(wasm_bytes) {
        let payload = match payload {
            Ok(p) => p,
            Err(_) => continue,
        };
        match payload {
            wasmparser::Payload::MemorySection(reader) => {
                for mem in reader.into_iter().flatten() {
                    max_initial = max_initial.max(mem.initial);
                }
            }
            wasmparser::Payload::ImportSection(reader) => {
                for import in reader.into_iter().flatten() {
                    if let wasmparser::TypeRef::Memory(m) = import.ty {
                        max_initial = max_initial.max(m.initial);
                    }
                }
            }
            _ => {}
        }
    }

    if max_initial <= MAX_INITIAL_MEMORY_PAGES {
        CheckResult {
            name: "memory_limits".to_string(),
            passed: true,
            detail: format!(
                "initial memory: {max_initial} pages ({} KB), max allowed: {MAX_INITIAL_MEMORY_PAGES} pages",
                max_initial * 64
            ),
        }
    } else {
        CheckResult {
            name: "memory_limits".to_string(),
            passed: false,
            detail: format!(
                "initial memory: {max_initial} pages ({} KB) exceeds limit of {MAX_INITIAL_MEMORY_PAGES} pages ({} KB)",
                max_initial * 64,
                MAX_INITIAL_MEMORY_PAGES * 64
            ),
        }
    }
}

/// Trait for persisting and querying sandbox reports.
pub trait SandboxReportStore: Send + Sync {
    fn store_report(&self, report: &SandboxReport);
    fn get_report(&self, chaincode_id: &str, version: &str) -> Option<SandboxReport>;
}

/// In-memory implementation.
pub struct MemorySandboxReportStore {
    reports: Mutex<HashMap<String, SandboxReport>>,
}

impl MemorySandboxReportStore {
    pub fn new() -> Self {
        Self {
            reports: Mutex::new(HashMap::new()),
        }
    }

    fn key(chaincode_id: &str, version: &str) -> String {
        format!("{chaincode_id}:{version}")
    }
}

impl Default for MemorySandboxReportStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxReportStore for MemorySandboxReportStore {
    fn store_report(&self, report: &SandboxReport) {
        self.reports
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                Self::key(&report.chaincode_id, &report.version),
                report.clone(),
            );
    }

    fn get_report(&self, chaincode_id: &str, version: &str) -> Option<SandboxReport> {
        self.reports
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&Self::key(chaincode_id, version))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid Wasm module (no imports, no memory).
    const VALID_EMPTY: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    /// Valid WAT with allowed imports only.
    const VALID_WITH_IMPORTS: &[u8] = br#"
(module
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i64) (i64.const 0))
)
"#;

    /// WAT with forbidden import (syscall).
    const FORBIDDEN_IMPORT: &[u8] = br#"
(module
  (import "env" "put_state" (func $put (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i64) (i64.const 0))
)
"#;

    /// WAT with oversized memory (32 pages = 2 MB, limit is 16).
    const OVERSIZED_MEMORY: &[u8] = br#"
(module
  (memory (export "memory") 32)
  (func (export "run") (result i64) (i64.const 0))
)
"#;

    #[test]
    fn valid_empty_module_passes() {
        let report = validate("test", "1.0", VALID_EMPTY);
        assert!(report.passed);
        assert!(report.checks.iter().all(|c| c.passed));
    }

    #[test]
    fn valid_with_allowed_imports_passes() {
        let report = validate("mycc", "1.0", VALID_WITH_IMPORTS);
        assert!(report.passed, "report: {report:?}");
        assert_eq!(report.checks.len(), 3);
    }

    #[test]
    fn malformed_wasm_fails() {
        let report = validate("bad", "1.0", b"not wasm at all");
        assert!(!report.passed);
        assert!(!report.checks[0].passed);
        assert!(report.checks[0].detail.contains("invalid Wasm"));
        // Subsequent checks skipped
        assert_eq!(report.checks.len(), 1);
    }

    #[test]
    fn forbidden_import_fails() {
        let report = validate("evil", "1.0", FORBIDDEN_IMPORT);
        assert!(!report.passed);
        let import_check = report
            .checks
            .iter()
            .find(|c| c.name == "import_whitelist")
            .unwrap();
        assert!(!import_check.passed);
        assert!(import_check.detail.contains("fd_write"));
    }

    #[test]
    fn oversized_memory_fails() {
        let report = validate("big", "1.0", OVERSIZED_MEMORY);
        assert!(!report.passed);
        let mem_check = report
            .checks
            .iter()
            .find(|c| c.name == "memory_limits")
            .unwrap();
        assert!(!mem_check.passed);
        assert!(mem_check.detail.contains("32 pages"));
    }

    #[test]
    fn report_includes_metadata() {
        let report = validate("mycc", "2.0", VALID_EMPTY);
        assert_eq!(report.chaincode_id, "mycc");
        assert_eq!(report.version, "2.0");
        assert_eq!(report.wasm_size_bytes, VALID_EMPTY.len());
    }

    #[test]
    fn report_store_roundtrip() {
        let store = MemorySandboxReportStore::new();
        let report = validate("mycc", "1.0", VALID_EMPTY);
        store.store_report(&report);

        let retrieved = store.get_report("mycc", "1.0").unwrap();
        assert_eq!(retrieved.chaincode_id, "mycc");
        assert_eq!(retrieved.passed, report.passed);
    }

    #[test]
    fn report_store_returns_none_for_missing() {
        let store = MemorySandboxReportStore::new();
        assert!(store.get_report("nonexistent", "1.0").is_none());
    }

    #[test]
    fn memory_at_limit_passes() {
        // Exactly MAX_INITIAL_MEMORY_PAGES (16 pages = 1 MB)
        let wat = br#"
(module
  (memory (export "memory") 16)
  (func (export "run") (result i64) (i64.const 0))
)
"#;
        let report = validate("exact", "1.0", wat);
        assert!(report.passed, "report: {report:?}");
    }

    #[test]
    fn memory_one_over_limit_fails() {
        let wat = br#"
(module
  (memory (export "memory") 17)
  (func (export "run") (result i64) (i64.const 0))
)
"#;
        let report = validate("over", "1.0", wat);
        assert!(!report.passed);
    }
}
