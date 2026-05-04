# Release Checklist — v0.1.0-pqc-testnet

## Quality Gate

- [x] cargo fmt: clean
- [x] cargo clippy -- -D warnings: clean
- [x] cargo test --lib: 1636 passed, 0 failed
- [x] cargo test --test network_e2e: 6 passed
- [x] cargo test --test chaos_network: 11 passed
- [x] cargo test --test adversarial_crypto_txs: 17 passed
- [x] cargo test --test bft_e2e: 16 passed
- [x] Total: 1686+ tests, 0 failures

## Validation

- [x] Manual testnet: 3 nodes, 2 blocks, balances consistent across all nodes
- [x] Tx propagation verified
- [x] Block sync verified
- [x] Invalid block rejection verified
- [x] Nonce monotonicity verified (nonce=2 after 2 txs)

## Security

- [x] Audit findings: 0 CRITICAL, 0 HIGH open (P12 audit)
- [x] Canonical binary serialization (no JSON in consensus paths)
- [x] Deterministic testnet key clearly marked DO NOT USE IN PRODUCTION
- [x] No secrets in source code

## Documentation

- [x] README quickstart section
- [x] Operational runbook (docs/testnet-manual-runbook.md)
- [x] start-testnet.sh script
- [x] demo-flow.sh script
- [x] CHANGELOG up to date

## Limitations

- Not externally audited
- TCP only (no TLS)
- No persistence (in-memory)
- No dynamic peer discovery
- No PoW/PoS consensus (single producer)
- Deterministic testnet key (not for production)
