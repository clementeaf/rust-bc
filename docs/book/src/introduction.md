# rust-bc

A high-performance blockchain node written in Rust with post-quantum cryptography, BFT consensus, and parallel transaction execution.

## Key Features

| Feature | Detail |
|---------|--------|
| **Consensus** | HotStuff-inspired BFT + Raft (selectable via `CONSENSUS_MODE`) |
| **Cryptography** | Ed25519 + ML-DSA-65 (FIPS 204 post-quantum) |
| **Execution** | Wave-parallel with conflict detection (RAW/WAW/WAR) |
| **Smart Contracts** | WebAssembly (Wasmtime) — any language that compiles to Wasm |
| **Tokenomics** | 100M NOTA supply cap, halving rewards, EIP-1559 dynamic fees, storage deposits |
| **Cross-chain** | Bridge framework with escrow, Merkle proofs, relay infrastructure |
| **Governance** | On-chain proposals, stake-weighted voting, timelock execution |
| **Identity** | W3C DID (`did:bc:*`), verifiable credentials |
| **Enterprise** | mTLS, ACL, private data collections, channel isolation, Fabric-compatible pipeline |
| **Light Client** | BFT-verified header chain + Merkle state proofs |
| **Footprint** | ~50 MB per node (vs 128 GB for IOTA, 500 MB for Fabric) |

## Quick Install

```bash
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc
cargo build --release
```

## Run a Local Node

```bash
# Single node (default: API on 8080, P2P on 8081)
cargo run --release

# With RocksDB persistence
STORAGE_BACKEND=rocksdb cargo run --release

# BFT mode
CONSENSUS_MODE=bft cargo run --release
```

## Run Tests

```bash
cargo test                        # All tests (~1300+)
cargo test --test bft_e2e         # BFT adversarial tests
cargo test --test bridge_e2e      # Cross-chain lifecycle tests
cargo test --release --test full_benchmark -- --nocapture  # TPS benchmarks
```

## Performance (release mode)

| Workload | TPS | Notes |
|----------|-----|-------|
| 500 independent txs | **56,104** | Single wave, zero conflicts |
| 1000 mixed (80/20) | **39,346** | 200 waves, 80% committed |
| Full pipeline (BFT + exec) | **>10,000** | 10 blocks x 100 txs |
