You are a senior Rust blockchain engineer and DevOps reliability auditor.

Your task is to take the current Cerulean Ledger (post-quantum L1 testnet) from:

👉 “functional + presentable testnet”
to
👉 “production-grade testnet-ready (pre-mainnet hardening)”

The system already has:

* PQC cryptography (ML-DSA + ML-KEM)
* account model (balances + nonces)
* signed transactions
* fee-ordered mempool
* block production + persistence
* P2P propagation
* adversarial transaction protections
* CLI wallet
* explorer UI
* whitepaper
* 1500+ tests passing

Now your goal is to implement the **remaining production-grade engineering layer**.

---

# 🎯 OBJECTIVE

Close all remaining **code-level gaps before real testnet exposure**:

* correctness under restart and sync
* economic state consistency
* API reliability
* CLI correctness
* observability
* reproducibility

---

# 📦 TARGET AREAS

You MUST implement and test the following 8 areas:

---

# 1. Wallet CLI test suite

## Goal

Ensure CLI is deterministic, safe, and reproducible.

## Create

```text
tests/wallet_cli.rs
```

## Cover

* key generation → valid format
* address derivation → matches hash(pubkey)
* signing → produces valid signature
* transfer → accepted by mempool
* invalid signature → rejected
* wrong chain_id → rejected

## Requirement

CLI must be scriptable:

```bash
wallet generate
wallet address <pubkey>
wallet sign <tx.json>
wallet send <signed_tx.json>
```

---

# 2. API / Explorer correctness tests

## Create

```text
tests/api_endpoints.rs
```

## Cover

* GET /accounts/{address}
* POST /transfer
* GET /mempool/stats
* GET /blocks/{height}
* GET /tx/{hash}

## Validate

* JSON schema
* status codes
* invalid input handling
* error messages (no panics)

---

# 3. Persistent AccountStore

## Goal

Remove reliance on in-memory account state.

## Implement

```text
RocksDbAccountStore
```

## Requirements

* atomic updates
* crash-safe writes
* consistent reads after restart
* snapshot capability (optional)

## Tests

```text
tests/account_persistence.rs
```

* write → restart → read same balances
* multiple txs → restart → state identical
* concurrent updates safe

---

# 4. Full node sync from genesis

## Goal

A new node must reconstruct state from blocks.

## Implement

* sync_blocks_from_peer()
* replay execution from genesis

## Tests

```text
tests/node_sync.rs
```

* Node A produces 100 blocks
* Node B joins empty
* B syncs from A
* assert:

  * same height
  * same state_hash
  * same balances

---

# 5. Fork-choice / reorg safety

## Goal

Handle competing chains correctly.

## Implement

* canonical chain selection rule
* rollback + reapply blocks

## Tests

```text
tests/reorg.rs
```

Scenarios:

* fork at height N
* longer chain replaces shorter
* balances revert and reapply correctly
* no double-spend leakage

---

# 6. Faucet hardening

## Modify

```text
src/api/handlers/faucet.rs
```

## Add

* per-address limit
* per-IP limit
* cooldown window
* max daily distribution

## Tests

```text
tests/faucet_limits.rs
```

---

# 7. Observability

## Add

* structured logging (tracing)
* metrics endpoint (/metrics)

## Metrics

* tx/sec
* mempool size
* block time
* failed tx count
* rejected signatures

## Health

```text
GET /health
```

returns:

```json
{
  "status": "ok",
  "height": 123,
  "peers": 3
}
```

---

# 8. Release packaging

## Create

```text
docker/
├── Dockerfile.node
├── Dockerfile.api
└── docker-compose.testnet.yml
```

## Requirements

* reproducible builds
* env-based config
* multi-node startup
* volume persistence

## Script

```text
scripts/release_testnet.sh
```

---

# 🧪 GLOBAL VALIDATION

After implementation, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

AND additionally:

```bash
cargo test --test wallet_cli
cargo test --test api_endpoints
cargo test --test account_persistence
cargo test --test node_sync
cargo test --test reorg
cargo test --test faucet_limits
```

---

# 📊 SUCCESS CRITERIA

All must be true:

* no panics in API
* full restart safety
* deterministic state after sync
* fork handling correct
* CLI reproducible
* faucet cannot be abused
* metrics available
* testnet runs via Docker

---

# 🧾 FINAL OUTPUT FORMAT

Report:

1. Files created/modified
2. Tests added (count)
3. Persistence backend used
4. Sync correctness verified (yes/no)
5. Reorg safety verified (yes/no)
6. Faucet abuse prevented (yes/no)
7. Observability endpoints added
8. Docker/testnet packaging status
9. Total tests passing
10. Final statement:

```text
Cerulean Ledger is now production-grade testnet-ready.
```

---

# 🧠 MINDSET

This is NOT feature development.

This is **system hardening**.

Focus on:

* determinism
* reproducibility
* failure safety
* state correctness

No shortcuts.
