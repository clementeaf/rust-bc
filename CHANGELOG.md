# Changelog

All notable changes to the rust-bc Digital ID System project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (Pending features for next release)

### Changed
- (Pending changes for next release)

### Deprecated
- (Features deprecated in development)

### Removed
- (Features removed in development)

### Fixed
- (Bug fixes in development)

### Security
- (Security patches in development)

---

## [0.1.0] — 2026-06-30 (Target: Q2 2026)

### Added

#### Backend (Rust)

**Storage Tier (Tier 1):**
- RocksDB persistence layer with block storage
- Merkle tree proof generation
- Index management (UTXO, timestamp, account)
- Ledger state management
- Storage error handling with exponential backoff
- 80+ unit tests (90%+ coverage)

**Consensus Tier (Tier 2):**
- DAG (Directed Acyclic Graph) consensus engine
- Slot-based mining with difficulty adjustment
- Fork resolution and canonical path selection
- Byzantine fault tolerance (33% threshold)
- Parallel mining with thread safety
- 120+ unit tests (85%+ coverage)

**Identity Tier (Tier 3):**
- DID (Decentralized Identity) document generation
- Credential issuance, verification, revocation
- Key derivation and rotation
- Ed25519 signature generation/verification
- eIDAS attribute mapping
- 90+ unit tests (88%+ coverage)

**API Tier (Tier 4):**
- REST API gateway (Actix-web)
- JSON request/response serialization
- Parameter validation and error formatting
- JWT authentication with refresh tokens
- Rate limiting (1000 req/min)
- API versioning (semantic)
- 60+ unit tests (80%+ coverage)

#### Frontend (C#/.NET MAUI)

**Persistence Layer (Layer 1):**
- SQLite local database with schema migrations
- AES-256-GCM encryption at rest
- Optimized CRUD operations
- Index management
- 50+ unit tests (85%+ coverage)

**Domain Models Layer (Layer 2):**
- Identity, Account, Transaction, Credential entities
- ValueObjects (Amount, PublicKey, Signature)
- Business rule validators
- Error state handling
- 80+ unit tests (90%+ coverage)

**Services Layer (Layer 3):**
- IdentityService: DID management
- TransactionService: creation, signing, broadcast
- SyncService: multi-device synchronization
- CredentialService: issuance & verification
- HTTP client with retry logic
- Offline queue management
- 70+ unit tests (80%+ coverage)

**ViewModel Layer (Layer 4):**
- MVVM ViewModels for core flows
- Command routing and execution
- Observable state management
- UI state persistence/restoration
- Error notification routing
- 50+ unit tests (75%+ coverage)

**View Layer (Layer 5):**
- XAML pages for onboarding flow
- Transaction creation UI
- Credential management UI
- Deep linking support
- WCAG AA accessibility compliance
- Multi-language support (English, Spanish, German, French)

#### Integration

- REST API contract with 15+ endpoints
- JSON-RPC compatibility layer
- WebSocket support for real-time updates
- Request/response versioning (v1, v2)
- Comprehensive error code catalog (40+ codes)
- API documentation (OpenAPI/Swagger)

#### Compliance & Security

**GDPR Compliance:**
- Data encryption at rest (AES-256-GCM)
- Encryption in transit (TLS 1.3)
- Audit logging with immutable Merkle chain
- Data subject rights (export, deletion, portability)
- 30-day automatic data retention policy
- GDPR impact assessment documented

**eIDAS Roadmap (Phase 1):**
- Credential format compatible with eIDAS Level 3
- Signature algorithm acceptable (EdDSA + SHA-512)
- Attribute schema mappable to eIDAS
- QTSP integration stub (Phase 2+)
- Trust list framework defined

**Security Scanning:**
- Dependency vulnerability scanning (cargo audit)
- SAST (static application security testing)
- Secrets detection (TruffleHog)
- Code quality gates (clippy, StyleCop)
- Pre-commit hooks for developers

#### DevOps & CI/CD

- GitHub Actions workflows (build, test, lint, security)
- Multi-OS testing (Linux, macOS)
- Code coverage tracking (80%+ target)
- Automated pre-commit hooks
- Branch protection rules (main/develop)
- Semantic versioning tags (v#.#.#)
- Blue-green deployment strategy documented

#### Documentation

- Architecture documentation (4 comprehensive guides)
- API contract specification
- Branching strategy guide
- Contributing guidelines
- Development setup instructions
- Testing strategy (test pyramid)
- Phase 2 week-by-week roadmap

### Changed

- (Placeholder for changes in initial release)

### Security

- TLS 1.3 required for all HTTPS connections
- Ed25519 signatures for transaction validation
- AES-256-GCM for data at rest encryption
- JWT tokens with 15-minute expiration
- Rate limiting enabled by default

---

## Release Process

### Version Numbering

- **MAJOR** (X.0.0): Breaking changes, API compatibility breaks
- **MINOR** (0.X.0): New features, backward compatible
- **PATCH** (0.0.X): Bug fixes, security patches

### Release Checklist

Before release, verify:
- [ ] All tests passing (811+ tests)
- [ ] Coverage ≥80% across all components
- [ ] No CRITICAL security vulnerabilities
- [ ] Performance baselines met (1000 TPS, <100ms p99)
- [ ] CHANGELOG.md updated
- [ ] Documentation reviewed
- [ ] Release notes prepared

### Release Candidates

Pre-release versions use format: `v1.0.0-rc.1`, `v1.0.0-rc.2`

Tagged as: `v1.0.0-rc.1` (GitHub tags)

---

## Archive

### Planned Releases (Roadmap)

- **v0.2.0** (Week 6): Consensus + Identity features
- **v0.5.0** (Week 10): Full system integration
- **v1.0.0** (Week 20): Production launch

---

## Contribution

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute changes.

## Contact

For questions about releases or changelog: See [SECURITY.md](SECURITY.md) for security-related changes.

---

**Last Updated:** December 19, 2025  
**Maintainer:** rust-bc team  
**Repository:** https://github.com/your-org/rust-bc
