You are a senior Rust systems engineer and DevOps engineer.

Your task is to take an already functional Rust blockchain (E2E with PQC ML-DSA-65 validated) and make it **demo-ready, reproducible, and defensible**.

## CONTEXT

The system already works end-to-end:

* Transactions submit → commit → query by tx_id
* Smart contracts (WASM kv_store set/get)
* World state (RocksDB)
* PQC signing (ML-DSA-65) validated, no fallback
* API endpoints exist

However, there are **quality gaps**:

1. 7 failing tests related to RocksDB
2. No reliable Docker Compose demo (only `cargo run`)
3. Multi-node behavior not explicitly proven
4. Restart persistence not validated
5. No minimal explorer endpoints standardized
6. README is not sufficient for 10-minute onboarding

## OBJECTIVE

Deliver a **fully reproducible demo environment** that any developer can run with:

```bash
docker compose up
```

and validate:

* multi-node consensus consistency
* PQC-signed transactions
* smart contract execution
* queryability from any node
* persistence across restarts

## STRICT RULES

* DO NOT redesign the system
* DO NOT introduce new frameworks
* DO NOT over-engineer
* ONLY fix, connect, and validate
* Keep changes minimal and explicit
* Fail loudly on critical issues

---

## STEP 1 — FIX ROCKSDB TEST FAILURES

Goal:

* Reduce failing tests from 7 → 0

Tasks:

* Identify root cause of each failing test
* Common issues to check:

  * path collisions
  * temp dir reuse
  * missing cleanup
  * concurrent access
* Ensure each test uses isolated temp DB

Output:

* list of failing tests
* fixes applied
* final: `cargo test` → all pass

---

## STEP 2 — DOCKER COMPOSE (CRITICAL)

Goal:

* One command reproducible demo

Requirements:

Services:

* node1 (peer + orderer)
* node2 (peer)
* node3 (peer)
* orderer cluster (if separate)
* optional: prometheus (can skip TLS fix for now)

Each node must:

* expose API port
* mount persistent volume
* read SIGNING_ALGORITHM env var

Deliver:

* docker-compose.yml
* Dockerfile (multi-stage if needed)

Validation:

```bash
docker compose up
```

Expected:

* all nodes healthy
* logs show:

  * algorithm: ml-dsa-65
  * block commits

---

## STEP 3 — MULTI-NODE CONSISTENCY PROOF

Goal:

* Prove network is NOT single-node illusion

Test:

1. Submit tx to node1:
   POST /submit
   tx_id: demo-multi-node

2. Query from:

   * node1
   * node2
   * node3

All must return:

* same block_height
* same tx_id
* same state

Also verify:

* block hash identical across nodes

Deliver:

* script (bash or curl)
* expected outputs

---

## STEP 4 — RESTART PERSISTENCE

Goal:

* Ensure durability

Test:

1. Submit tx: demo-persistence
2. Stop containers:
   docker compose down
3. Restart:
   docker compose up
4. Query:
   GET /tx/demo-persistence

Must return:

* state: committed
* correct block_height

Ensure:

* RocksDB volumes persist

---

## STEP 5 — MINIMAL EXPLORER API

Standardize endpoints:

* GET /health
* GET /blocks
* GET /blocks/{height}
* GET /tx/{tx_id}

Requirements:

* JSON clean
* consistent schema
* no internal debug noise

---

## STEP 6 — README (10-MINUTE RULE)

Write a README that includes:

1. What this is (2 lines max)
2. Requirements
3. Run:
   docker compose up
4. Test commands:

   * submit tx
   * query tx
   * run smart contract
5. Expected output examples
6. Note:
   "PQC ML-DSA-65 enabled demo"

The README must allow a developer to run everything in ≤10 minutes.

---

## OUTPUT FORMAT

Return:

1. Files changed
2. docker-compose.yml
3. test fixes summary
4. commands to validate each step
5. expected outputs
6. any remaining known limitation

---

## SUCCESS CRITERIA

* `cargo test` → 100% pass
* `docker compose up` works without manual steps
* tx visible from ALL nodes
* restart does NOT lose state
* PQC signing confirmed in logs
* demo reproducible by third party

Start with STEP 1.
