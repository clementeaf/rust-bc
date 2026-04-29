# Cerulean Ledger — Public Testnet Launch Plan

**Status**: Experimental public testnet — not mainnet-ready.
**Test tokens have no monetary value.**

---

## Purpose

Validate the Cerulean Ledger protocol under real-world conditions with external participants before any mainnet consideration. Specifically:

- Verify BFT consensus stability over days/weeks
- Stress-test the native transfer pipeline under organic load
- Validate P2P sync, reorg safety, and state determinism across independent operators
- Exercise the faucet, explorer, and wallet CLI with real users
- Collect feedback on UX, API ergonomics, and documentation gaps

## Scope

| In scope | Out of scope |
|---|---|
| Native NOTA transfers | Smart contract execution |
| BFT consensus (3–5 validators) | EVM integration |
| Faucet-funded test accounts | Real-value tokens |
| Explorer and wallet CLI | HD wallets / BIP-39 |
| Post-quantum signatures (Ed25519 + ML-DSA-65) | FIPS certification claims |
| RocksDB persistence | Sharding / L2 |

## What is NOT guaranteed

- **Uptime**: the network may be reset without notice
- **State preservation**: chain may be wiped between phases
- **Backwards compatibility**: API and wire protocol may change
- **Security**: the system has not been externally audited
- **Token value**: NOTA test tokens have zero monetary value

## Expected Users

| Phase | Audience | Count |
|---|---|---|
| Private dry run | Core team only | 2–3 |
| Closed alpha | Invited developers + Blockchain Chamber members | 10–20 |
| Public testnet | Anyone with the endpoint URL | 50–200 |
| Incentivized testnet (future) | Validators with stake | TBD |

## Launch Phases

### Phase 1 — Private Dry Run (Week 1)

- 3 validator nodes on team-controlled infrastructure
- Internal team runs wallet CLI, explorer, faucet
- Verify: blocks produced, sync works, no consensus halt
- **Exit criteria**: 48h continuous operation with no intervention

### Phase 2 — Closed Alpha (Week 2–3)

- Invite 10–20 external developers
- Public API endpoint (rate-limited)
- Faucet enabled with conservative limits
- Collect bug reports via GitHub Issues
- **Exit criteria**: 7 days stable, <5 critical bugs, sync verified by 3+ external nodes

### Phase 3 — Public Testnet (Week 4+)

- Announce publicly (Blockchain Chamber, social media)
- Explorer available at public URL
- User Guide published
- Faucet with stricter limits (per-IP, daily cap)
- **Exit criteria**: 30 days stable, community validators running

### Phase 4 — Incentivized Testnet (Future)

- Requires mainnet readiness milestones
- Not part of this plan

## Success Criteria

| Metric | Target |
|---|---|
| Uptime | >95% over first 48h |
| Sync reliability | New node syncs to tip within 10 minutes |
| Faucet availability | <1% request failures |
| Consensus halt | Zero during Phase 2+ |
| State divergence | Zero (verified by state hash comparison) |
| Mean block time | 15s ± 5s |
| Explorer response | <2s p95 |

---

*Cerulean Ledger is an experimental post-quantum-aligned blockchain. It is not FIPS certified, not mainnet-ready, and test tokens have no monetary value.*
