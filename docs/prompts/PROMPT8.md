You are a senior Rust performance engineer and blockchain protocol benchmark auditor.

Your task is to add a complete benchmark suite for this Rust DLT under strict PQC mode.

The system already passes security, chaos, persistence, DoS, equivocation, and slashing tests.
Now we need to measure production limits.

---

## Objective

Benchmark the cost and limits of:

* ML-DSA signing
* ML-DSA verification
* SHA3-256 hashing
* block validation
* RocksDB persistence
* consensus acceptance path
* gossip/flood rejection path
* full node throughput under realistic load

---

## Target files

Prefer creating:

`benches/pqc_performance.rs`

If needed, add test helpers under:

`tests/perf_helpers.rs`

Use Criterion if already available.
If not, add:

```toml
[dev-dependencies]
criterion = "0.5"
```

And configure:

```toml
[[bench]]
name = "pqc_performance"
harness = false
```

---

## Required config

Benchmark under:

```env
REQUIRE_PQC_SIGNATURES=true
TLS_PQC_KEM=true
DUAL_SIGN_VERIFY_MODE=both
HASH_ALGORITHM=sha3-256
SIGNING_ALGORITHM=ml-dsa-65
```

---

## Benchmarks to implement

### 1. `bench_mldsa_sign`

Measure:

* average signing time
* p95
* p99
* ops/sec

---

### 2. `bench_mldsa_verify`

Measure:

* average verification time
* p95
* p99
* verifications/sec

---

### 3. `bench_sha3_block_hash`

Measure SHA3-256 cost for:

* small block: 10 tx
* medium block: 100 tx
* large block: 1000 tx

---

### 4. `bench_block_validation_strict_pqc`

Measure full validation path:

```text
signature consistency
→ PQC policy
→ ML-DSA verify
→ hash check
→ consensus acceptance
```

Run for:

* 100 blocks
* 1,000 blocks
* 10,000 blocks

---

### 5. `bench_rocksdb_write_read_blocks`

Measure:

* block write latency
* block read latency
* batch write throughput
* restart/load time for 1k, 10k, 100k blocks if feasible

---

### 6. `bench_invalid_flood_rejection`

Measure rejection cost for:

* malformed block
* wrong signature size
* duplicate hash
* stale height
* rate-limited peer
* invalid ML-DSA signature

Important:

Cheap rejection should be much faster than ML-DSA verification.

---

### 7. `bench_full_node_throughput`

Simulate:

* 4 honest nodes
* valid block production
* strict PQC
* RocksDB persistence

Measure:

* accepted blocks/sec
* tx/sec
* average block latency
* p95 block latency
* p99 block latency

---

## Required output

Generate benchmark summary similar to:

```text
PQC Performance Report

ML-DSA sign:
  avg:
  p95:
  p99:
  ops/sec:

ML-DSA verify:
  avg:
  p95:
  p99:
  ops/sec:

Block validation strict PQC:
  blocks/sec:
  avg latency:
  p95:
  p99:

RocksDB:
  write avg:
  read avg:
  batch throughput:

Invalid flood rejection:
  malformed:
  size mismatch:
  duplicate:
  stale:
  rate-limited:
  invalid ML-DSA:

Full node throughput:
  blocks/sec:
  tx/sec:
  bottleneck:
```

---

## Performance guardrails

Add non-flaky threshold tests separately from Criterion.

Create:

`tests/performance_guardrails.rs`

Add tests like:

```rust
#[test]
fn cheap_rejection_is_at_least_10x_faster_than_mldsa_verify() {}

#[test]
fn duplicate_flood_does_not_trigger_unbounded_verification() {}

#[test]
fn rocksdb_restart_10k_blocks_under_reasonable_time() {}

#[test]
fn strict_pqc_validation_handles_minimum_blocks_per_second() {}
```

Use relaxed thresholds and counters where possible.

---

## Diagnostics

If a benchmark exposes a bottleneck, report:

* operation name
* observed latency
* expected threshold
* likely cause
* recommended optimization

Possible optimizations to evaluate:

* verification cache
* batch verification if available
* parallel validation
* RocksDB batch writes
* async gossip queue
* backpressure
* peer scoring before crypto validation

---

## Final output format

Report:

1. Benchmark files added
2. Guardrail tests added
3. ML-DSA sign/verify performance
4. SHA3 block hash performance
5. strict PQC validation throughput
6. RocksDB persistence performance
7. invalid flood rejection cost
8. full node throughput
9. bottlenecks found
10. recommended optimizations
11. exact commands used:

```bash
cargo bench --bench pqc_performance
cargo test --test performance_guardrails
```

Be strict. The goal is to discover whether the DLT is secure but too slow, or secure and production-feasible.
