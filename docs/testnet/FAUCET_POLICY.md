# Cerulean Ledger — Faucet Policy

**Test tokens have no monetary value.**

---

## Rate Limits

| Limit | Value | Rationale |
|---|---|---|
| Per-address cooldown | 100 blocks (~25 min) | Prevent single-address drain |
| Per-IP daily limit | 10 drips/day | Prevent multi-address sybil from same IP |
| Global daily cap | 100,000 NOTA/day | Preserve faucet longevity |
| Drip amount | 1,000 NOTA per request | Enough for ~100 transfers at fee=5 |
| Total faucet reserve | 10,000,000 NOTA | ~100 days at max daily rate |

## Configuration

```env
FAUCET_ENABLED=true
FAUCET_AMOUNT=1000
FAUCET_COOLDOWN_BLOCKS=100
FAUCET_IP_DAILY_LIMIT=10
FAUCET_GLOBAL_DAILY_CAP=100000
FAUCET_MAX_TOTAL=10000000
```

## Abuse Handling

### Automated

- Requests from exhausted IPs return HTTP 429 with cooldown info
- Daily cap returns HTTP 429 with "daily cap reached" message
- Address cooldown returns HTTP 429 with next available block

### Manual

| Action | When | How |
|---|---|---|
| Disable faucet | Active drain attack | Set `FAUCET_ENABLED=false`, restart API node |
| Block IP | Identified abuser | Add to nginx deny list or firewall |
| Reduce drip amount | Sustained high demand | Lower `FAUCET_AMOUNT` to 100 |
| Increase cooldown | Moderate abuse | Raise `FAUCET_COOLDOWN_BLOCKS` to 500 |

### Manual Override

For authorized users (e.g., demo preparation):

```bash
# Direct credit via API (requires ACL_MODE=permissive or valid identity)
curl -X POST http://localhost:8080/api/v1/faucet/drip \
  -H "Content-Type: application/json" \
  -d '{"address": "TARGET_ADDRESS"}'
```

## Faucet Disable Switch

Emergency disable without restart:

1. Set `FAUCET_ENABLED=false` in environment
2. Restart API node: `docker compose restart api-1`
3. Verify: `curl http://api-host/api/v1/faucet/status` should show `enabled: false`

## Monitoring

Track daily:
- Total drips served
- Unique addresses served
- Unique IPs served
- Rejection rate (cooldown / IP limit / daily cap / depleted)
- Faucet remaining balance
