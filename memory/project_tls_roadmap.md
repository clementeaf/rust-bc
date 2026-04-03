---
name: Roadmap de fases completadas
description: Estado de todas las fases implementadas en rust-bc (TLS A-F, Consensus G-H)
type: project
---

Stack TLS completo (Fases A–F, 2026-04-02) y Consensus en curso (Fases G–H, 2026-04-03).

**Why:** TLS asegura HTTP y P2P antes de despliegue público. Consensus implementa fork resolution y el engine central.

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

**How to apply:** Próximas áreas libres: Storage (persistencia en disco) o Networking (gossip/peer discovery).
