# Cerulean Ledger — Upgrade & Rollback Policy

---

## Versioning Scheme

```
v0.MINOR.PATCH-testnet
```

- **MINOR**: breaking changes (consensus rules, state format, wire protocol)
- **PATCH**: bug fixes, performance, non-breaking additions

Examples: `v0.1.0-testnet`, `v0.1.1-testnet`, `v0.2.0-testnet`

## Release Checklist

Before deploying any release to testnet:

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] All lib tests pass (`cargo test --lib`)
- [ ] All integration tests pass (adversarial, wallet, API, faucet)
- [ ] Docker image builds successfully
- [ ] Canary node runs for 1 hour without issues
- [ ] CHANGELOG updated
- [ ] Git tag created: `git tag v0.X.Y-testnet`

## Upgrade Procedure

### 1. Canary Node (1 of 3 validators)

```bash
# Build new image
docker compose build validator-1

# Restart only validator-1
docker compose up -d validator-1

# Monitor for 30 minutes
docker logs -f validator-1

# Verify: still producing blocks, peers connected, same height as others
```

### 2. Rolling Upgrade (remaining validators)

```bash
# One at a time, with 10-minute observation between each
docker compose up -d validator-2
sleep 600
# Verify height, peer count, consensus
docker compose up -d validator-3
```

### 3. API and Explorer

```bash
docker compose up -d api-1 explorer-1
```

### 4. Sentries

```bash
docker compose up -d sentry-1 sentry-2
```

## Rollback Conditions

Rollback if ANY of these occur during canary:

- Node crashes within 30 minutes
- Consensus halt (no new blocks)
- State divergence with other validators
- API errors > 1% of requests
- Block time > 60 seconds sustained

## Rollback Procedure

```bash
# Revert to previous image
docker compose down validator-1
docker tag cerulean-ledger:previous cerulean-ledger:latest
docker compose up -d validator-1
```

If state is corrupted by the new version:

```bash
# Delete data and resync from healthy validators
docker compose stop validator-1
docker volume rm public-testnet_validator1-data
docker compose up -d validator-1
```

## State Compatibility

### Non-breaking changes (PATCH)

- New API endpoints
- Performance improvements
- Bug fixes that don't change execution results
- New log messages

No special handling needed. Rolling upgrade safe.

### Breaking changes (MINOR)

- Changes to transaction execution (different balance outcomes)
- New fields required in block/transaction format
- Consensus rule changes
- RocksDB schema changes

Requires coordinated upgrade:
1. Announce upgrade height (e.g., "upgrade at block 10,000")
2. All validators must upgrade before that height
3. If missed, resync from genesis with new version

## Database Migration Policy

- **No automatic migrations** — testnet may be wiped
- If schema changes: document in CHANGELOG, provide `migrate.sh` if feasible
- Default approach: wipe and resync (acceptable for testnet)
