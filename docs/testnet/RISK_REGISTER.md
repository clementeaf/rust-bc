# Cerulean Ledger — Testnet Risk Register

| # | Risk | Severity | Likelihood | Mitigation | Owner | Status |
|---|---|---|---|---|---|---|
| R-01 | Consensus halt (no new blocks) | CRITICAL | Medium | BFT tolerates 1 of 3 crash faults. Runbook covers restart. Alert at 5min stall. | Ops | MITIGATED |
| R-02 | Faucet abuse / drain | HIGH | High | Per-IP limit (10/day), per-address cooldown (100 blocks), daily cap (100K NOTA), disable switch. | Ops | MITIGATED |
| R-03 | P2P spam / DDoS | HIGH | Medium | Sentry nodes shield validators. Rate limiting on API. Peer allowlist available. Firewall rules documented. | Ops | MITIGATED |
| R-04 | Disk exhaustion | MEDIUM | Low | Alert at 80%. ~1 GB/month growth. 50 GB initial allocation. Runbook covers pruning. | Ops | MITIGATED |
| R-05 | Validator key compromise | CRITICAL | Low | Keys stored only in container memory (ZeroizeOnDrop). No key export API. Testnet keys have no value. | Sec | ACCEPTED |
| R-06 | State divergence between validators | CRITICAL | Low | Deterministic execution tested. State hash comparison in runbook. Resync procedure documented. | Dev | MITIGATED |
| R-07 | Broken release (consensus-breaking bug) | HIGH | Medium | Canary node upgrade. 30-min observation. Rollback procedure documented. Integration test suite (58 tests). | Dev | MITIGATED |
| R-08 | Explorer / API outage | MEDIUM | Medium | API and explorer on separate nodes from validators. Restart doesn't affect consensus. | Ops | ACCEPTED |
| R-09 | Insufficient users / no feedback | LOW | Medium | Blockchain Chamber outreach. User Guide published. Faucet available. Demo script exists. | PM | OPEN |
| R-10 | Misleading claims about certification | HIGH | Low | All docs state: "experimental", "not FIPS certified", "not mainnet-ready", "tokens have no value". Language rules enforced in PROMPT20. | Legal | MITIGATED |
| R-11 | Memory leak / OOM crash | MEDIUM | Low | 1595 tests including stress tests. Mutex poison recovery on all locks. Monitor memory via Grafana. | Dev | MITIGATED |
| R-12 | Chain reorg deeper than snapshot depth | MEDIUM | Low | ReorgManager retains configurable snapshots. Default depth sufficient for BFT (instant finality). | Dev | MITIGATED |
| R-13 | RocksDB corruption | MEDIUM | Low | Atomic WriteBatch for all state changes. Resync from genesis documented. Backups in runbook. | Ops | MITIGATED |
| R-14 | Time-dependent test failures | LOW | Low | Deterministic seeds for chaos tests. No clock-dependent assertions in unit tests. | Dev | MITIGATED |

## Risk Severity Scale

| Level | Impact |
|---|---|
| CRITICAL | Network halt, state corruption, data loss |
| HIGH | Degraded service, partial outage, abuse |
| MEDIUM | Single component failure, recoverable |
| LOW | Minor inconvenience, cosmetic |

## Review Schedule

- Before each phase transition (dry run → alpha → public)
- After any CRITICAL or HIGH incident
- Monthly during public testnet operation
