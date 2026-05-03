You are a senior distributed systems engineer and cryptography engineer.

Your task is to take an existing Rust-based blockchain system and CONNECT (not redesign) the missing pieces required to produce a fully functional end-to-end distributed demo.

## CONTEXT

The system already has:

* Gateway pipeline: submit → endorse → order → commit (currently single-node)
* Multiple nodes (peers + orderers) running in containers
* Raft nodes running but NOT integrated into commit flow
* RocksDB storage per node
* DID / identity module working
* API endpoints exposed
* Wallet exists but PQC signing is NOT wired into the gateway
* Smart contract system scaffold exists but no working deployment example

## CURRENT PROBLEMS (DO NOT IGNORE)

1. Transactions are committed locally only (no distributed consensus)
2. Raft cluster is running but not used in ordering
3. Peers do NOT replicate blocks (isolated state per node)
4. PQC signatures (ML-DSA-65) are NOT applied (blocks contain zero signatures)
5. No tx indexing → cannot query by tx_id after commit
6. Smart contracts cannot be deployed (no WASM example / pipeline)
7. Prometheus cannot scrape nodes due to TLS issues (non-critical for now)

## OBJECTIVE

Produce a MINIMAL but REAL distributed system demo that satisfies:

* A transaction submitted from node1:

  * is signed with PQC (ML-DSA-65)
  * is ordered through Raft (leader-based)
  * is committed to multiple peers (at least 2 nodes)
  * is queryable by tx_id from ANY node

## CONSTRAINTS

* DO NOT redesign architecture
* DO NOT introduce new frameworks
* DO NOT over-engineer
* ONLY connect existing components
* Keep changes minimal and incremental
* Prefer clarity over abstraction

## PRIORITY ORDER (STRICT)

### STEP 1 — PQC SIGNATURE INTEGRATION

* Wire wallet → gateway → block signing
* Ensure blocks contain valid ML-DSA-65 signatures
* Add verification step on commit

### STEP 2 — RAFT INTEGRATION

* Route ordering through Raft leader
* Ensure:

  * proposal → leader
  * replication → followers
  * commit after quorum

### STEP 3 — PEER REPLICATION

* Broadcast committed blocks to peers
* Ensure peers:

  * validate block
  * persist to RocksDB
  * update height consistently

### STEP 4 — TX INDEXING

* Store tx_id → block reference mapping
* Implement API:
  GET /tx/{tx_id}

### STEP 5 — MINIMAL SMART CONTRACT

* Deploy a simple WASM contract:
  function: set(key, value)
  function: get(key)
* Ensure execution works through existing pipeline

## OUTPUT FORMAT

For EACH step, provide:

1. Explanation (short, direct)
2. Exact code changes (Rust)
3. Files/modules affected
4. How to test (curl or CLI)
5. Expected output

## SUCCESS CRITERIA

* Submit tx → appears in multiple nodes
* tx_id can be queried from any node
* block signature is valid (NOT zeroed)
* Raft leader is involved in ordering
* system still runs in Docker environment

## STYLE

* Be surgical
* No generic explanations
* No theory unless necessary
* Focus on making it WORK

Start with STEP 1.
