You are a distributed systems chaos engineer and Rust security auditor.

Your task is to DESIGN and IMPLEMENT a **realistic adversarial network stress test framework** for a Rust-based Blockchain DLT that already passes all PQC security tests.

This is NOT a unit test task.
This is a **multi-node, failure-injected, convergence validation system**.

---

## 🎯 OBJECTIVE

Prove that the DLT:

* Maintains **consensus integrity under adversarial conditions**
* Enforces **PQC cryptography correctly across nodes**
* Resists **downgrade, replay, partition, and malicious peers**
* **Converges to a single valid state** after chaos

---

## 🧠 TEST ARCHITECTURE

Build a test harness that spins up **N in-process nodes** (recommended: 5–10 nodes), each with:

* Independent state
* Independent config (env vars may differ intentionally)
* P2P communication (simulate network layer, not just direct calls)

Use async runtime (tokio).

---

## ⚙️ FAULT INJECTION CAPABILITIES

You MUST implement the following:

### 1. Network Conditions

* Random latency (0ms – 500ms)
* Message reordering
* Packet loss (drop % configurable)
* Duplicate delivery

### 2. Node Failures

* Crash node (stop processing)
* Restart node (recover from peers)
* State desync simulation

### 3. Network Partitions

* Split nodes into groups A / B
* Allow:

  * A cannot see B
  * B CAN see A (asymmetric partition)
* Heal partition later

### 4. Adversarial Nodes

Create malicious nodes that:

* Send blocks with invalid PQC signatures
* Send blocks with mismatched algorithm tags
* Attempt classical-only signatures when PQC required
* Attempt TLS downgrade (simulate handshake failure or fallback)
* Replay old valid messages
* Send malformed or random bytes

---

## 🔐 PQC VALIDATION REQUIREMENTS

System is configured with:

REQUIRE_PQC_SIGNATURES=true
TLS_PQC_KEM=true
DUAL_SIGN_VERIFY_MODE=both
HASH_ALGORITHM=sha3-256

You must verify:

* No node EVER accepts classical-only signatures
* No node EVER accepts mismatched algorithm tags
* No node EVER accepts invalid PQC signatures
* No node EVER downgrades TLS silently
* All nodes enforce same PQC policy

---

## 🧪 REQUIRED TEST SCENARIOS

### Scenario 1: Normal operation

* All nodes online
* Produce blocks
* Verify full consensus convergence

---

### Scenario 2: Malicious node injection

* 1–2 nodes send invalid PQC blocks
* Ensure:

  * blocks rejected
  * no propagation
  * network remains consistent

---

### Scenario 3: Network partition + healing

* Split network into 2 groups
* Both produce blocks
* Heal network
* Verify:

  * only valid chain survives
  * no invalid PQC block enters final state

---

### Scenario 4: Replay attack

* Capture valid messages
* Replay after state progressed
* Ensure rejection

---

### Scenario 5: Downgrade attempt

* Malicious node attempts classical TLS / signature
* Ensure:

  * connection rejected OR flagged
  * no state contamination

---

### Scenario 6: Node crash + recovery

* Kill node mid-sync
* Restart it
* Ensure:

  * it syncs correctly
  * does not accept invalid history

---

### Scenario 7: Mixed configuration (critical)

* Some nodes:
  REQUIRE_PQC_SIGNATURES=false (simulate misconfig)
* Others:
  REQUIRE_PQC_SIGNATURES=true

Verify:

* secure nodes NEVER accept invalid blocks
* network does NOT converge to insecure state

---

## 📊 ASSERTIONS (MANDATORY)

At the end of each scenario:

* All honest nodes have IDENTICAL state hash
* No invalid block exists in final chain
* No node accepted a block violating PQC policy
* Chain is valid under strict PQC verification

---

## 🧾 OUTPUT REQUIREMENTS

You must produce:

1. `tests/chaos_network.rs`
2. Node harness implementation (TestNodeCluster or similar)
3. Fault injection module
4. Logs showing:

   * rejected attacks
   * partition events
   * convergence result
5. Clear assertions per scenario

---

## ⚠️ IMPORTANT RULES

* Do NOT mock cryptography — use real PQC logic
* Do NOT simplify validation — use real consensus paths
* Fail fast if ANY invalid block is accepted
* Prefer deterministic randomness (seeded RNG)

---

## 🧠 MINDSET

Assume the system is under attack.

Your job is to BREAK it.

If it survives, prove it.

---

## 🧾 FINAL OUTPUT FORMAT

* Summary of scenarios executed
* Any vulnerabilities found
* Code for test harness
* Key logs demonstrating behavior
* Final verdict: SAFE / NOT SAFE under adversarial network

---

Be precise. Be adversarial. Be exhaustive.
