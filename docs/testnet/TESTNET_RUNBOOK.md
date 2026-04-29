# Cerulean Ledger — Testnet Runbook

Exact commands for operating the public testnet.

---

## Deploy (first time)

```bash
# Clone and build
git clone <repo> && cd cerulean-ledger
git checkout feat/cryptocurrency-exploration

# Build Docker images
docker compose -f deploy/public-testnet/docker-compose.public.yml build

# Start all nodes
docker compose -f deploy/public-testnet/docker-compose.public.yml up -d
```

## Start Nodes

```bash
# All nodes
docker compose -f deploy/public-testnet/docker-compose.public.yml up -d

# Single node
docker compose -f deploy/public-testnet/docker-compose.public.yml up -d validator-1
```

## Stop Nodes

```bash
# Graceful stop (preserves data)
docker compose -f deploy/public-testnet/docker-compose.public.yml stop

# Stop and remove containers (preserves volumes)
docker compose -f deploy/public-testnet/docker-compose.public.yml down

# Stop and destroy everything including data
docker compose -f deploy/public-testnet/docker-compose.public.yml down -v
```

## Check Health

```bash
# All nodes
for port in 8080 8082 8084; do
  echo -n "Node $port: "
  curl -sf "http://localhost:$port/api/v1/health" | python3 -m json.tool
done

# Single node
curl -s http://localhost:8080/api/v1/health | python3 -m json.tool
```

## Check Height

```bash
for port in 8080 8082 8084; do
  echo -n "Node $port height: "
  curl -s "http://localhost:$port/api/v1/store/blocks/latest" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(d.get('data', {}).get('height', 'unknown'))
" 2>/dev/null
done
```

## Check Peers

```bash
curl -s http://localhost:8080/api/v1/discovery/peers | python3 -m json.tool
```

## Check Mempool

```bash
curl -s http://localhost:8080/api/v1/mempool/stats | python3 -m json.tool
```

## Inspect Logs

```bash
# All nodes
docker compose -f deploy/public-testnet/docker-compose.public.yml logs -f

# Single node, last 100 lines
docker compose -f deploy/public-testnet/docker-compose.public.yml logs --tail 100 validator-1

# Filter for errors
docker compose -f deploy/public-testnet/docker-compose.public.yml logs validator-1 2>&1 | grep -i "error\|panic\|fatal"
```

## Restart a Node

```bash
# Graceful restart (keeps data)
docker compose -f deploy/public-testnet/docker-compose.public.yml restart validator-2
```

## Resync a Node from Genesis

```bash
# Stop the node
docker compose -f deploy/public-testnet/docker-compose.public.yml stop validator-2

# Remove its data volume
docker volume rm public-testnet_validator2-data

# Restart — it will sync from peers
docker compose -f deploy/public-testnet/docker-compose.public.yml up -d validator-2
```

## Verify State Consistency

```bash
# Compare block heights across all validators
./scripts/testnet.sh status

# Compare account balances
for port in 8080 8082 8084; do
  echo -n "Node $port faucet balance: "
  curl -s "http://localhost:$port/api/v1/accounts/faucet" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(d.get('data', {}).get('balance', '?'))
" 2>/dev/null
done
```

## Faucet Operations

```bash
# Check faucet status
curl -s http://localhost:8080/api/v1/faucet/status | python3 -m json.tool

# Manual drip
curl -s -X POST http://localhost:8080/api/v1/faucet/drip \
  -H "Content-Type: application/json" \
  -d '{"address": "TARGET_ADDRESS"}' | python3 -m json.tool
```

## Emergency: Full Network Restart

```bash
docker compose -f deploy/public-testnet/docker-compose.public.yml down
sleep 5
docker compose -f deploy/public-testnet/docker-compose.public.yml up -d
```
