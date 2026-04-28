//! Report generation for ACVP dry-run results.

use crate::vectors::AlgorithmReport;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FullReport {
    pub tool: String,
    pub version: String,
    pub algorithms: Vec<AlgorithmReport>,
    pub total_passed: u32,
    pub total_failed: u32,
}

impl FullReport {
    pub fn new(algorithms: Vec<AlgorithmReport>) -> Self {
        let total_passed = algorithms.iter().map(|a| a.passed).sum();
        let total_failed = algorithms.iter().map(|a| a.failed).sum();
        Self {
            tool: "acvp_dry_run".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            algorithms,
            total_passed,
            total_failed,
        }
    }

    pub fn print_summary(&self) {
        println!("\n=== ACVP Dry-Run Report ===\n");
        for alg in &self.algorithms {
            let status = if alg.failed == 0 { "PASS" } else { "FAIL" };
            println!(
                "  {}: {} ({} passed, {} failed)",
                alg.algorithm, status, alg.passed, alg.failed
            );
        }
        println!(
            "\n  Total: {} passed, {} failed",
            self.total_passed, self.total_failed
        );
        if self.total_failed == 0 {
            println!("  Status: ALL VECTORS PASSED");
        } else {
            println!("  Status: FAILURES DETECTED");
        }
        println!();
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}
