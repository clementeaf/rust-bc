# rust-bc Digital ID System

[![Build](https://github.com/your-org/rust-bc/workflows/Build/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/build.yml)
[![Test](https://github.com/your-org/rust-bc/workflows/Test/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/test.yml)
[![Lint](https://github.com/your-org/rust-bc/workflows/Lint/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/lint.yml)
[![Security](https://github.com/your-org/rust-bc/workflows/Security/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/security.yml)

**Digital ID System for rust-bc Blockchain** — A comprehensive platform integrating blockchain-based identity management with GDPR/eIDAS compliance.

**Status:** Phase 2 Implementation (Weeks 1-20)  
**Technology:** Rust (backend and API server)  
**Target Launch:** Q2 2026

---

## Quick Start

### Prerequisites

- **Rust 1.75.0+** (automatically enforced via `rust-toolchain.toml`)
- **Git** with pre-commit hooks
- **macOS** / **Linux** (Windows support via WSL2)

### Development Environment Setup

1. **Clone repository:**
   ```bash
   git clone https://github.com/your-org/rust-bc.git
   cd rust-bc
   ```

2. **Install dependencies:**
   ```bash
   # macOS
   brew install rustup pre-commit
   rustup update

   # Linux
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   pip install pre-commit
   ```

3. **Setup pre-commit hooks:**
   ```bash
   pre-commit install
   pre-commit run --all-files  # Run all checks once
   ```

4. **Verify toolchain:**
   ```bash
   rustc --version  # Should be 1.75.0
   cargo --version
   ```

5. **Build & test locally:**
   ```bash
   cargo build --release
   cargo test --all
   ```

---

## Project Structure

```
rust-bc/
├── src/
│   ├── storage/               # Tier 1: RocksDB persistence
│   ├── consensus/             # Tier 2: DAG consensus engine
│   ├── identity/              # Tier 3: DID & credentials
│   ├── api/                   # Tier 4: REST gateway
│   └── lib.rs
├── tests/
│   ├── storage/
│   ├── consensus/
│   ├── identity/
│   └── api/
├── .github/
│   ├── workflows/             # CI/CD pipelines
│   └── CODEOWNERS            # Ownership matrix
├── docs/                      # All extended docs (analysis, archive, commercial, dev, technical)
├── rust-toolchain.toml        # Rust version pinning
├── Cargo.toml                 # Rust dependencies
├── CONTRIBUTING.md            # Contribution guidelines
└── README.md                  # This file
```

---

## Architecture

### Backend (Rust) — 4 Tiers

| Tier | Component | Responsibility | Tech |
|------|-----------|-----------------|------|
| 1 | **Storage** | Block persistence, indexing | RocksDB |
| 2 | **Consensus** | DAG mining, fork resolution | Custom DAG |
| 3 | **Identity** | DID, credentials, verification | Did-key |
| 4 | **API** | REST gateway, serialization | Actix-web |

### Integration

- **Protocol:** REST/HTTPS + TLS 1.3
- **Auth:** JWT + refresh tokens
- **Versioning:** Semantic versioning (`v1.2.3`)
- **Error Handling:** Standardized error codes

---

## TLS Configuration

Both the HTTP API server and the P2P layer support TLS. TLS is **opt-in**: if the environment variables below are not set the node runs over plain TCP/HTTP.

### Environment variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `TLS_CERT_PATH` | For TLS | — | Path to the PEM certificate file. Must be set together with `TLS_KEY_PATH`. |
| `TLS_KEY_PATH` | For TLS | — | Path to the PEM private key file. Must be set together with `TLS_CERT_PATH`. |
| `TLS_VERIFY_PEER` | No | `true` | Set to `false` to disable peer certificate verification on outbound P2P connections. **Only for development/testing.** |
| `TLS_CA_CERT_PATH` | No | — | Path to a custom CA certificate (PEM). Outbound connections verify peers against this CA instead of the Mozilla root store. Required when `TLS_MUTUAL=true`. |
| `TLS_MUTUAL` | No | `false` | Set to `true` to enable mutual TLS (mTLS): the server requires a client certificate signed by `TLS_CA_CERT_PATH`. |
| `TLS_PINNED_CERTS` | No | — | Comma-separated list of SHA-256 fingerprints (hex, 64 chars each) of allowed peer certificates. When set, connections are rejected unless the peer cert matches one of the pins. |
| `TLS_RELOAD_INTERVAL` | No | — | Interval in seconds for automatic certificate rotation. When set, the node validates the cert files on disk at each tick and restarts gracefully if they have changed. |
| `TLS_OCSP_STAPLE_PATH` | No | — | Path to a DER-encoded OCSP response file. When set, the server attaches the staple to every TLS handshake so clients do not need to contact the CA's OCSP endpoint. |

### Quick examples

**Production node with mTLS:**
```bash
export TLS_CERT_PATH=/etc/rust-bc/node.crt
export TLS_KEY_PATH=/etc/rust-bc/node.key
export TLS_CA_CERT_PATH=/etc/rust-bc/ca.crt
export TLS_MUTUAL=true
cargo run --release
```

**Production node with certificate pinning:**
```bash
export TLS_CERT_PATH=/etc/rust-bc/node.crt
export TLS_KEY_PATH=/etc/rust-bc/node.key
export TLS_CA_CERT_PATH=/etc/rust-bc/ca.crt
# Generate fingerprint: openssl x509 -in peer.crt -outform DER | sha256sum
export TLS_PINNED_CERTS=abc123...def456,789abc...012def
cargo run --release
```

**Production node with OCSP stapling:**
```bash
# Generate the staple (refresh daily via cron)
openssl ocsp -issuer ca.pem -cert node.crt \
    -url http://ocsp.example.com -respout /etc/rust-bc/ocsp.der

export TLS_CERT_PATH=/etc/rust-bc/node.crt
export TLS_KEY_PATH=/etc/rust-bc/node.key
export TLS_OCSP_STAPLE_PATH=/etc/rust-bc/ocsp.der
cargo run --release
```

**Development / localhost (self-signed cert, skip peer verification):**
```bash
export TLS_CERT_PATH=tests/fixtures/test_cert.pem
export TLS_KEY_PATH=tests/fixtures/test_key.pem
export TLS_VERIFY_PEER=false
cargo run
```

**No TLS (plain TCP + HTTP):**
```bash
# Simply do not set TLS_CERT_PATH or TLS_KEY_PATH
cargo run
```

### Generating a self-signed certificate for testing

```bash
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
  -keyout key.pem -out cert.pem -days 3650 -nodes -subj '/CN=localhost'
```

### Certificate rotation

The node supports zero-downtime certificate rotation via two mechanisms:

**Manual rotation (SIGHUP):**

Replace the cert/key files on disk, then send `SIGHUP` to the process. The node
validates the new files and, if they are valid, shuts down gracefully so the
process supervisor (systemd, PM2, etc.) can restart it with the new certificates.

```bash
# Replace certs on disk, then:
kill -HUP $(pidof rust-bc)
```

If the new cert files are invalid or unreadable, the node logs the error and
continues serving with the existing certificates.

**Automatic rotation (`TLS_RELOAD_INTERVAL`):**

```bash
export TLS_RELOAD_INTERVAL=3600   # check every hour
```

At each interval the node validates the cert files on disk. If they differ
(e.g. renewed by certbot/ACME), the node stops gracefully for the supervisor
to restart it. If validation fails the node keeps running and retries at the
next tick.

> **Note:** Setting only one of `TLS_CERT_PATH` / `TLS_KEY_PATH` is an error and the node will refuse to start.

---

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b feature/ws1-storage-rocksdb-adapters develop
```

**Branch naming:** `<type>/<component>-<description>`
- Types: `feature`, `bugfix`, `hotfix`, `release`, `docs`, `spike`
- Components: `storage`, `consensus`, `identity`, `api`, `persistence`, etc.

### 2. Make Changes & Commit

```bash
# Pre-commit hooks run automatically
git add .
git commit -m "feat(storage): add RocksDB persistence adapter

Implement storage tier abstraction with RocksDB backend.
Supports block append, proof generation, index queries.

- Factory pattern for adapter instantiation
- Error handling with exponential backoff
- Audit logging per operation

Fixes #42
Co-Authored-By: Warp <agent@warp.dev>"
```

**Commit format:**
```
<type>(<scope>): <subject>

<body>

<footer>
```

### 3. Push & Create Pull Request

```bash
git push origin feature/ws1-storage-rocksdb-adapters
# Create PR on GitHub
```

**PR Requirements:**
- ✅ All CI checks passing (build, test, lint, security)
- ✅ Test coverage ≥80% (delta)
- ✅ ≥1 code review approval
- ✅ CODEOWNERS approval for critical paths
- ✅ Linked to GitHub issue

### 4. Merge to Develop

After approval:
```bash
# GitHub merges (squash or rebase)
# CI/CD deploys to staging
# Branch auto-deleted
```

### 5. Release Cycle (Every 2 Weeks)

```bash
# Create release branch
git checkout -b release/v1.0.0 develop

# Update versions
# - Cargo.toml: version = "1.0.0"

# Update CHANGELOG.md

# Create PR: release/v1.0.0 → main
# After merge:
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# GitHub Actions deploys to production (canary 5%)
```

---

## Testing

### Unit Tests

```bash
cargo test --lib --all
```

**Target Coverage:** 80%+ overall (unit: 75%, service: 20%, integration: 5%)

### Integration Tests

```bash
cargo test --all
```

### Load Testing

```bash
# 1000 TPS sustained (Week 15)
# Run via GitHub Actions: .github/workflows/test.yml
```

### Pre-commit Local Testing

```bash
pre-commit run --all-files
```

---

## Code Quality

### Linting

**Rust:**
```bash
cargo fmt --all
cargo clippy --all -- -D warnings
```

### Security Scanning

```bash
# Secrets detection
cargo install cargo-audit
cargo audit
```

### Pre-commit Automation

```bash
# All checks run automatically on commit:
# - Format check (rustfmt)
# - Lint (clippy)
# - Secrets scan
# - Branch name validation
# - Commit message validation
```

---

## CI/CD Pipeline

### Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| **Build** | Push/PR | Compile Rust |
| **Test** | Push/PR | Run unit tests, coverage |
| **Lint** | Push/PR | Code style checks |
| **Security** | Push/PR + daily | Vulnerability scanning |

### Branch Protection Rules

| Branch | Reviews | Status Checks | CODEOWNERS | Up-to-date |
|--------|---------|---------------|-----------|-----------|
| `main` | ≥2 | Yes | Yes | Yes |
| `develop` | ≥1 | Yes | Selective | Yes |
| `feature/*` | ≥1 | Yes | No | Yes |

### Deployment Pipeline

```
feature/* → PR → develop → staging → main → production (canary 5% → 100%)
```

---

## Documentation

- **Index:** [docs/README.md](docs/README.md)
- **Architecture studies:** [docs/analysis/](docs/analysis/)
- **Branching strategy:** [docs/dev/BRANCHING_STRATEGY.md](docs/dev/BRANCHING_STRATEGY.md)
- **Onboarding:** [docs/dev/ONBOARDING.md](docs/dev/ONBOARDING.md)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)

Optional references (if present in tree): `docs/ARCHITECTURE.md`, `docs/API_CONTRACT.md`, `docs/GDPR.md`, `docs/eIDAS_ROADMAP.md`.

---

## Team & Ownership

See [.github/CODEOWNERS](.github/CODEOWNERS) for tier/layer ownership and review requirements.

**Workstreams:**
- **WS1:** Backend Storage & Consensus (2 engineers)
- **WS2:** Backend Identity & API (1-2 engineers)
- **WS3:** Client / UX (future; not part of this repository)
- **WS4:** DevEx — CI/CD, Tooling (1 engineer)
- **WS5:** Compliance & Security (1 officer, shared)

---

## Common Tasks

### Add a New Dependency

**Rust:**
```bash
cargo add serde --features derive
cargo update
```

### Run Specific Tests

```bash
cargo test storage:: -- --nocapture
cargo test consensus::fork_resolution
```

### Debug Build

```bash
cargo build --debug
RUST_LOG=debug cargo run
```

### Update Toolchain

```bash
rustup update 1.75.0
rustup component add rustfmt clippy
```

---

## Troubleshooting

### Build Fails: "Incompatible Rust Version"

```bash
# Verify toolchain
rustc --version  # Should be 1.75.0

# Force update
rustup update 1.75.0
```

### Pre-commit Hooks Slow

```bash
# Install optional tools
cargo install cargo-audit --locked
```

### CI Pipeline Blocked

1. Check GitHub Actions logs: **Actions** tab
2. Common issues:
   - Missing secrets (set in GitHub → Settings → Secrets)
   - Flaky tests (check logs, add retry logic)
   - Dependency conflict (update Cargo.lock / packages.lock)

### Secrets Accidentally Committed

```bash
# If local commit only
git reset HEAD~1  # Undo commit
git add .gitignore
git commit -m "Remove secrets"

# If pushed to remote
# Contact maintainers immediately for emergency remediation
```

---

## Phase 2 Timeline

| Weeks | Focus | Deliverables |
|-------|-------|--------------|
| 1-2 | **Foundation** | CI/CD, Storage tests | 45% coverage |
| 3-6 | **Expansion** | Consensus, Identity | 70% coverage |
| 7-10 | **Integration** | Full stack tests, Load testing | 78% coverage |
| 11-18 | **Refinement** | Edge cases, GDPR/eIDAS | 82% coverage |
| 19-20 | **Release** | Staging soak, Production launch | 85% coverage |

**Success Criteria:**
- ✅ 80%+ code coverage
- ✅ 1000 TPS throughput
- ✅ <100ms p99 latency
- ✅ Zero CRITICAL vulnerabilities
- ✅ GDPR/eIDAS compliance verified

---

## Support

- **Issues:** [GitHub Issues](https://github.com/your-org/rust-bc/issues)
- **Discussions:** [GitHub Discussions](https://github.com/your-org/rust-bc/discussions)
- **Slack:** #rust-bc channel
- **Docs:** See links above

---

## License

[Your License Here]

---

**Last Updated:** December 19, 2025  
**Phase:** 2 Implementation  
**Maintained By:** DevEx + Tech Lead
