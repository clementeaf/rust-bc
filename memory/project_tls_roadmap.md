---
name: Roadmap de fases completadas
description: Fases completadas — TLS A-F, Consensus G-H, Storage I-V
type: project
---

TLS A–F, Consensus G–H, Storage I–V completos a 2026-04-03.

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

## Storage ✅

| Fase | Descripción | Módulos |
|------|-------------|---------|
| I | `MemoryStore`, `Arc<T>` impl, endpoints `GET /store/blocks/{height}` y `/latest` | `src/storage/memory.rs`, `src/storage/traits.rs`, `src/api/handlers/blocks.rs` |
| II | `RocksDbBlockStore` con JSON + `WriteBatch` atómico | `src/storage/adapters.rs` |
| III | Backend switcheable vía `STORAGE_BACKEND=rocksdb\|memory` | `src/main.rs` |
| IV | Column Families: `blocks`, `transactions`, `identities`, `credentials`, `meta` | `src/storage/adapters.rs` |
| V | REST endpoints store: `POST/GET /store/transactions`, `/store/identities`, `/store/credentials` | `src/api/handlers/{transactions,identity,credentials}.rs`, `src/api/routes.rs` |

**How to apply:** Próxima área: índices secundarios por rango (e.g. tx por block_height) o iteradores CF.
