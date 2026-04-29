# Cerulean Ledger — Incident Response Playbook

---

## 1. Consensus Halt

**Symptoms**: No new blocks for >5 minutes. Block height static across all validators.

**Impact**: Network stalled. No transactions processed.

**Immediate action**:
1. Check all validator logs for panics: `docker logs validator-N 2>&1 | grep -i panic`
2. Verify peer connectivity: `curl localhost:808X/api/v1/health`
3. If <3 validators alive: restart crashed nodes

**Diagnosis**:
```bash
# Check heights
for p in 8080 8082 8084; do
  curl -s localhost:$p/api/v1/store/blocks/latest
done

# Check if BFT round is stuck
docker logs validator-1 2>&1 | grep -i "round\|timeout\|view.change" | tail -20
```

**Recovery**: Restart all validators simultaneously. If consensus corrupted, resync from genesis.

**Postmortem**: Collect logs from all validators. Check for equivocation or network partition.

---

## 2. State Divergence

**Symptoms**: Different validators report different balances for the same account or different block heights.

**Impact**: Critical — chain integrity compromised.

**Immediate action**:
1. Stop accepting new transactions (disable faucet)
2. Identify divergence height by comparing block hashes across nodes
3. Identify the canonical chain (majority)

**Diagnosis**:
```bash
# Compare heights
./scripts/testnet.sh status

# Compare specific account
for p in 8080 8082 8084; do
  echo "Node $p:"
  curl -s localhost:$p/api/v1/accounts/alice
done
```

**Recovery**: Stop diverged node(s). Delete their data volumes. Resync from healthy node.

**Postmortem**: Investigate if non-deterministic execution caused the divergence. Check for floating-point or time-dependent logic in transaction execution.

---

## 3. Validator Crash

**Symptoms**: One validator container exits or becomes unresponsive.

**Impact**: Reduced fault tolerance. With 3 validators, losing 1 still allows consensus (3 > 2f+1 where f=0). Losing 2 halts consensus.

**Immediate action**:
```bash
docker compose -f deploy/public-testnet/docker-compose.public.yml restart validator-N
```

**Diagnosis**:
```bash
docker logs validator-N 2>&1 | tail -50
docker inspect validator-N --format='{{.State.ExitCode}}'
```

**Recovery**: Restart. If persistent crash, check RocksDB corruption: delete volume and resync.

---

## 4. Disk Full

**Symptoms**: Node crashes with I/O errors. RocksDB write failures in logs.

**Impact**: Affected node stops producing/validating blocks.

**Immediate action**:
```bash
# Check disk usage
docker exec validator-N df -h /data

# If >90%, prune old logs
docker exec validator-N find /data -name "*.log" -mtime +7 -delete
```

**Recovery**: Expand volume or move to larger disk. Restart node.

**Prevention**: Set up disk usage alerts at 80%. Plan for ~1 GB/month growth at 15s blocks.

---

## 5. Faucet Abuse

**Symptoms**: Rapid faucet requests from single IP. Faucet balance depleting unexpectedly fast.

**Impact**: Legitimate users can't get test tokens.

**Immediate action**:
```bash
# Disable faucet
# Set FAUCET_ENABLED=false and restart API node

# Or block abusing IP at nginx level
# Add to nginx.conf: deny <IP>;
```

**Diagnosis**: Check faucet logs for request patterns. Look for many different addresses from same IP.

**Recovery**: Re-enable faucet with stricter limits. Consider CAPTCHA for web faucet.

---

## 6. API Overload

**Symptoms**: API response times >5s. HTTP 503 errors. High CPU on API node.

**Impact**: Explorer and wallet CLI unusable.

**Immediate action**:
1. Tighten nginx rate limit (reduce from 100 to 20 req/min)
2. If DDoS, block offending IPs at firewall level

**Diagnosis**:
```bash
# Check nginx access log for top IPs
docker exec api-nginx cat /var/log/nginx/access.log | awk '{print $1}' | sort | uniq -c | sort -rn | head
```

**Recovery**: Scale API node or add read replicas behind load balancer.

---

## 7. P2P Spam

**Symptoms**: High CPU on sentry nodes. Excessive log volume. Network bandwidth spike.

**Impact**: Legitimate peers can't connect. Block propagation delayed.

**Immediate action**:
1. Check peer connections: `docker logs sentry-1 | grep "connect\|peer" | tail -20`
2. Enable peer allowlist if available (`PEER_ALLOWLIST` env var)
3. Block offending IPs at firewall

**Recovery**: Restart sentries with stricter peer limits.

---

## 8. Chain Reorg Anomaly

**Symptoms**: Block height decreases on one or more nodes. Transactions that were confirmed become unconfirmed.

**Impact**: Temporary double-spend window. User confusion.

**Immediate action**: This is expected behavior in fork resolution. Verify the reorg depth is small (<5 blocks).

**Diagnosis**:
```bash
docker logs validator-1 2>&1 | grep -i "reorg\|rollback\|fork"
```

**Escalation**: If reorg depth >10 blocks, treat as potential consensus attack. Stop network and investigate.

---

## 9. Corrupted RocksDB

**Symptoms**: Node crashes on startup with RocksDB errors. `Corruption` or `IO error` in logs.

**Impact**: Affected node cannot start.

**Immediate action**:
```bash
# Stop node
docker compose stop validator-N

# Delete data and resync
docker volume rm public-testnet_validatorN-data
docker compose up -d validator-N
```

**Prevention**: Use `sync_wal=true` in RocksDB options. Ensure clean shutdown (SIGTERM, not SIGKILL).

---

## 10. Emergency: Full Network Restart

**When**: Unrecoverable state across all nodes.

**Procedure**:
1. Stop all nodes: `docker compose down`
2. Back up all volumes: `docker run --rm -v VOL:/data -v $(pwd):/backup alpine tar czf /backup/VOL.tar.gz /data`
3. If resetting chain: `docker compose down -v`
4. Restart: `docker compose up -d`
5. Verify health on all nodes
6. Re-enable faucet

**Communication**: Post notice in public channels before and after reset.
