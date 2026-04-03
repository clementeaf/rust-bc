# Changelog

All notable changes to the rust-bc Digital ID System project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

**Consensus — Fase G Fork Resolution (2026-04-03)**

- `src/consensus/dag.rs`: selección de cadena canónica y resolución de forks
  - `Dag::subtree_weight()`: cuenta descendientes totales de un bloque
  - `Dag::canonical_chain()`: recorre el DAG desde genesis eligiendo el hijo con mayor peso; desempate por hash
  - `Dag::resolve_fork()`: dado un conjunto de tips competidores, retorna el que pertenece a la cadena canónica
- `src/consensus/fork_choice.rs`: módulo nuevo
  - `ForkChoiceRule`: enum con dos estrategias — `HeaviestSubtree` (default) y `LongestChain`
  - `ForkChoice`: engine configurable que expone `canonical_chain()` y `resolve()`
- 33 tests nuevos (22 en `dag`, 11 en `fork_choice`); todos pasando

**Consensus — Fase H ConsensusEngine (2026-04-03)**

- `src/consensus/engine.rs`: módulo nuevo
  - `ConsensusEngine`: agrupa `Dag`, `ForkChoice` y `SlotScheduler` en un único punto de entrada
  - `accept_block()`: valida el bloque (formato, firma, parent, slot) e inserta en el DAG
  - `canonical_tip()`: retorna el hash del tip canónico actual
  - `canonical_chain()`: retorna el path completo genesis → tip
  - `ConsensusError`: errores tipados (`InvalidBlock`, `DagError`) vía `thiserror`
- 11 tests (accept, reject ×5, canonical tip/chain, fork)

**Storage — Fase I: MemoryStore + API (2026-04-03)**

- `src/storage/memory.rs`: `MemoryStore` — `BlockStore` in-memory con `Mutex` interno
- `src/storage/traits.rs`: impl `BlockStore` para `Arc<T>` — compartir store entre engine y API
- `src/consensus/engine.rs`: `ConsensusEngine::with_store()` — persiste bloques aceptados
- `src/app_state.rs`: campo `store: Option<Arc<dyn BlockStore>>`
- `src/api/handlers/blocks.rs`: `GET /api/v1/store/blocks/{height}` y `/latest`
- `tests/store_blocks_api_test.rs`: 7 tests de integración actix-web

**Storage — Fase III: backend switcheable (2026-04-03)**

- `src/main.rs`: `AppState.store` ya no es `None` fijo; se inicializa según `STORAGE_BACKEND`
  - `memory` (default) → `MemoryStore`
  - `rocksdb` → `RocksDbBlockStore` en `ROCKSDB_PATH` (default `./data/blocks`); fallback a `MemoryStore` si falla la apertura

**Storage — Fase V: REST endpoints store para transacciones, identidades y credenciales (2026-04-03)**

- `src/api/handlers/transactions.rs`: `POST /api/v1/store/transactions`, `GET /api/v1/store/transactions/{tx_id}`
- `src/api/handlers/identity.rs`: `POST /api/v1/store/identities`, `GET /api/v1/store/identities/{did}`
- `src/api/handlers/credentials.rs`: `POST /api/v1/store/credentials`, `GET /api/v1/store/credentials/{cred_id}`
- `src/api/routes.rs`: tres nuevos scopes `store_transactions_routes`, `store_identities_routes`, `store_credentials_routes`
- Todos los handlers delegan a `state.store` siguiendo el patrón de `store_get_block`; responden 404 si el store no está configurado

**Storage — Índices secundarios por rango (2026-04-03)**

- `src/storage/adapters.rs`: nueva CF `tx_by_block`
  - Key schema: `{012_padded_height}:{tx_id}` → value vacío; prefijo fijo garantiza colocalización y orden lexicográfico
  - `write_transaction` y `write_batch` escriben la entrada de índice en el mismo `WriteBatch` (atómico)
  - `transactions_by_block_height(height)`: prefix scan con `IteratorMode::From`, extrae `tx_id` del key y resuelve la tx en CF `transactions`
- `src/storage/memory.rs`: misma query implementada como scan lineal sobre el `HashMap` filtrando por `block_height`
- `src/storage/traits.rs`: `BlockStore::transactions_by_block_height` añadido al trait + delegación en el blanket `Arc<T>`
- 9 tests nuevos (5 adapter, 3 memory, 1 trait): formato de key, vaciado para altura desconocida, filtrado correcto, no bleed-over entre alturas adyacentes, batch indexing; 463 tests totales pasando

**Storage — Fase IV: Column Families en RocksDB (2026-04-03)**

- `src/storage/adapters.rs`: migración de prefijos de clave a Column Families dedicadas
  - 5 CFs: `blocks`, `transactions`, `identities`, `credentials`, `meta`
  - `DB::open_cf_descriptors` con `create_missing_column_families(true)` — compatible con DBs nuevas y existentes
  - Helpers privados `cf_blocks()` / `cf_transactions()` / etc. con error tipado `ColumnFamilyNotFound`
  - Todas las operaciones usan `put_cf` / `get_cf`; `WriteBatch` usa `put_cf` por CF
  - Claves sin prefijo (el CF provee el namespace); bloques usan altura zero-padded `000000000001`
  - 17 tests: roundtrip por tipo, aislamiento entre CFs, `reopen` con datos persistidos

**Storage — Fase II: RocksDB (2026-04-03)**

- `Cargo.toml`: dependencia `rocksdb = "0.22"`
- `src/storage/traits.rs`: serde derives en `Transaction`, `IdentityRecord`, `Credential`
- `src/storage/adapters.rs`: `RocksDbBlockStore` con implementación real
  - Serialización JSON por clave prefijada (`BLK:`, `TX:`, `DID:`, `CRED:`)
  - `write_batch` atómico vía `WriteBatch`
  - `META:latest_height` — tracking persistente de la altura máxima
  - 13 tests unitarios con `TempDir` (aislados entre runs)

**CI — fix toolchain (2026-04-03)**

- `.github/workflows/`: añadido `toolchain: stable` en los 4 workflows (`build`, `security`, `lint`, `test`) — `dtolnay/rust-toolchain@master` requiere el input explícito

**TLS — Fase A (2026-04-02)**

- `src/tls.rs`: cargar PEM cert+key, construir `ServerConfig` y `ClientConfig`
  - `PeerVerification`: `Full` (WebPKI roots o CA propia) y `Dangerous` (solo dev)
  - Variables de entorno: `TLS_CERT_PATH`, `TLS_KEY_PATH`, `TLS_VERIFY_PEER`, `TLS_CA_CERT_PATH`
- `src/network.rs`: conexiones P2P envueltas en TLS vía `TlsAcceptor` / `TlsConnector`
- `tests/tls_p2p_integration_test.rs`: 3 tests de integración (handshake, rechazo TCP plano, echo bidireccional)
- 20 tests unitarios en `tls.rs` y `network.rs`; todos pasando
- `README.md`: sección "TLS Configuration" con tabla de variables y ejemplos
- Dependencias: `rustls 0.23`, `rustls-pemfile 2`, `tokio-rustls 0.26`, `webpki-roots 0.26`

**TLS — Fase B mTLS (2026-04-02)**

- `src/tls.rs`: autenticación mutua de nodos (mTLS)
  - `build_server_config_mtls`: exige certificado cliente firmado por la CA
  - `build_client_config_mtls`: presenta cert+key propio al servidor y verifica la CA
  - `load_tls_config_from_env` / `load_client_config_from_env` leen `TLS_MUTUAL=true` y `TLS_CA_CERT_PATH`
  - Error `MtlsMissingCa`: falla explícito si `TLS_MUTUAL=true` sin CA configurada
- `tests/tls_p2p_integration_test.rs`: 2 tests mTLS P2P
  - Handshake mTLS exitoso entre dos nodos con cert válido
  - Servidor rechaza cliente sin certificado
- 21 tests unitarios en `tls.rs`; todos pasando (26 en total con integración)
- Dependencia dev: `rcgen 0.13` para generar certs en tests

**TLS — Fase C Certificate Pinning (2026-04-02)**

- `src/tls.rs`: allowlist de certificados por fingerprint SHA-256
  - `CertPinConfig`: parse y validación de fingerprints hex; desactivado si la lista está vacía
  - `PinningServerCertVerifier`: valida CA primero, luego comprueba el fingerprint del cert del servidor
  - `PinningClientCertVerifier`: ídem para el cert del cliente en mTLS
  - Variable `TLS_PINNED_CERTS`: fingerprints separados por coma; ausente → pinning desactivado
- 32 tests TLS en total; todos pasando
- `docs/NETWORK_MEMBERSHIP.md`: nueva sección "Certificate Pinning TLS" con tabla de variables, comportamiento, comando openssl y guía de rotación

### Changed
- Reorganized documentation: `ANALYSIS/` → `docs/analysis/`, former `Documents/` → `docs/archive/`
- Stop tracking local `blockchain_blocks/` sample data

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

#### Client applications

Mobile or desktop clients are **not** maintained in this repository; API consumers may be implemented separately.

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
- Code quality gates (clippy, rustfmt)
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

**Last Updated:** April 3, 2026 (Storage — índices secundarios por rango)
**Maintainer:** rust-bc team  
**Repository:** https://github.com/your-org/rust-bc
