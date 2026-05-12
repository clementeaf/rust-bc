# Cerulean Ledger — Sandbox Benchmark Report

**Date:** 2026-05-12
**Environment:** Docker sandbox, single node (1 CPU, 512MB RAM, RocksDB, ML-DSA-65)
**Platform:** Apple Silicon (ARM64) via Docker Desktop

## Summary

| Metric | Result |
|---|---|
| Stress modules | 10/10 passed, 0 failed |
| Pentest scenarios | 40 total, 36 blocked, 4 detected, **0 vulnerable** |
| Health latency | 20ms avg (20 samples) |
| Block reads (HTTP) | 110-580 req/s |
| Identity writes (HTTP) | 22 writes/s (rate-limited, see note) |
| Gateway transactions (HTTP) | 22 tx/s (rate-limited, see note) |

> **Note:** HTTP benchmarks are constrained by the per-IP token-bucket rate limiter.
> Internal module throughput (below) reflects actual processing capacity without HTTP overhead.

## Internal Module Throughput (1,000 ops/module)

| Module | ops/s | p99 latency | Status |
|---|---|---|---|
| crypto_hash (SHA-256) | 6,640,856 | <1us | Pass |
| identity | 1,918,465 | <1us | Pass |
| anomaly_detection | 1,352,874 | 1us | Pass |
| iso20022_validation | 1,305,270 | <1us | Pass |
| forensic | 1,052,263 | <1us | Pass |
| governance | 957,664 | <1us | Pass |
| credential | 806,885 | <1us | Pass |
| pattern_detection | 698,833 | 1us | Pass |
| risk_scoring | 4,837,719 | <1us | Pass |
| storage (RocksDB) | 48,072 | 1us | Pass |

## Adversarial Pentest (40 scenarios)

| Category | Scenarios | Blocked | Detected |
|---|---|---|---|
| Integrity | 2 | 2 | 0 |
| Cryptography | 2 | 2 | 0 |
| Protocol | 2 | 2 | 0 |
| Consensus/BFT | 5 | 5 | 0 |
| Access Control | 2 | 2 | 0 |
| DoS Protection | 2 | 2 | 0 |
| Isolation | 1 | 1 | 0 |
| Identity | 4 | 3 | 1 |
| Input Validation | 2 | 2 | 0 |
| Key Management | 1 | 1 | 0 |
| Oracle | 1 | 1 | 0 |
| Governance | 5 | 5 | 0 |
| Economic | 3 | 2 | 1 |
| EVM | 4 | 4 | 0 |
| Network | 3 | 3 | 0 |
| Arithmetic | 1 | 0 | 1 |

**Detected (not vulnerable):**
- PEN-012: Integer overflow — saturating arithmetic prevents overflow, mint rejected
- PEN-019: Credential forgery — forged credential distinguishable by issuer DID
- PEN-030: Fee suppression — base fee floors at MIN_BASE_FEE, recovers in ~33 blocks
- PEN-040: Signature bypass — invalid hex rejected at decode stage (400 response)

**Vulnerable: 0**

## CI/CD Pipeline

All workflows run on every push to `main`:

| Workflow | Gates |
|---|---|
| CI | fmt check, clippy -D warnings, cargo check, unit tests, integration tests, benchmarks, E2E Docker |
| Test | lib tests, pqc_crypto_module, crypto boundary, FIPS readiness, property invariants, BFT tests |
| Coverage | cargo-llvm-cov |
| Security | cargo audit, dependency review |
| Performance | TPS benchmarks (release mode) |
| Lint | clippy, fmt |
| Fuzz | cargo-fuzz |

## How to reproduce

```bash
# Start sandbox
docker compose -f docker-compose.sandbox.yml up -d
./scripts/seed-sandbox.sh

# Stress test (adjust ops count)
curl http://localhost:9600/api/v1/stress/report?ops=1000 | jq .data

# Pentest
curl http://localhost:9600/api/v1/pentest/report | jq .data

# Benchmark script (requires multi-node cluster)
./scripts/benchmark.sh
```
