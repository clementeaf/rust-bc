# Security Audit Package

Prepared for external security auditors. Last updated: 2026-04-13.

## Project Overview

rust-bc is a permissioned blockchain node written in Rust with Fabric-inspired architecture. It supports ERC-20/ERC-721 tokens, WASM chaincode, W3C DIDs, post-quantum cryptography (ML-DSA-65), and Raft consensus.

**Codebase:** ~25K lines of Rust, 992 unit tests, 104 E2E assertions, 28 recovery tests, 15 fuzz targets.

## Scope for Audit

### Critical paths (priority)
| Area | Files | Why |
|------|-------|-----|
| ACL enforcement | `src/api/errors.rs` | All auth decisions flow through `enforce_acl()` |
| Transaction validation | `src/transaction_validation.rs` | Replay, double-spend, timestamp drift |
| P2P message handling | `src/network/mod.rs` | Message parsing, peer auth, state sync |
| Chaincode sandbox | `src/chaincode/executor.rs` | Wasmtime fuel/memory limits, host functions |
| Cryptographic signing | `src/identity/signing.rs` | Ed25519 + ML-DSA-65, key management |
| Smart contracts | `src/smart_contracts.rs` | ERC-20/721 logic, overflow protection |
| Input validation | `src/api/handlers/validation.rs` | XSS, null bytes, field length limits |

### Infrastructure
| Area | Files |
|------|-------|
| TLS/mTLS | `src/tls.rs`, `src/pki.rs` |
| Checkpoint integrity | `src/checkpoint.rs` (HMAC-SHA256) |
| Rate limiting | `src/middleware.rs`, `src/api/rate_limit.rs` |
| Network security | `src/network_security.rs` |
| Storage | `src/storage/adapters.rs` (RocksDB) |

## Internal Audit Results

Full report: `docs/SECURITY-AUDIT.md`

| Finding | Severity | Status |
|---------|----------|--------|
| Legacy routes without ACL | CRITICAL | FIXED |
| Header spoofing in strict mode | CRITICAL | FIXED |
| Rate limiter dead code | HIGH | FIXED |
| Weak double-spend heuristic | HIGH | FIXED |
| In-memory replay prevention | HIGH | FIXED (RocksDB persisted) |
| Network security dead code | HIGH | FIXED (integrated into P2P) |
| Debug logging in production | MEDIUM | FIXED |
| No HMAC on checkpoints | MEDIUM | FIXED |
| No Wasm hash verification | MEDIUM | FIXED |
| JWT announced but unused | MEDIUM | DOCUMENTED |

## Testing Artifacts

| Test suite | Command | Assertions |
|-----------|---------|-----------|
| Unit tests | `cargo test --lib` | 992 |
| Integration | `cargo test --test store_blocks_api_test` | 4 |
| E2E (Docker) | `./scripts/e2e-test.sh` | 104 |
| Recovery | `./scripts/recovery-test.sh` | 28 |
| Fuzz (proptest) | `cargo test --test fuzz_tests` | 15 targets, ~27K cases |
| Stress | `./scripts/stress-test.sh` | 4 phases |
| Load | `./scripts/load-test.sh` | Sustained throughput |

## Stress Test Results

| Phase | Result |
|-------|--------|
| Throughput ceiling | 147 tx/s at concurrency 10 |
| 500 concurrent connections | 100% success |
| 5MB payload | Correctly rejected |
| XSS / null bytes / huge ID | Correctly rejected |
| Node survives all malformed input | Yes |

## Fuzzing Results

15 proptest targets, ~27K generated inputs:
- 0 panics in JSON deserialization
- 0 panics in smart contract operations
- 0 panics in storage roundtrips
- 1 integer overflow found and fixed (`validate_timestamp`, `saturating_add`)

## Dependency Audit

```bash
cargo audit              # Known CVEs
cargo deny check         # License + advisory
cargo tree -d            # Duplicate dependencies
```

Key dependencies:
- `wasmtime` v36 (15 CVEs fixed from v21)
- `rustls` (no OpenSSL)
- `ed25519-dalek` with `ZeroizeOnDrop`
- `sha2`, `hmac` for integrity
- `rocksdb` for persistence

## How to Run

```bash
# Full test suite
cargo test --lib
cargo test --test fuzz_tests
cargo test --test store_blocks_api_test

# Docker network
docker compose up -d
./scripts/e2e-test.sh
./scripts/recovery-test.sh
./scripts/stress-test.sh
./scripts/load-test.sh --duration 60 --rate 100

# Clippy (zero warnings required)
cargo clippy -- -D warnings
```
