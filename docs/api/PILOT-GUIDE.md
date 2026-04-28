# Pilot Deployment Guide

Step-by-step guide for deploying the first production pilot with a partner organization.

---

## Prerequisites

Before starting:

- [ ] Partner organization identified (org name, contact, use case)
- [ ] Network topology agreed (how many peers per org, shared orderers or dedicated)
- [ ] Data classification done (what goes on-chain vs private data vs off-chain)
- [ ] TLS certificates issued (production CA, not self-signed)
- [ ] DNS entries created for each node
- [ ] Docker or bare-metal hosts provisioned (2+ CPU, 4+ GB RAM, SSD)

## Phase 1 — Network setup (Day 1)

### 1.1 Generate production TLS certificates

Do NOT use `deploy/generate-tls.sh` in production. Use your organization's CA or a service like Vault PKI.

```bash
# Example with cfssl (CloudFlare PKI toolkit)
cfssl gencert -initca ca-csr.json | cfssljson -bare ca
cfssl gencert -ca=ca.pem -ca-key=ca-key.pem orderer1-csr.json | cfssljson -bare orderer1
cfssl gencert -ca=ca.pem -ca-key=ca-key.pem peer1-csr.json | cfssljson -bare peer1
```

Each node needs:
- `node-cert.pem` — Node certificate (signed by CA)
- `node-key.pem` — Node private key (600 permissions)
- `ca-cert.pem` — CA certificate (for peer verification)

### 1.2 Configure orderer cluster

Minimum 3 orderers for crash fault tolerance. All orderers must know each other.

```yaml
# orderer1 environment
ORDERING_BACKEND: "raft"
RAFT_NODE_ID: "1"
RAFT_PEERS: "1:orderer1.example.com:8081,2:orderer2.example.com:8081,3:orderer3.example.com:8081"
STORAGE_BACKEND: "rocksdb"
STORAGE_PATH: "/data/rocksdb"
RUST_BC_ENV: "production"
JWT_SECRET: "<generated-256-bit-secret>"
SIGNING_ALGORITHM: "ml-dsa-65"  # or "ed25519"
```

### 1.3 Configure peer nodes

Each organization runs at least one peer.

```yaml
# peer1 (org1) environment
NODE_ROLE: "peer"
ORG_ID: "org1"
STORAGE_BACKEND: "rocksdb"
STORAGE_PATH: "/data/rocksdb"
BOOTSTRAP_NODES: "orderer1.example.com:8081,orderer2.example.com:8081"
ACL_MODE: "strict"
RUST_BC_ENV: "production"
JWT_SECRET: "<same-secret-as-orderers>"
```

### 1.4 Start the network

```bash
# On each host
docker run -d \
  --name rust-bc \
  -p 8080:8080 -p 8081:8081 \
  -v /data/rocksdb:/data/rocksdb \
  -v /tls:/tls:ro \
  --env-file .env \
  rust-bc:latest
```

### 1.5 Verify health

```bash
# From any host
curl -sk https://orderer1.example.com:8080/api/v1/health | jq
curl -sk https://peer1-org1.example.com:8080/api/v1/health | jq
```

All nodes should report `"status": "healthy"`.

## Phase 2 — Organization onboarding (Day 2)

### 2.1 Register organizations

```bash
# Register org1
curl -sk -X POST https://peer1-org1.example.com:8080/api/v1/organizations \
  -H "Content-Type: application/json" \
  -d '{
    "org_id": "org1",
    "msp_id": "org1MSP",
    "admin_dids": ["did:bc:org1:admin"],
    "root_public_keys": ["<hex-encoded-public-key>"]
  }'

# Register org2
curl -sk -X POST https://peer1-org2.example.com:8080/api/v1/organizations \
  -H "Content-Type: application/json" \
  -d '{
    "org_id": "org2",
    "msp_id": "org2MSP",
    "admin_dids": ["did:bc:org2:admin"],
    "root_public_keys": ["<hex-encoded-public-key>"]
  }'
```

### 2.2 Create a channel

```bash
curl -sk -X POST https://peer1-org1.example.com:8080/api/v1/channels \
  -H "Content-Type: application/json" \
  -d '{"channel_id": "pilot-channel"}'
```

### 2.3 Set endorsement policy

```bash
curl -sk -X POST https://peer1-org1.example.com:8080/api/v1/policies \
  -H "Content-Type: application/json" \
  -d '{
    "policy_id": "pilot-policy",
    "policy": {"AllOf": ["org1", "org2"]}
  }'
```

## Phase 3 — Chaincode deployment (Day 3)

### 3.1 Write and compile chaincode

```bash
cd my-chaincode
cargo build --target wasm32-unknown-unknown --release
```

### 3.2 Install on all peers

```bash
for peer in peer1-org1 peer1-org2; do
  curl -sk -X POST "https://${peer}.example.com:8080/api/v1/chaincode/install" \
    -H "Content-Type: application/octet-stream" \
    -H "X-Chaincode-Id: pilot-cc" \
    -H "X-Chaincode-Version: 1.0" \
    --data-binary @target/wasm32-unknown-unknown/release/my_chaincode.wasm
done
```

### 3.3 Approve and commit

```bash
# Org1 approves
curl -sk -X POST "https://peer1-org1.example.com:8080/api/v1/chaincode/pilot-cc/approve?version=1.0" \
  -H "X-Org-Id: org1"

# Org2 approves
curl -sk -X POST "https://peer1-org2.example.com:8080/api/v1/chaincode/pilot-cc/approve?version=1.0" \
  -H "X-Org-Id: org2"

# Commit (any peer)
curl -sk -X POST "https://peer1-org1.example.com:8080/api/v1/chaincode/pilot-cc/commit?version=1.0"
```

## Phase 4 — Validation (Day 4-5)

### 4.1 Run load test

```bash
./scripts/load-test.sh --duration 3600 --rate 100 --node https://peer1-org1.example.com:8080
```

Expected results for a pilot:
- Throughput: 50+ tx/s sustained
- p99 latency: < 500ms
- Error rate: < 1%
- No node crashes over 1 hour

### 4.2 Verify block propagation

```bash
# Check all peers have the same block height
for peer in peer1-org1 peer1-org2; do
  echo -n "${peer}: "
  curl -sk "https://${peer}.example.com:8080/api/v1/store/blocks/latest" | jq '.data'
done
```

### 4.3 Test failover

```bash
# Stop one orderer
docker stop rust-bc-orderer2

# Verify the network continues operating (2/3 orderers = majority)
curl -sk -X POST "https://peer1-org1.example.com:8080/api/v1/store/transactions" \
  -H "Content-Type: application/json" \
  -d '{"id":"failover-test","block_height":0,"timestamp":0,"input_did":"test","output_recipient":"test","amount":1,"state":"test"}'

# Restart orderer
docker start rust-bc-orderer2
```

### 4.4 Verify audit trail

```bash
curl -sk "https://peer1-org1.example.com:8080/api/v1/audit?limit=10" | jq
```

## Phase 5 — Go-live checklist

Before declaring the pilot live:

- [ ] All nodes healthy for 24+ hours
- [ ] Load test passed (1 hour, target TPS, < 1% errors)
- [ ] Block propagation confirmed across all peers
- [ ] Orderer failover tested (stop 1 of 3, verify continuity)
- [ ] Audit trail captures all requests
- [ ] Monitoring dashboards accessible (Grafana)
- [ ] Backup procedure tested (RocksDB snapshot + restore)
- [ ] Incident response contact identified
- [ ] Partner org has API access and can submit transactions
- [ ] Chaincode deployed and invocable by both orgs

## Backup procedure

### Create backup

```bash
# On each node
docker exec rust-bc tar czf /tmp/backup.tar.gz -C /data/rocksdb .
docker cp rust-bc:/tmp/backup.tar.gz ./backups/node1-$(date +%Y%m%d).tar.gz
```

### Restore from backup

```bash
docker stop rust-bc
docker run --rm -v node1-data:/data alpine sh -c "rm -rf /data/rocksdb/*"
docker cp ./backups/node1-20260410.tar.gz rust-bc:/tmp/
docker exec rust-bc tar xzf /tmp/backup.tar.gz -C /data/rocksdb
docker start rust-bc
```

### Verify integrity after restore

```bash
curl -sk https://node1.example.com:8080/api/v1/health | jq
curl -sk https://node1.example.com:8080/api/v1/store/blocks/latest | jq
```

## Monitoring alerts

Configure these alerts in Grafana for the pilot:

| Alert | Condition | Severity |
|---|---|---|
| Node down | Health check fails for 60s | Critical |
| High error rate | API 5xx rate > 5% for 5 min | Critical |
| Block height stale | No new blocks for 5 min | Warning |
| Disk usage high | RocksDB volume > 80% | Warning |
| Raft leader lost | No Raft leader for 30s | Critical |
| High latency | p99 API latency > 1s for 5 min | Warning |
