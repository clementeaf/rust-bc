# k6 Load Testing

## Prerequisites

```bash
# Install k6
brew install k6          # macOS
# or: https://k6.io/docs/get-started/installation/

# Start node in permissive mode
ACL_MODE=permissive cargo run --bin rust-bc
```

## Run

```bash
# Default: 50 VUs, 2 minutes, ramp up/down
k6 run tools/k6/load-test.js

# Against a remote node
k6 run tools/k6/load-test.js --env BASE_URL=https://node.example.com:8080

# Spike test: 200 VUs sudden burst
k6 run tools/k6/load-test.js --env SCENARIO=spike

# Soak test: 30 VUs for 10 minutes
k6 run tools/k6/load-test.js --env SCENARIO=soak

# Stress test: ramp to 300 VUs
k6 run tools/k6/load-test.js --env SCENARIO=stress
```

## Thresholds (auto-fail)

| Metric | Threshold |
|--------|-----------|
| p(95) response time | < 500ms |
| Error rate | < 1% |
| Health check p(99) | < 100ms |

## Traffic Distribution

| Action | Weight | Type |
|--------|--------|------|
| Health check | 25% | Read |
| Stats | 15% | Read |
| List blocks | 15% | Read |
| Mempool | 10% | Read |
| Audit query | 10% | Read |
| Create identity | 10% | Write |
| Wallet + mine | 8% | Write |
| Create wallet | 7% | Write |

## Results

JSON reports saved to `tools/k6/results/` after each run.
