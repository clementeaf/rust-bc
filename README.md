# rust-bc Digital ID System

[![Build](https://github.com/your-org/rust-bc/workflows/Build/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/build.yml)
[![Test](https://github.com/your-org/rust-bc/workflows/Test/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/test.yml)
[![Lint](https://github.com/your-org/rust-bc/workflows/Lint/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/lint.yml)
[![Security](https://github.com/your-org/rust-bc/workflows/Security/badge.svg)](https://github.com/your-org/rust-bc/actions/workflows/security.yml)

**Digital ID System for rust-bc Blockchain** — A comprehensive platform integrating blockchain-based identity management with GDPR/eIDAS compliance.

**Status:** Phase 2 Implementation (Weeks 1-20)  
**Technology:** Rust (backend), C# MAUI (frontend)  
**Target Launch:** Q2 2026

---

## Quick Start

### Prerequisites

- **Rust 1.75.0+** (automatically enforced via `rust-toolchain.toml`)
- **.NET SDK 8.0.0+** (automatically enforced via `global.json`)
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
   dotnet --version # Should be 8.0.0+
   cargo --version
   ```

5. **Build & test locally:**
   ```bash
   # Backend
   cargo build --release
   cargo test --all

   # Frontend
   dotnet build --configuration Release
   dotnet test --configuration Release
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
├── src/Features/              # Frontend (C# MAUI)
│   ├── Persistence/           # Layer 1: SQLite
│   ├── Domain/                # Layer 2: Models
│   ├── Services/              # Layer 3: Business logic
│   ├── UI/ViewModels/         # Layer 4: MVVM state
│   └── UI/Views/              # Layer 5: XAML
├── .github/
│   ├── workflows/             # CI/CD pipelines
│   └── CODEOWNERS            # Ownership matrix
├── rust-toolchain.toml        # Rust version pinning
├── global.json                # .NET version pinning
├── Cargo.toml                 # Backend dependencies
├── *.csproj                   # Frontend projects
├── BRANCHING_STRATEGY.md      # Git workflow
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

### Frontend (C#) — 5 Layers

| Layer | Component | Responsibility | Tech |
|-------|-----------|-----------------|------|
| 1 | **Persistence** | SQLite, encryption | SQLite-net |
| 2 | **Models** | Domain objects, validation | C# records |
| 3 | **Services** | Business logic, HTTP | HttpClient |
| 4 | **ViewModel** | UI state, commands | MVVM Community Toolkit |
| 5 | **Views** | Layout, binding | MAUI/XAML |

### Integration

- **Protocol:** REST/HTTPS + TLS 1.3
- **Auth:** JWT + refresh tokens
- **Versioning:** Semantic versioning (`v1.2.3`)
- **Error Handling:** Standardized error codes

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
# - *.csproj: <Version>1.0.0</Version>

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
# Backend
cargo test --lib --all

# Frontend
dotnet test --configuration Release
```

**Target Coverage:** 80%+ overall (unit: 75%, service: 20%, integration: 5%)

### Integration Tests

```bash
# Full stack
cargo test --all
dotnet test --configuration Release
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

**C#:**
```bash
dotnet format --verify-no-changes
dotnet build /p:EnforceCodeStyleInBuild=true
```

### Security Scanning

```bash
# Secrets detection
cargo install cargo-audit
cargo audit

# Dependency scan
dotnet list package --outdated
```

### Pre-commit Automation

```bash
# All checks run automatically on commit:
# - Format check (rustfmt, dotnet format)
# - Lint (clippy, StyleCop)
# - Secrets scan
# - Branch name validation
# - Commit message validation
```

---

## CI/CD Pipeline

### Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| **Build** | Push/PR | Compile Rust + C# |
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

- **Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **API Contract:** [docs/API_CONTRACT.md](docs/API_CONTRACT.md)
- **GDPR Compliance:** [docs/GDPR.md](docs/GDPR.md)
- **eIDAS Roadmap:** [docs/eIDAS_ROADMAP.md](docs/eIDAS_ROADMAP.md)
- **Branching Strategy:** [BRANCHING_STRATEGY.md](BRANCHING_STRATEGY.md)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)

---

## Team & Ownership

See [.github/CODEOWNERS](.github/CODEOWNERS) for tier/layer ownership and review requirements.

**Workstreams:**
- **WS1:** Backend Storage & Consensus (2 engineers)
- **WS2:** Backend Identity & API (1-2 engineers)
- **WS3:** Frontend MAUI App (2 engineers)
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

**C#:**
```bash
dotnet add package Newtonsoft.Json
```

### Run Specific Tests

```bash
# Backend
cargo test storage:: -- --nocapture
cargo test consensus::fork_resolution

# Frontend
dotnet test --filter "DisplayName~Persistence" --configuration Release
```

### Debug Build

```bash
# Rust
cargo build --debug
RUST_LOG=debug cargo run

# C#
dotnet build --configuration Debug
```

### Update Toolchain

```bash
# Rust
rustup update 1.75.0
rustup component add rustfmt clippy

# .NET
dotnet sdk check
# Edit global.json manually for version bump
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
