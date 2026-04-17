# Your First dApp

Build and deploy a simple token contract using Wasm chaincode.

## 1. Write the Contract (Rust)

Create a new Rust library:

```bash
cargo new --lib my_token
cd my_token
```

Add to `Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib"]
```

Write `src/lib.rs`:
```rust
#[no_mangle]
pub extern "C" fn init() -> i32 {
    // Called once when chaincode is instantiated
    0 // success
}

#[no_mangle]
pub extern "C" fn invoke() -> i32 {
    // Called on each transaction
    0 // success
}
```

## 2. Compile to Wasm

```bash
cargo build --target wasm32-unknown-unknown --release
```

The output is at `target/wasm32-unknown-unknown/release/my_token.wasm`.

## 3. Install on the Node

```bash
# Install chaincode package
curl -X POST http://localhost:8080/api/v1/chaincode/install \
  -F "file=@target/wasm32-unknown-unknown/release/my_token.wasm" \
  -F "id=my_token" \
  -F "version=1.0"
```

## 4. Approve and Commit

```bash
# Approve the definition
curl -X POST http://localhost:8080/api/v1/chaincode/approve \
  -H "Content-Type: application/json" \
  -d '{"chaincode_id": "my_token", "version": "1.0"}'

# Commit (activate on the channel)
curl -X POST http://localhost:8080/api/v1/chaincode/commit \
  -H "Content-Type: application/json" \
  -d '{"chaincode_id": "my_token", "version": "1.0"}'
```

## 5. Invoke

```bash
curl -X POST http://localhost:8080/api/v1/gateway/submit \
  -H "Content-Type: application/json" \
  -d '{
    "chaincode_id": "my_token",
    "channel_id": "default",
    "tx": {
      "id": "tx-001",
      "from": "alice",
      "to": "bob",
      "amount": 100
    }
  }'
```

## Using the Python SDK

```python
from rust_bc import Client

client = Client("http://localhost:8080")
result = client.submit_transaction(
    chaincode_id="my_token",
    channel_id="default",
    tx_id="tx-002",
    from_did="did:bc:alice",
    to_did="did:bc:bob",
    amount=50
)
print(f"Block height: {result.block_height}")
```

## Using the JavaScript SDK

```javascript
import { RustBcClient } from 'rust-bc-sdk';

const client = new RustBcClient('http://localhost:8080');
const result = await client.submitTransaction({
  chaincodeId: 'my_token',
  channelId: 'default',
  txId: 'tx-003',
  from: 'did:bc:alice',
  to: 'did:bc:bob',
  amount: 25
});
console.log(`Block: ${result.blockHeight}`);
```
