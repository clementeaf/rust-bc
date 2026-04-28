//! ACVP-inspired JSON vector format types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct VectorFile {
    #[allow(dead_code)]
    pub algorithm: String,
    #[serde(rename = "testGroups")]
    pub test_groups: Vec<TestGroup>,
}

#[derive(Debug, Deserialize)]
pub struct TestGroup {
    #[serde(rename = "tgId")]
    #[allow(dead_code)]
    pub tg_id: u32,
    pub tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
pub struct TestCase {
    #[serde(rename = "tcId")]
    pub tc_id: u32,
    #[serde(rename = "msgHex")]
    pub msg_hex: Option<String>,
    #[serde(rename = "expectedDigestHex")]
    pub expected_digest_hex: Option<String>,
    #[serde(rename = "messageHex")]
    pub message_hex: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    #[serde(rename = "tcId")]
    pub tc_id: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AlgorithmReport {
    pub algorithm: String,
    pub passed: u32,
    pub failed: u32,
    pub results: Vec<TestResult>,
}

pub fn load_vectors(path: &str) -> Result<VectorFile, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read {path}: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("failed to parse {path}: {e}"))
}
