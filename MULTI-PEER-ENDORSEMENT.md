# Multi-Peer Endorsement RPC

Implementation plan to make rust-bc's Gateway collect endorsements from multiple org peers via P2P before submitting to the orderer — the core architectural feature that defines Hyperledger Fabric.

## Current state

The Gateway **self-endorses**: it checks the local org registry against the endorsement policy, but never actually sends the transaction to remote peers for simulation. All the data structures already exist:

| Component | Status | Location |
|---|---|---|
| `EndorsedTransaction` (proposal + endorsements + rwset) | Exists | `src/transaction/endorsed.rs:7-13` |
| `TransactionProposal` (tx + creator sig + rwset) | Exists | `src/transaction/proposal.rs:7-15` |
| `ProposalResponse` (rwset + endorsement) | Exists | `src/transaction/proposal.rs:17-22` |
| `Endorsement` (signer_did, org_id, Ed25519 sig, payload_hash) | Exists | `src/endorsement/types.rs:35-50` |
| `ReadWriteSet` (reads + writes, derives `PartialEq + Eq`) | Exists | `src/transaction/rwset.rs:17-28` |
| `WorldState` trait (`get`/`put`/`delete`/`get_range`/`get_history`) + `MemoryWorldState` | Exists | `src/storage/world_state.rs:24-42` |
| `SigningProvider` trait (`sign`/`public_key`/`verify`) + `SoftwareSigningProvider` (Ed25519) | Exists | `src/identity/signing.rs:20-60` |
| `WasmExecutor::simulate()` → `(result, rwset)` | Works | `src/chaincode/executor.rs:506-516` |
| `DiscoveryService::endorsement_plan()` → `Vec<PeerDescriptor>` | Works | `src/discovery/service.rs:107-144` |
| `validate_endorsements()` | Works | `src/endorsement/validator.rs` |

**What's missing:** P2P message types for proposal request/response, peer-side simulation handler, P2P send primitive (current layer is fire-and-forget with no `send_to_peer` method), request-response correlation pattern, and Gateway orchestration to collect endorsements from multiple peers.

---

## Target flow (matches Fabric 2.5)

```
Client                  Gateway (node1)           Peer (node2, org2)         Peer (node3, org1)
  |                         |                          |                          |
  |-- POST /gateway/submit -->                         |                          |
  |                         |                          |                          |
  |                   1. Build TransactionProposal     |                          |
  |                   2. Query endorsement_plan()      |                          |
  |                      → [node2:8083, node3:8085]    |                          |
  |                         |                          |                          |
  |                   3. Send ProposalRequest via P2P  |                          |
  |                         |--- ProposalRequest ----->|                          |
  |                         |--- ProposalRequest --------------------------->|
  |                         |                          |                          |
  |                         |                    4. Simulate Wasm           4. Simulate Wasm
  |                         |                       (own world state)          (own world state)
  |                         |                    5. Sign rwset hash         5. Sign rwset hash
  |                         |                          |                          |
  |                         |<-- ProposalResponse -----|                          |
  |                         |<-- ProposalResponse ----------------------------|
  |                         |                          |                          |
  |                   6. Compare rwsets (must match)    |                          |
  |                   7. Validate endorsement policy    |                          |
  |                   8. Build EndorsedTransaction      |                          |
  |                   9. Submit to OrderingService      |                          |
  |                  10. Cut block, MVCC, commit        |                          |
  |                         |                          |                          |
  |<-- TxResult (tx_id, height, valid) --              |                          |
```

---

## Implementation steps

### Step 1: P2P message types

Add two new variants to `Message` enum in `src/network/mod.rs:28-68`:

```rust
/// Peer receives this, simulates chaincode, returns ProposalResponse.
ProposalRequest {
    /// Unique ID to correlate request/response.
    request_id: String,
    /// Chaincode to simulate.
    chaincode_id: String,
    /// Function to invoke (e.g. "invoke", "run").
    function: String,
    /// Channel context.
    channel_id: String,
    /// The transaction proposal from the client.
    proposal: crate::transaction::proposal::TransactionProposal,
},

/// Peer sends this back after simulation.
ProposalResponse {
    request_id: String,
    /// The rwset produced by simulation.
    rwset: crate::transaction::rwset::ReadWriteSet,
    /// This peer's endorsement (org_id + signature over rwset hash).
    endorsement: crate::endorsement::types::Endorsement,
    /// Simulation result bytes (optional, for chaincode return value).
    result: Vec<u8>,
},
```

**Files:** `src/network/mod.rs`
**Effort:** Small

---

### Step 2: Peer-side proposal handler

In `src/network/mod.rs`, handle `ProposalRequest` in `process_message()`:

```rust
Message::ProposalRequest { request_id, chaincode_id, function, channel_id, proposal } => {
    // 1. Load the chaincode Wasm package from local store
    // 2. Create WasmExecutor for this chaincode
    // 3. Simulate: executor.simulate(world_state, &function)
    // 4. Sign the rwset hash with this node's Ed25519 key
    // 5. Build Endorsement { signer_did, org_id, signature, payload_hash, timestamp }
    // 6. Return Message::ProposalResponse { request_id, rwset, endorsement, result }
    Ok(Some(Message::ProposalResponse { ... }))
}
```

**Dependencies:**
- Node needs access to `chaincode_package_store` to load Wasm by chaincode_id
- Node needs access to `world_state` for simulation (trait already exists at `src/storage/world_state.rs:24`)
- Node needs a `SigningProvider` to produce endorsements (trait already exists at `src/identity/signing.rs:20`)
- `org_id` already exists on `Node` as `pub org_id: String` (non-optional)

**What to add to `Node` struct** (at `src/network/mod.rs:82`):

```rust
pub struct Node {
    // ... existing fields (including org_id: String) ...
    /// Chaincode store for loading Wasm modules during endorsement.
    pub chaincode_store: Option<Arc<dyn ChaincodePackageStore>>,
    /// World state for simulation (`src/storage/world_state.rs`).
    pub world_state: Option<Arc<dyn WorldState>>,
    /// Signing provider for endorsements (`src/identity/signing.rs`).
    /// Uses the existing `SigningProvider` trait — not a raw `ed25519_dalek::SigningKey`.
    pub signing_provider: Option<Arc<dyn SigningProvider>>,
}
```

> **Note:** `org_id` is already a field on `Node` (`pub org_id: String`). No need to add it.

**Files:** `src/network/mod.rs`
**Effort:** Medium

---

### Step 3: Gateway endorsement orchestrator

Replace `self_endorse()` in `Gateway::submit()` with actual peer-to-peer endorsement collection.

New method in `src/gateway/mod.rs`:

```rust
/// Collect endorsements from remote peers for a transaction proposal.
///
/// 1. Query discovery for required endorsers.
/// 2. Send ProposalRequest to each peer via P2P.
/// 3. Wait for ProposalResponse from each (with timeout).
/// 4. Validate all rwsets match (deterministic execution guarantee).
/// 5. Validate endorsement policy is satisfied.
/// 6. Return the collected endorsements + shared rwset.
fn collect_endorsements(
    &self,
    chaincode_id: &str,
    channel_id: &str,
    proposal: &TransactionProposal,
) -> Result<(ReadWriteSet, Vec<Endorsement>), GatewayError> {
    let endorsers = self.discovery_service
        .as_ref()
        .ok_or(GatewayError::PolicyNotSatisfied("no discovery service".into()))?
        .endorsement_plan(chaincode_id, channel_id)
        .map_err(|e| GatewayError::PolicyNotSatisfied(e.to_string()))?;

    // Send ProposalRequest to each endorser peer via P2P
    let mut responses: Vec<ProposalResponse> = Vec::new();
    for peer in &endorsers {
        let response = self.p2p_node
            .send_and_wait(&peer.peer_address, Message::ProposalRequest {
                request_id: uuid::Uuid::new_v4().to_string(),
                chaincode_id: chaincode_id.to_string(),
                function: "invoke".to_string(),
                channel_id: channel_id.to_string(),
                proposal: proposal.clone(),
            }, ENDORSEMENT_TIMEOUT)
            .map_err(|e| GatewayError::PolicyNotSatisfied(
                format!("peer {} failed: {}", peer.peer_address, e)
            ))?;

        match response {
            Message::ProposalResponse { rwset, endorsement, .. } => {
                responses.push(ProposalResponse { rwset, endorsement });
            }
            _ => return Err(GatewayError::PolicyNotSatisfied(
                format!("unexpected response from {}", peer.peer_address)
            )),
        }
    }

    // All rwsets must match (deterministic simulation)
    let reference_rwset = &responses[0].rwset;
    for resp in &responses[1..] {
        if resp.rwset != *reference_rwset {
            return Err(GatewayError::Simulation(
                "rwset mismatch between endorsers — non-deterministic chaincode".into()
            ));
        }
    }

    let endorsements: Vec<Endorsement> = responses.iter()
        .map(|r| r.endorsement.clone())
        .collect();

    Ok((reference_rwset.clone(), endorsements))
}
```

**New fields needed on Gateway:**

```rust
pub struct Gateway {
    // ... existing fields ...
    /// P2P node handle for sending endorsement requests to remote peers.
    pub p2p_node: Option<Arc<Node>>,
}
```

**Files:** `src/gateway/mod.rs`
**Effort:** Medium

---

### Step 4: P2P request-response pattern

The current P2P layer is fire-and-forget and **has no `send_to_peer` method at all** — messages are only sent inline during `process_message()` as responses. This step requires implementing both the basic send primitive and the request-response correlation on top of it:

```rust
impl Node {
    /// Send a message to a peer and wait for a correlated response.
    ///
    /// Uses `request_id` to match response to request.
    /// Returns Err on timeout (default: 5s).
    pub async fn send_and_wait(
        &self,
        peer_address: &str,
        message: Message,
        timeout: Duration,
    ) -> Result<Message, NetworkError> {
        // 1. Register a oneshot channel keyed by request_id
        // 2. Send the message to the peer
        // 3. Await the oneshot receiver with timeout
        // 4. When ProposalResponse arrives in process_message(),
        //    look up the oneshot sender by request_id and deliver
    }
}
```

**Implementation approach:**

```rust
// In Node struct:
pub pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<Message>>>>,

// In send_and_wait:
let (tx, rx) = tokio::sync::oneshot::channel();
self.pending_requests.lock().unwrap().insert(request_id.clone(), tx);
self.send_to_peer(peer_address, &message).await?;
tokio::time::timeout(timeout, rx).await
    .map_err(|_| NetworkError::Timeout)?
    .map_err(|_| NetworkError::ChannelClosed)

// In process_message, when receiving ProposalResponse:
Message::ProposalResponse { ref request_id, .. } => {
    if let Some(sender) = self.pending_requests.lock().unwrap().remove(request_id) {
        let _ = sender.send(msg.clone());
        Ok(None)  // Already handled via oneshot
    } else {
        Ok(None)  // Stale response, ignore
    }
}
```

**Files:** `src/network/mod.rs`
**Effort:** Medium — this is the trickiest part (async request-response over TCP)

---

### Step 5: Wire Gateway::submit() to use multi-peer endorsement

Modify `Gateway::submit()` to use the new flow:

```rust
pub fn submit(
    &self,
    chaincode_id: &str,
    channel_id: &str,
    tx: Transaction,
) -> Result<TxResult, GatewayError> {
    // ── Step 1: Build proposal ──────────────────────────────────────────
    let proposal = TransactionProposal {
        tx: tx.clone(),
        creator_did: String::new(), // TODO: from caller identity
        creator_signature: [0u8; 64], // TODO: sign with caller key
        rwset: ReadWriteSet::default(),
    };

    // ── Step 2: Collect endorsements ────────────────────────────────────
    let (rwset, endorsements) = if self.p2p_node.is_some() && self.discovery_service.is_some() {
        // Multi-peer path: send to remote endorsers
        self.collect_endorsements(chaincode_id, channel_id, &proposal)?
    } else if let (Some(exec), Some(ws)) = (&self.wasm_executor, &self.world_state) {
        // Single-node fallback: simulate locally
        let (_, rwset) = exec.simulate(Arc::clone(ws), "invoke")
            .map_err(|e| GatewayError::Simulation(e.to_string()))?;
        self.validate_key_policies_for_rwset(chaincode_id, &rwset)?;
        (rwset, vec![]) // No remote endorsements in single-node mode
    } else {
        // No simulation: policy-only check
        self.self_endorse(chaincode_id)?;
        (ReadWriteSet::default(), vec![])
    };

    // ── Step 3: Build EndorsedTransaction ───────────────────────────────
    let endorsed_tx = EndorsedTransaction {
        proposal: TransactionProposal { rwset: rwset.clone(), ..proposal },
        endorsements,
        rwset: rwset.clone(),
    };

    // ── Step 4: Submit to ordering ──────────────────────────────────────
    self.ordering_service.submit_endorsed_tx(endorsed_tx)
        .map_err(|e| GatewayError::Ordering(e.to_string()))?;

    // ── Step 5: Cut block, MVCC, commit, events (same as today) ────────
    // ...
}
```

**Files:** `src/gateway/mod.rs`
**Effort:** Small (mostly restructuring existing code)

---

### Step 6: OrderingService accepts EndorsedTransaction

Add a variant that preserves endorsement metadata through ordering:

```rust
// In src/ordering/service.rs:
pub fn submit_endorsed_tx(&self, etx: EndorsedTransaction) -> StorageResult<()> {
    // Store the full EndorsedTransaction so validation can happen at commit time
    self.pending_txs.lock().unwrap().push_back(etx.proposal.tx.clone());
    // Optionally store endorsements for later validation
    Ok(())
}
```

For Fabric-accurate behavior, the orderer should carry the `EndorsedTransaction` through to the block, so committer peers can re-validate endorsements. For MVP, preserving the bare `Transaction` is sufficient since Gateway already validated.

**Files:** `src/ordering/service.rs`
**Effort:** Small

---

### Step 7: Node initialization wiring

In `src/main.rs`, pass the required resources to the P2P node:

```rust
// After creating the node (org_id is already set during Node construction):
node.chaincode_store = Some(chaincode_package_store.clone());
node.world_state = Some(world_state.clone());
node.signing_provider = Some(Arc::new(
    SoftwareSigningProvider::generate()  // or load from persisted key material
));

// Pass node to Gateway:
gateway.p2p_node = Some(node_arc.clone());
```

**Files:** `src/main.rs`
**Effort:** Small

---

## Dependency order

```
Step 1 (Message types)
  └── Step 2 (Peer handler) ── requires chaincode_store + world_state on Node
  └── Step 4 (Request-response pattern) ── async, trickiest part
        └── Step 3 (Gateway orchestrator) ── uses send_and_wait
              └── Step 5 (Wire submit()) ── restructure existing flow
                    └── Step 6 (Ordering accepts endorsed TX)
                          └── Step 7 (main.rs wiring)
```

**Critical path:** Steps 1 → 4 → 3 → 5 (the P2P request-response pattern is the bottleneck)

---

## Testing strategy

### Unit tests (in each module)

| Test | What it verifies |
|---|---|
| `ProposalRequest` serializes/deserializes over P2P | Message roundtrip |
| Peer receives ProposalRequest, returns ProposalResponse with valid rwset | Peer-side handler |
| `send_and_wait` returns response within timeout | Request-response pattern |
| `send_and_wait` returns error on timeout | Timeout behavior |
| Gateway collects 2 endorsements, validates matching rwsets | Orchestrator happy path |
| Gateway rejects mismatched rwsets | Non-deterministic chaincode detection |
| Gateway falls back to self-endorse when no P2P node | Backwards compatibility |

### E2E tests (add to `scripts/e2e-test.sh`)

```bash
# ── Test 19: Multi-peer endorsement ──────────────────────────────
# Setup: install same chaincode on node1 + node2, register peers in discovery

# Install chaincode on node2
curl -X POST "$NODE2/api/v1/chaincode/install?chaincode_id=endorsed-cc&version=1.0" ...

# Register node2 as endorser
curl -X POST "$NODE1/api/v1/discovery/register" -d '{
    "peer_address": "node2:8083",
    "org_id": "org2",
    "chaincodes": ["endorsed-cc"],
    "channels": ["mychannel"]
}'

# Set policy requiring both orgs
curl -X POST "$NODE1/api/v1/store/policies" -d '{
    "resource_id": "mychannel/endorsed-cc",
    "policy": {"AllOf": ["org1", "org2"]}
}'

# Submit TX — Gateway should collect endorsements from both peers
resp=$(curl -X POST "$NODE1/api/v1/gateway/submit" -d '{
    "chaincode_id": "endorsed-cc",
    "channel_id": "mychannel",
    "transaction": { "id": "multi-endorse-1", ... }
}')

# Verify: response includes endorsement count
assert_eq $(echo $resp | jq '.data.endorsement_count') 2
assert_eq $(echo $resp | jq '.data.valid') true
```

---

## Effort estimate

| Step | Effort | Lines (approx) |
|---|---|---|
| 1. Message types | Small | ~20 |
| 2. Peer handler | Medium | ~80 |
| 3. Gateway orchestrator | Medium | ~100 |
| 4. P2P send + request-response pattern | Large | ~180 |
| 5. Wire submit() | Small | ~50 |
| 6. Ordering endorsed TX | Small | ~20 |
| 7. main.rs wiring | Small | ~15 |
| Tests | Medium | ~200 |
| **Total** | | **~665 lines** |

**Estimated calendar time:** 3-4 focused sessions.
