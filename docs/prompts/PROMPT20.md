You are a senior blockchain infrastructure engineer and testnet launch architect.

Your task is to design and prepare a **public testnet launch plan** for Cerulean Ledger.

Current state:

* Production-grade testnet-ready
* Post-quantum cryptography integrated
* Native cryptoasset implemented
* RocksDB persistence
* deterministic sync from genesis
* reorg safety
* wallet CLI
* explorer page
* faucet
* metrics/health
* Docker/testnet packaging
* 1595 tests passing, 0 failures

Now the goal is:

👉 launch a public testnet that does not collapse in the first 48 hours.

---

# Objective

Create a practical public testnet launch plan covering:

1. infrastructure
2. node topology
3. monitoring
4. security limits
5. faucet control
6. upgrade/rollback
7. incident response
8. public documentation

---

# Target output

Create:

```text
docs/testnet/
├── PUBLIC_TESTNET_LAUNCH_PLAN.md
├── NODE_TOPOLOGY.md
├── TESTNET_RUNBOOK.md
├── INCIDENT_RESPONSE.md
├── FAUCET_POLICY.md
├── MONITORING.md
├── UPGRADE_ROLLBACK.md
├── USER_GUIDE.md
└── RISK_REGISTER.md
```

---

# 1. Public testnet launch plan

Create:

```text
docs/testnet/PUBLIC_TESTNET_LAUNCH_PLAN.md
```

Include:

* testnet purpose
* scope
* expected users
* what is NOT guaranteed
* launch phases:

  * private dry run
  * closed alpha
  * public testnet
  * incentivized testnet later
* success criteria:

  * uptime
  * sync reliability
  * faucet availability
  * no consensus halt
  * no state divergence

---

# 2. Node topology

Create:

```text
docs/testnet/NODE_TOPOLOGY.md
```

Design initial deployment:

```text
3 validator nodes
2 sentry nodes
1 API node
1 explorer node
1 monitoring node
```

Recommended layout:

* validator nodes not directly exposed
* sentry nodes handle external P2P
* API node rate-limited
* faucet isolated
* monitoring private

Include ports, firewall rules, and env vars.

---

# 3. Testnet runbook

Create:

```text
docs/testnet/TESTNET_RUNBOOK.md
```

Include exact commands for:

* deploy
* start nodes
* stop nodes
* check health
* check height
* check peers
* check mempool
* inspect logs
* restart a node
* resync a node from genesis
* restore from snapshot

---

# 4. Incident response

Create:

```text
docs/testnet/INCIDENT_RESPONSE.md
```

Cover:

* consensus halt
* state divergence
* validator crash
* disk full
* faucet abuse
* API overload
* P2P spam
* chain reorg anomaly
* corrupted RocksDB
* emergency restart

For each incident include:

```text
Symptoms
Impact
Immediate action
Diagnosis commands
Rollback/recovery
Postmortem checklist
```

---

# 5. Faucet policy

Create:

```text
docs/testnet/FAUCET_POLICY.md
```

Include:

* per-address cooldown
* per-IP daily cap
* global daily cap
* total faucet reserve
* abuse handling
* manual override
* faucet disable switch

Add config suggestions:

```env
FAUCET_ENABLED=true
FAUCET_AMOUNT=10
FAUCET_ADDRESS_COOLDOWN_SECONDS=86400
FAUCET_IP_DAILY_LIMIT=3
FAUCET_GLOBAL_DAILY_CAP=10000
```

---

# 6. Monitoring

Create:

```text
docs/testnet/MONITORING.md
```

Define metrics:

* block height
* peer count
* block time
* mempool size
* rejected txs
* invalid signatures
* faucet requests
* RocksDB errors
* CPU/memory/disk
* validator liveness

Include Prometheus/Grafana setup if applicable.

Add alert thresholds:

* no new block for 5 minutes
* peer count < 2
* disk > 80%
* rejected tx spike
* faucet spike
* height divergence between validators

---

# 7. Upgrade / rollback

Create:

```text
docs/testnet/UPGRADE_ROLLBACK.md
```

Include:

* versioning scheme
* release checklist
* canary node upgrade
* validator rolling upgrade
* rollback conditions
* state compatibility checklist
* database migration policy

---

# 8. User guide

Create:

```text
docs/testnet/USER_GUIDE.md
```

Include:

* what Cerulean testnet is
* how to create wallet
* how to request faucet funds
* how to send transfer
* how to view explorer
* known limitations
* how to report bugs

---

# 9. Risk register

Create:

```text
docs/testnet/RISK_REGISTER.md
```

Table:

```text
Risk
Severity
Likelihood
Mitigation
Owner
Status
```

Include at least:

* consensus halt
* faucet abuse
* P2P spam
* disk exhaustion
* validator key compromise
* state divergence
* broken release
* explorer/API outage
* insufficient users
* misleading claims about certification

---

# 10. Optional deployment assets

If possible, add:

```text
deploy/public-testnet/
├── docker-compose.public.yml
├── .env.example
├── nginx.conf
└── prometheus.yml
```

---

# Important language rules

Do NOT claim:

* mainnet-ready
* FIPS certified
* NIST certified
* financially valuable
* investment opportunity

Use:

```text
experimental public testnet
post-quantum aligned
FIPS-oriented architecture
not certified
test tokens have no monetary value
```

---

# Final output format

Report:

1. Testnet docs created
2. Node topology proposed
3. Monitoring plan created
4. Faucet policy created
5. Incident response covered
6. Upgrade/rollback covered
7. User guide created
8. Risk register created
9. Optional deployment assets created
10. Final statement:

```text
Cerulean Ledger is ready for a controlled public testnet launch.
Test tokens have no monetary value.
The network remains experimental and not mainnet-ready.
```

Be strict. The goal is to avoid a chaotic public launch.
