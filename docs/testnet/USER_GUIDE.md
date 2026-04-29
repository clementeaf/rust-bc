# Cerulean Ledger Testnet — User Guide

**Test tokens have no monetary value. The network is experimental.**

---

## What is Cerulean Testnet?

Cerulean Ledger is a post-quantum-aligned Layer 1 blockchain built in Rust. The testnet lets you experiment with:

- Creating wallets with Ed25519 or ML-DSA-65 (post-quantum) keys
- Sending native NOTA token transfers
- Exploring blocks, transactions, and account balances
- Using the faucet to get free test tokens

## 1. Create a Wallet

### Using the CLI

```bash
# Generate an Ed25519 keypair
cargo run --bin wallet -- generate

# Output:
# Algorithm:   Ed25519
# Address:     a1b2c3d4e5f6...  (40 hex chars)
# Public key:  <hex>
# Private key: <hex>
```

Save your private key securely — it cannot be recovered.

For post-quantum keys:
```bash
cargo run --bin wallet -- generate --pqc
```

### Derive address from existing public key

```bash
cargo run --bin wallet -- address <pubkey_hex>
```

## 2. Request Faucet Funds

### Via CLI

```bash
./scripts/testnet.sh faucet <your_address>
```

### Via API

```bash
curl -X POST https://testnet.cerulean.example/api/v1/faucet/drip \
  -H "Content-Type: application/json" \
  -d '{"address": "YOUR_ADDRESS"}'
```

### Via Explorer

Navigate to the Crypto page and use the Faucet section.

**Limits**: 1,000 NOTA per drip, 10 drips per IP per day, 25-minute cooldown per address.

## 3. Check Your Balance

```bash
# CLI
cargo run --bin wallet -- balance <your_address>

# Or via script
./scripts/testnet.sh balance <your_address>

# Or via API
curl https://testnet.cerulean.example/api/v1/accounts/<your_address>
```

## 4. Send a Transfer

```bash
# Using the wallet CLI
cargo run --bin wallet -- transfer <from_address> <to_address> <amount> \
  --key <your_private_key_hex> \
  --fee 5

# Or via testnet script
./scripts/testnet.sh transfer <from> <to> <amount>
```

### Transfer requirements

- Sender must have enough balance for `amount + fee`
- Fee must be >= current base fee (usually 1 NOTA)
- Nonce is auto-fetched from the network
- Transactions are queued in the mempool and included in the next block

## 5. View the Explorer

Open the block explorer in your browser:

```
https://testnet.cerulean.example
```

Navigate to the **Crypto** page to see:
- Mempool status and base fee
- Account lookup (enter any address)
- Faucet (request tokens)
- Recent blocks

## Known Limitations

- **No smart contracts**: only native NOTA transfers
- **No HD wallets**: each key must be managed individually
- **Mempool not persistent**: pending transactions lost on node restart
- **Signature not verified at API**: verification happens at block production
- **Chain may be reset**: testnet state is not permanent

## Reporting Bugs

Please report bugs via GitHub Issues:

1. Describe what you did
2. What you expected
3. What actually happened
4. Include your transaction ID if relevant (never share your private key)

## FAQ

**Q: Are test tokens worth anything?**
A: No. NOTA test tokens have zero monetary value. They exist only for testing.

**Q: Is this FIPS certified?**
A: No. The architecture is FIPS-oriented (using ML-DSA-65, ML-KEM-768, SHA3-256) but has not been submitted for FIPS 140-3 certification.

**Q: Can I run my own node?**
A: Yes. Clone the repo, build with `cargo build`, and connect to the testnet bootstrap nodes. See the deployment docs.

**Q: What happens if I lose my private key?**
A: Your test tokens are lost. There is no recovery mechanism. Generate a new key and request from the faucet.
