# Cloud Deployment Guide

Deploy rust-bc on 3+ VMs for production benchmarking with real WAN latency.

## Prerequisites

- 3 VMs (Ubuntu 22.04+, 2 vCPU, 4GB RAM, 20GB SSD each)
- Docker + Docker Compose on each VM
- Open ports: 8080 (API), 8081 (P2P), 9090 (Prometheus)
- SSH access from your machine to all VMs

## Quick Start

```bash
# 1. Edit inventory
cp inventory.example.env inventory.env
vim inventory.env   # Set your VM IPs

# 2. Deploy
./deploy-cloud.sh setup    # Install Docker, copy configs
./deploy-cloud.sh certs    # Generate and distribute TLS certs
./deploy-cloud.sh start    # Start all nodes
./deploy-cloud.sh status   # Health check all nodes
./deploy-cloud.sh bench    # Run load test against the network

# Lifecycle
./deploy-cloud.sh stop     # Stop all nodes
./deploy-cloud.sh logs     # Tail logs from all nodes
./deploy-cloud.sh destroy  # Remove everything
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   VM1 (peer)    │    │   VM2 (peer)    │    │  VM3 (orderer)  │
│  node1:8080/81  │◄──►│  node2:8080/81  │◄──►│  orderer:8080/81│
│  org1           │    │  org2           │    │  Raft leader    │
│  RocksDB        │    │  RocksDB        │    │  RocksDB        │
│  Prometheus     │    │                 │    │                 │
│  Grafana:3000   │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Environment Variables

Each node requires these in `.env`:

| Variable | VM1 | VM2 | VM3 |
|----------|-----|-----|-----|
| `API_PORT` | 8080 | 8080 | 8080 |
| `P2P_PORT` | 8081 | 8081 | 8081 |
| `BIND_ADDR` | 0.0.0.0 | 0.0.0.0 | 0.0.0.0 |
| `STORAGE_BACKEND` | rocksdb | rocksdb | rocksdb |
| `NETWORK_ID` | prod-net | prod-net | prod-net |
| `ACL_MODE` | strict | strict | strict |
| `BOOTSTRAP_NODES` | vm2:8081,vm3:8081 | vm1:8081,vm3:8081 | vm1:8081,vm2:8081 |
| `P2P_EXTERNAL_ADDRESS` | vm1:8081 | vm2:8081 | vm3:8081 |
| `ORG_ID` | org1 | org2 | orderer |
| `SIGNING_ALGORITHM` | ed25519 | ed25519 | ed25519 |
| `CHECKPOINT_HMAC_SECRET` | (unique) | (unique) | (unique) |
| `RATE_LIMIT_PER_SECOND` | 100 | 100 | 100 |
| `RATE_LIMIT_PER_MINUTE` | 3000 | 3000 | 3000 |

## Benchmarking

After deployment, run the load test from any machine that can reach the API:

```bash
# From your laptop
./scripts/load-test.sh --node https://<VM1_IP>:8080 --duration 300 --rate 500

# Full E2E validation
NODE1=https://<VM1_IP>:8080 NODE2=https://<VM2_IP>:8080 ./scripts/e2e-test.sh

# Recovery test
./scripts/recovery-test.sh  # Requires SSH access for docker stop/start
```

## Expected Results (3 VMs, same region)

| Metric | Docker localhost | Cloud (same region) | Cloud (cross-region) |
|--------|-----------------|--------------------|--------------------|
| TPS | ~80 tx/s | ~200-500 tx/s | ~50-150 tx/s |
| p50 latency | 18ms | 5-15ms | 50-200ms |
| p99 latency | 136ms | 30-80ms | 200-500ms |
| Block propagation | <1s | 1-3s | 3-10s |

Note: Docker localhost numbers are lower due to emulation overhead.
Cloud VMs with native linux achieve significantly higher throughput.
