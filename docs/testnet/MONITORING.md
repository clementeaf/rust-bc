# Cerulean Ledger — Testnet Monitoring

---

## Metrics

### Blockchain

| Metric | Source | Type |
|---|---|---|
| Block height | `/api/v1/store/blocks/latest` | Gauge |
| Block time (seconds since last block) | Computed from timestamps | Gauge |
| Peer count | `/api/v1/discovery/peers` | Gauge |
| Mempool size | `/api/v1/mempool/stats` → `pending` | Gauge |
| Base fee | `/api/v1/mempool/stats` → `base_fee` | Gauge |

### Crypto Layer (CryptoMetrics)

| Metric | Description | Type |
|---|---|---|
| `transfers_total` | Successful native transfers | Counter |
| `transfers_failed` | Failed transfer attempts | Counter |
| `rejected_signatures` | Invalid signatures at mempool admission | Counter |
| `blocks_produced` | Blocks produced by this node | Counter |
| `fees_burned` | Total NOTA burned via fees | Counter |
| `fees_to_proposers` | Total NOTA paid to proposers | Counter |
| `rewards_minted` | Total block rewards minted | Counter |

### Faucet

| Metric | Description |
|---|---|
| Drips served (total) | Faucet total_distributed |
| Remaining balance | Faucet remaining() |
| Rejections by type | Cooldown / IP limit / daily cap / depleted |

### Infrastructure

| Metric | Source | Alert threshold |
|---|---|---|
| CPU usage | Docker stats / node_exporter | >80% sustained |
| Memory usage | Docker stats | >80% |
| Disk usage | df / node_exporter | >80% |
| RocksDB errors | Application logs | Any |

## Alert Thresholds

| Condition | Severity | Action |
|---|---|---|
| No new block for 5 minutes | CRITICAL | Page on-call. Check consensus. |
| Peer count < 2 | HIGH | Check P2P connectivity, restart sentries |
| Disk > 80% | HIGH | Expand storage or prune |
| Rejected TX spike (>100/min) | MEDIUM | Investigate spam, tighten rate limits |
| Faucet drip spike (>50/min) | MEDIUM | Check for abuse, consider disable |
| Height divergence between validators (>2 blocks) | HIGH | Check for fork, verify state hashes |
| Validator offline > 2 minutes | HIGH | Restart validator |
| API response time p95 > 5s | MEDIUM | Check load, scale if needed |

## Prometheus Setup

```yaml
# deploy/public-testnet/prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'cerulean-validators'
    static_configs:
      - targets:
          - 'validator-1:8080'
          - 'validator-2:8082'
          - 'validator-3:8084'
    metrics_path: '/metrics'

  - job_name: 'cerulean-api'
    static_configs:
      - targets: ['api-1:8080']

  - job_name: 'node-exporter'
    static_configs:
      - targets:
          - 'validator-1:9100'
          - 'validator-2:9100'
          - 'validator-3:9100'
```

## Grafana Dashboards

| Dashboard | Panels |
|---|---|
| Network Overview | Block height, block time, peer count, mempool |
| Crypto Economics | Transfers/sec, fees burned, rewards minted, base fee |
| Faucet | Drips/hour, remaining balance, rejections |
| Infrastructure | CPU, memory, disk per node |

## Health Endpoint

```
GET /api/v1/health
```

Expected response:
```json
{
  "status": "ok",
  "checks": {
    "storage": "ok",
    "peers": "ok",
    "ordering": "ok"
  }
}
```

Degraded when storage or ordering unavailable.
