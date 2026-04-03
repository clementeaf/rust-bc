---
name: Roadmap de fases completadas
description: Fases completadas — TLS A-F, Consensus G-H, Storage I-III (MemoryStore, RocksDB, backend switcheable)
type: project
---

TLS A–F, Consensus G–H, Storage I–III completos a 2026-04-03.

**Why:** Cada capa es prerequisito de la siguiente: TLS → Consensus → Storage → API de bloques.

---

## TLS — Fases A–F ✅

| Fase | Descripción | Módulos |
|------|-------------|---------|
| A | TLS básico HTTP + P2P | `src/tls.rs`, `src/network.rs` |
| B | mTLS (autenticación mutua de nodos) | `src/tls.rs` |
| C | Certificate pinning SHA-256 | `src/tls.rs` |
| D | Hot-reload de certificados (SIGHUP + intervalo) | `src/tls.rs` |
| E | PKI interna — CA propia, auto-provisioning | `src/pki.rs` |
| F | OCSP stapling | `src/tls.rs`, `src/pki.rs` |

Variables de entorno TLS: `TLS_CERT_PATH`, `TLS_KEY_PATH`, `TLS_VERIFY_PEER`, `TLS_CA_CERT_PATH`, `TLS_MUTUAL`, `TLS_PINNED_CERTS`, `TLS_RELOAD_INTERVAL`, `TLS_CA_KEY_PATH`, `TLS_OCSP_STAPLE_PATH`.

---

## Consensus — Fases G–H ✅

| Fase | Descripción | Módulos |
|------|-------------|---------|
| G | Fork resolution: `canonical_chain`, `resolve_fork`, `ForkChoice` | `src/consensus/dag.rs`, `src/consensus/fork_choice.rs` |
| H | `ConsensusEngine`: `accept_block`, `canonical_tip`, `canonical_chain` | `src/consensus/engine.rs` |

---

## Storage — Fase I ✅ (2026-04-03)

| Tarea | Descripción | Módulos |
|-------|-------------|---------|
| T1 | `MemoryStore` — `BlockStore` in-memory con `Mutex` | `src/storage/memory.rs` |
| T2 | `BlockStore` impl para `Arc<T>` — compartir store entre engine y API | `src/storage/traits.rs` |
| T3 | `ConsensusEngine::with_store()` — persiste bloques aceptados | `src/consensus/engine.rs` |
| T4 | `AppState.store` + endpoints `GET /store/blocks/{height}` y `/latest` | `src/app_state.rs`, `src/api/handlers/blocks.rs`, `src/api/routes.rs` |
| T5 | 7 tests de integración actix-web para ambos endpoints | `tests/store_blocks_api_test.rs` |

## Storage — Fase II ✅ (2026-04-03)

| Tarea | Descripción | Módulos |
|-------|-------------|---------|
| RocksDB | `RocksDbBlockStore` real con `rocksdb = "0.22"`, serde en todos los tipos, `WriteBatch` atómico, `META:latest_height` | `src/storage/adapters.rs`, `src/storage/traits.rs` |

## Storage — Fase III ✅ (2026-04-03)

| Tarea | Descripción | Módulos |
|-------|-------------|---------|
| III | `AppState.store` switcheable via `STORAGE_BACKEND=rocksdb\|memory` + `ROCKSDB_PATH` | `src/main.rs` |

**How to apply:** Próxima área: column families en RocksDB por tipo (bloques, txs, identidades).
