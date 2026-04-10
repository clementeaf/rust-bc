# Chaincode SDK

Write smart contracts in Rust that compile to WebAssembly and run on the rust-bc blockchain.

## Setup

Add the SDK to your chaincode project:

```toml
[package]
name = "my-chaincode"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
chaincode-sdk = { path = "../chaincode-sdk" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

Install the Wasm target:

```bash
rustup target add wasm32-unknown-unknown
```

## Write a contract

```rust
use chaincode_sdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Asset {
    owner: String,
    value: u64,
}

#[no_mangle]
pub extern "C" fn create() -> i32 {
    let asset = Asset { owner: "alice".into(), value: 100 };
    state_put_json("asset:1", &asset).unwrap();
    emit_event("Created", b"asset:1").unwrap();
    0
}

#[no_mangle]
pub extern "C" fn read() -> i32 {
    match state_get("asset:1") {
        Some(data) => { set_response(&data); 0 }
        None => -1
    }
}

#[no_mangle]
pub extern "C" fn transfer() -> i32 {
    let mut asset: Asset = state_get_json("asset:1").unwrap();
    asset.owner = "bob".into();
    state_put_json("asset:1", &asset).unwrap();
    emit_event("Transferred", b"asset:1,to:bob").unwrap();
    0
}
```

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled Wasm file is at `target/wasm32-unknown-unknown/release/my_chaincode.wasm`.

## Deploy

```bash
# Install chaincode
curl -X POST https://localhost:8080/api/v1/chaincode/install \
  -H "Content-Type: application/octet-stream" \
  -H "X-Chaincode-Id: my_cc" \
  -H "X-Chaincode-Version: 1.0" \
  --data-binary @target/wasm32-unknown-unknown/release/my_chaincode.wasm

# Approve (per org)
curl -X POST https://localhost:8080/api/v1/chaincode/my_cc/approve?version=1.0 \
  -H "X-Org-Id: org1"

# Commit
curl -X POST https://localhost:8080/api/v1/chaincode/my_cc/commit?version=1.0
```

## Invoke

```bash
# Call the "create" function
curl -X POST https://localhost:8080/api/v1/chaincode/my_cc/invoke \
  -H "Content-Type: application/json" \
  -d '{"function": "create"}'

# Call the "read" function
curl -X POST https://localhost:8080/api/v1/chaincode/my_cc/invoke \
  -H "Content-Type: application/json" \
  -d '{"function": "read"}'
```

## API Reference

### State operations

| Function | Description |
|---|---|
| `state_put(key, value)` | Write raw bytes to world state |
| `state_get(key)` | Read raw bytes from world state |
| `state_put_json(key, &value)` | Serialize to JSON and write |
| `state_get_json::<T>(key)` | Read and deserialize from JSON |

### Events

| Function | Description |
|---|---|
| `emit_event(name, payload)` | Emit a chaincode event (delivered via WebSocket) |

### Advanced

| Function | Description |
|---|---|
| `set_key_policy(key, policy_json)` | Override endorsement policy for a key |
| `history_for_key(key)` | Get version history for a key |
| `invoke(chaincode_id, function)` | Call another chaincode |
| `set_response(data)` | Set the return value for the caller |

## Examples

See `examples/asset_transfer.rs` for a complete asset management contract.

Build the example:

```bash
cargo build --example asset_transfer --target wasm32-unknown-unknown --release
```
