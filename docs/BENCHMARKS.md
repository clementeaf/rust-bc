# Performance Benchmarks

Methodology, tools, and results for rust-bc performance characterization.

## Quick run

```bash
# Micro-benchmarks (no network required)
cargo bench

# Full benchmark suite (requires running Docker network)
docker compose up -d node1 node2 node3 orderer1
./scripts/benchmark.sh
```

## Benchmark categories

### 1. Ordering throughput (Criterion)

Measures transactions/second through the Solo ordering service: submit N transactions then cut one block.

| Metric | Measurement |
|--------|-------------|
| Benchmark | `ordering_service/submit_and_cut/100` |
| Batch size | 100 TXs |
| Operation | submit_tx × 100 + cut_block |
| Tool | Criterion (statistical, 100+ iterations) |

### 2. Endorsement validation latency (Criterion)

Measures Ed25519 signature verification cost per endorsement with AllOf(N) policy.

| N orgs | Measurement |
|--------|-------------|
| 1 | `endorsement_validation/validate_endorsements/1` |
| 3 | `endorsement_validation/validate_endorsements/3` |
| 5 | `endorsement_validation/validate_endorsements/5` |
| 10 | `endorsement_validation/validate_endorsements/10` |

### 3. Event bus fan-out (Criterion)

Measures `publish()` latency as subscriber count grows.

| Subscribers | Measurement |
|-------------|-------------|
| 1 | `event_bus_fanout/publish_1_event/1` |
| 5 | `event_bus_fanout/publish_1_event/5` |
| 10 | `event_bus_fanout/publish_1_event/10` |
| 50 | `event_bus_fanout/publish_1_event/50` |

### 4. RocksDB write throughput (Criterion)

Measures sequential block writes to RocksDB.

| Blocks | Measurement |
|--------|-------------|
| 10 | `rocksdb_storage/write_blocks/10` |
| 100 | `rocksdb_storage/write_blocks/100` |

### 5. Gateway end-to-end latency (live)

Single `POST /gateway/submit` request — measures full pipeline: HTTP → ACL → endorse → order → commit → response.

### 6. Sequential throughput (live)

50 sequential gateway submits — measures sustained TPS under serial load.

### 7. Block propagation (live)

Time from mining a block on node1 until it appears on node2 via gossip pull-sync.

### 8. Health check latency (live)

Average of 10 `GET /health` requests — baseline for API overhead.

## Running benchmarks

### Criterion (micro-benchmarks)

```bash
# All benchmarks
cargo bench

# Specific group
cargo bench -- ordering_service
cargo bench -- endorsement_validation
cargo bench -- event_bus_fanout
cargo bench -- rocksdb_storage

# HTML reports
open target/criterion/report/index.html
```

### Live benchmarks

```bash
# Default (localhost:8080)
./scripts/benchmark.sh

# Custom node
./scripts/benchmark.sh https://node1.example.com:8080
```

## Environment

Document the hardware specs when reporting results:

```
Machine:     [e.g. MacBook Pro M2, 16GB RAM]
OS:          [e.g. macOS 14.5 / Ubuntu 22.04]
Rust:        [e.g. nightly-2024-12-18]
Docker:      [e.g. Docker Desktop 4.30]
Nodes:       [e.g. 3 peers + 1 orderer, local Docker]
Storage:     [e.g. RocksDB on SSD]
```

## Comparison with Fabric 2.5

For a fair comparison, run the same workload on both systems:

| Metric | rust-bc | Fabric 2.5 | Notes |
|--------|---------|-----------|-------|
| Gateway submit latency | TBD | ~50-100ms | Single TX, Solo ordering |
| Sequential TPS | TBD | ~100-300 TPS | Serial submits |
| Block propagation | TBD | ~1-3s | Gossip-based |
| Memory footprint | ~50MB/node | ~500MB/peer | Container RSS |
| Startup time | ~2s | ~15-30s | Cold start |

Run `./scripts/benchmark.sh` and fill in the "TBD" columns with your measurements.
