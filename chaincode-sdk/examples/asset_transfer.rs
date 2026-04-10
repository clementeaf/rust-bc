//! Example chaincode: Asset Transfer
//!
//! Demonstrates a simple asset management contract with create, read,
//! update, and transfer operations.
//!
//! ## Build
//!
//! ```bash
//! cargo build --example asset_transfer --target wasm32-unknown-unknown --release
//! ```
//!
//! ## Deploy
//!
//! ```bash
//! curl -X POST https://localhost:8080/api/v1/chaincode/install \
//!   -H "Content-Type: application/octet-stream" \
//!   -H "X-Chaincode-Id: asset_transfer" \
//!   -H "X-Chaincode-Version: 1.0" \
//!   --data-binary @target/wasm32-unknown-unknown/release/examples/asset_transfer.wasm
//! ```

use chaincode_sdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Asset {
    id: String,
    owner: String,
    value: u64,
}

/// Create a new asset.
///
/// Invoke with function name "create_asset".
/// Reads asset data from state key "input:create_asset".
#[no_mangle]
pub extern "C" fn create_asset() -> i32 {
    // In a real implementation, the input would come from the transaction
    // proposal. For this example, we use a convention where the gateway
    // writes input to a known key before invoking.
    let input = match state_get("input:create_asset") {
        Some(data) => data,
        None => return -1,
    };

    let asset: Asset = match serde_json::from_slice(&input) {
        Ok(a) => a,
        Err(_) => return -1,
    };

    // Check if asset already exists
    if state_get(&format!("asset:{}", asset.id)).is_some() {
        return -1; // asset already exists
    }

    // Store the asset
    let key = format!("asset:{}", asset.id);
    if state_put_json(&key, &asset).is_err() {
        return -1;
    }

    // Emit event
    let _ = emit_event(
        "AssetCreated",
        format!("id={},owner={},value={}", asset.id, asset.owner, asset.value).as_bytes(),
    );

    0
}

/// Read an asset by ID.
///
/// Invoke with function name "read_asset".
/// Reads the asset ID from state key "input:read_asset".
#[no_mangle]
pub extern "C" fn read_asset() -> i32 {
    let input = match state_get("input:read_asset") {
        Some(data) => data,
        None => return -1,
    };

    let asset_id = match core::str::from_utf8(&input) {
        Ok(s) => s.trim(),
        Err(_) => return -1,
    };

    let key = format!("asset:{asset_id}");
    match state_get(&key) {
        Some(data) => {
            set_response(&data);
            0
        }
        None => -1,
    }
}

/// Transfer asset ownership.
///
/// Invoke with function name "transfer_asset".
/// Reads transfer data from state key "input:transfer_asset" as JSON:
/// `{"asset_id": "...", "new_owner": "..."}`
#[no_mangle]
pub extern "C" fn transfer_asset() -> i32 {
    let input = match state_get("input:transfer_asset") {
        Some(data) => data,
        None => return -1,
    };

    #[derive(Deserialize)]
    struct TransferInput {
        asset_id: String,
        new_owner: String,
    }

    let transfer: TransferInput = match serde_json::from_slice(&input) {
        Ok(t) => t,
        Err(_) => return -1,
    };

    let key = format!("asset:{}", transfer.asset_id);
    let mut asset: Asset = match state_get_json(&key) {
        Some(a) => a,
        None => return -1,
    };

    let old_owner = asset.owner.clone();
    asset.owner = transfer.new_owner.clone();

    if state_put_json(&key, &asset).is_err() {
        return -1;
    }

    let _ = emit_event(
        "AssetTransferred",
        format!(
            "id={},from={},to={}",
            transfer.asset_id, old_owner, transfer.new_owner
        )
        .as_bytes(),
    );

    0
}

/// Get asset history.
///
/// Invoke with function name "asset_history".
#[no_mangle]
pub extern "C" fn asset_history() -> i32 {
    let input = match state_get("input:asset_history") {
        Some(data) => data,
        None => return -1,
    };

    let asset_id = match core::str::from_utf8(&input) {
        Ok(s) => s.trim(),
        Err(_) => return -1,
    };

    let key = format!("asset:{asset_id}");
    match history_for_key(&key) {
        Some(history) => {
            set_response(&history);
            0
        }
        None => -1,
    }
}
