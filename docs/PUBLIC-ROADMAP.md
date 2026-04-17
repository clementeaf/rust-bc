# Roadmap Público — rust-bc

Hoja de ruta orientada a partners, evaluadores y la comunidad blockchain chilena.

**Última actualización:** 2026-04-15

---

## Estado actual: Core completo

La plataforma blockchain está funcional con ~95% de paridad respecto a Hyperledger Fabric 2.5:

- Ciclo completo: endorse → order → commit
- Raft persistente con crash recovery
- Channels, private data, chaincode Wasm
- Identidad DID + credenciales verificables
- Firmas post-cuánticas ML-DSA-65 (FIPS 204)
- 2,040+ tests, 71 E2E, CI verde
- Docker deployment con monitoreo (Prometheus + Grafana)

---

## Q2 2026 — Producto mínimo viable

Transición de plataforma técnica a producto entregable.

| Entregable | Descripción | Estado |
|---|---|---|
| Graceful shutdown | SIGTERM/SIGINT con flush de RocksDB y drain de conexiones | Completado |
| Persistent service stores | 9 servicios persistentes en RocksDB (no solo bloques) | Completado |
| Health check avanzado | Verificación de RocksDB, peers, ordering; status degradado | Completado |
| SDK TypeScript | Operaciones Fabric-style: submit, evaluate, private data, events | Completado |
| CLI operador (`bcctl`) | 14 comandos, output JSON/tabla, exit codes | Completado |
| Documentación completa | Quick-start, API reference (68 endpoints), deployment guide | Completado |
| Benchmarks publicados | Criterion + scripts live, comparación con Fabric | Completado |

## Q3 2026 — Ecosistema Chile

Foco en adopción en el mercado chileno y ecosistema de desarrolladores.

| Entregable | Descripción | Estado |
|---|---|---|
| Documentación en español | Quick-start, API, deployment, comparación Fabric | Pendiente |
| SDK Python | Paridad con SDK TS, type hints, publicación PyPI | Pendiente |
| Onboarding multi-org | Script automatizado: identidad → registro → nodo → channel join en < 10 min | Pendiente |
| Audit trail para reguladores | Middleware de auditoría, export CSV, endpoints de consulta | Pendiente |

## Q4 2026 — Enterprise hardening

Preparación para pilotos en producción y evaluaciones de seguridad.

| Entregable | Descripción | Estado |
|---|---|---|
| FIPS 140-3 preparación | Self-tests KAT, zeroización de claves, documentación de módulo criptográfico | En progreso |
| Property-based testing | Proptest para firma/verificación, fuzzing para parsers | Pendiente |
| Encryption at rest | Documentación + configuración de cifrado en disco (RocksDB/LUKS) | Pendiente |
| Vulnerability disclosure | Proceso documentado de reporte, tiempos de respuesta | Pendiente |

## 2027 — Certificaciones y expansión

| Entregable | Descripción |
|---|---|
| SOC 2 Type II | Período de observación + auditoría (6-12 meses) |
| ISO 27001 | Framework de políticas + certificación |
| Fabric CA integration | Servicio de enrolamiento y registro de identidades |
| gRPC protocol | Interoperabilidad con redes Fabric existentes (solo si hay demanda) |
| ML-KEM (FIPS 203) | Key exchange post-cuántico para TLS nodo-a-nodo |
| Hybrid signatures | Dual Ed25519 + ML-DSA-65 para máxima compatibilidad |

---

## Métricas de éxito

| Hito | Criterio |
|---|---|
| MVP entregable | Red de 4 nodos sobrevive restart sin pérdida de datos |
| Adopción Chile | Desarrollador va de cero a transacción en < 10 min con docs en español |
| Enterprise-ready | Piloto con al menos 1 organización en producción |
| Certificable | Level 2 del certification roadmap completado |

---

## Cómo participar

- **Evaluadores técnicos:** Clone el repo, siga el [Quick Start](QUICK-START.md), ejecute `./scripts/e2e-test.sh`
- **Partners potenciales:** Revise la [Comparación con Fabric](FABRIC-COMPARISON.md) y los [Benchmarks](BENCHMARKS-RESULTS.md)
- **Casos de uso:** Vea el [análisis de impacto para Hoktus](HOKTUS-BLOCKCHAIN-IMPACT.md) como ejemplo de integración SaaS

---

*rust-bc es un proyecto activo en desarrollo. Este roadmap refleja la dirección actual y puede ajustarse según feedback de la comunidad y partners.*
