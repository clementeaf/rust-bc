# Roadmap de Integración — Ecosistema Cerulean DLT

Derivado de la Guía Maestra de Integración v1.0 (Abraxas), adaptado a la arquitectura real del codebase.

Fecha: 2026-05-11

---

## Fase 1: ISO Compliance Hooks

**Qué:** Audit trail ISO 27001 integrado en el ciclo de vida del bloque.

**Por qué primero:** Es la base observable. Sin trazabilidad de eventos, las fases siguientes no pueden demostrar cumplimiento.

**Dónde se ancla:**
- `src/consensus/` — emit de eventos al confirmar bloque
- `src/chaincode/` — emit al instalar/actualizar chaincode
- `src/identity/` — emit al registrar/revocar DID
- `src/api/` — emit en mutaciones (mine, transfer, stake)

**Entregables:**
1. `src/audit/mod.rs` — `AuditEvent` struct (timestamp, actor, action, resource, outcome, metadata)
2. `src/audit/logger.rs` — trait `AuditLogger` con impl en memoria + impl append-only a archivo (JSONL)
3. Hooks en los 4 puntos de anclaje: consensus commit, chaincode lifecycle, identity mutations, API mutations
4. Campo `audit_log: Arc<dyn AuditLogger>` en `AppState`
5. Endpoint `GET /api/v1/audit/events` con filtros (actor, action, date range)
6. Tests: >=20 unit + integration

**No incluye:** Certificación ISO formal (eso es proceso organizacional, no código).

---

## Fase 2: Sandbox / Gatekeeper para Chaincode

**Qué:** Validación obligatoria de chaincode antes de inyectarlo al estado global.

**Por qué segundo:** Depende de Fase 1 para registrar resultados de validación en el audit trail.

**Dónde se ancla:**
- `src/chaincode/upgrade.rs` — `UpgradeManager` ya tiene propose/approve/commit
- `src/chaincode/` — package store existente (Wasm bytes + SHA-256)

**Entregables:**
1. `src/chaincode/sandbox.rs` — `SandboxValidator` trait
2. Validaciones mínimas: Wasm well-formedness, memory limits, import whitelist (no syscalls prohibidos), execution timeout
3. `SandboxReport` struct (passed: bool, checks: Vec<CheckResult>, gas_estimate, duration)
4. Gate en `UpgradeManager::commit()` — rechazar si sandbox no pasó
5. `AuditEvent` emitido con resultado del sandbox
6. Endpoint `GET /api/v1/chaincode/{id}/sandbox-report`
7. Tests: validación de Wasm malformado, imports prohibidos, timeout, happy path

**No incluye:** Ejecución especulativa completa del contrato contra estado simulado (futuro).

---

## Fase 3: Dashboard Forense

**Qué:** Vista de compliance en tiempo real en el block explorer.

**Por qué tercero:** Consume los eventos de Fase 1 y reportes de Fase 2.

**Dónde se ancla:**
- `block-explorer-vite/` — React + Vite existente
- Endpoints de audit y sandbox de fases anteriores

**Entregables:**
1. Página "Compliance" en el explorer: tabla de audit events con filtros
2. Página "Chaincode Health": lista de contratos con estado de sandbox (pass/fail/pending)
3. Timeline visual: eventos de compliance por bloque
4. Indicadores: total events, failed validations, DID mutations, chaincode deployments
5. Auto-refresh vía polling (WebSocket futuro)

**No incluye:** Alertas push, integración con SIEM externo.

---

## Fase 4: Oráculo Legal

**Qué:** Servicio off-chain que consulta APIs legales externas y publica resultados on-chain como transacciones firmadas.

**Por qué cuarto:** Independiente técnicamente, pero se beneficia de la infraestructura de audit (Fase 1) y sandbox (Fase 2) para validar los datos que inyecta.

**Dónde se ancla:**
- `src/api/` — nuevo endpoint para registrar/consultar oráculos
- `src/storage/traits.rs` — nuevo tipo `OracleRecord`

**Entregables:**
1. `src/oracle/mod.rs` — `OracleService` trait (query, publish, verify)
2. `src/oracle/legal.rs` — impl para consulta de APIs legales (BCN u otras fuentes configurables)
3. `OracleRecord` en storage: source, query, response_hash, timestamp, signature
4. Verificación: respuesta firmada por el oráculo, hash almacenado on-chain, datos completos off-chain
5. Endpoint `POST /api/v1/oracle/query` + `GET /api/v1/oracle/records`
6. Rate limiting y cache de consultas (evitar spam a APIs externas)
7. `AuditEvent` por cada consulta y publicación
8. Tests: mock de API externa, verificación de firma, cache hit/miss

**No incluye:** Automatización de cumplimiento legal (el oráculo informa, no decide).

---

## Fase 5: ZKP para Identidad Soberana

**Qué:** Pruebas de conocimiento cero sobre atributos de identidad (edad >= 18, nacionalidad, credencial válida) sin revelar datos subyacentes.

**Por qué último:** Mayor complejidad criptográfica. Requiere identidad madura (DID + VC ya existen) y audit trail (Fase 1) para trazabilidad de verificaciones.

**Dónde se ancla:**
- `src/identity/` — DID + key management existente
- `src/storage/traits.rs` — `Credential` ya tiene claims como `HashMap<String, String>`

**Entregables:**
1. Investigación: selección de esquema ZKP (Groth16 vs Bulletproofs vs PLONK) basada en: tamaño de prueba, tiempo de verificación, compatibilidad con Wasm
2. `src/identity/zkp.rs` — trait `ZkpProver` / `ZkpVerifier`
3. Predicados soportados iniciales: range proof (edad), set membership (nacionalidad in [lista]), credential validity (sin revelar issuer)
4. `ZkPresentation` struct: prueba + public inputs + credential reference
5. Endpoint `POST /api/v1/identity/zkp/prove` + `POST /api/v1/identity/zkp/verify`
6. Integración con `SigningProvider` existente (Ed25519 / ML-DSA-65 para firmar la presentación)
7. `AuditEvent` por cada verificación ZKP (sin datos revelados, solo resultado)
8. Tests: prueba válida, prueba inválida, predicados fuera de rango, credential revocada

**No incluye:** Circuitos custom, integración con wallets móviles (eso sería Fase 5b).

---

## Dependencias entre fases

```
Fase 1 (ISO Audit) ─────┬──> Fase 2 (Sandbox)
                         │        │
                         │        v
                         ├──> Fase 3 (Dashboard)
                         │
                         ├──> Fase 4 (Oráculo)
                         │
                         └──> Fase 5 (ZKP)
```

Fase 1 es prerequisito de todas. Fases 2-5 pueden paralelizarse después, con la excepción de que Fase 3 consume datos de Fase 2.

## Estimación de complejidad

| Fase | Complejidad | Módulos nuevos | Tests estimados |
|------|-------------|----------------|-----------------|
| 1. ISO Audit | Media | 2 (`audit/`) | 20-30 |
| 2. Sandbox | Media | 1 (`chaincode/sandbox.rs`) | 15-20 |
| 3. Dashboard | Media (frontend) | 3 páginas React | 10 (E2E) |
| 4. Oráculo | Media-Alta | 2 (`oracle/`) | 20-25 |
| 5. ZKP | Alta | 2+ (`identity/zkp.rs`) | 25-35 |

## Notas

- Cada fase sigue el flujo: tests primero (RED) -> implementación (GREEN) -> refactor (IMPROVE)
- Cada fase produce un commit atómico con changelog entry
- La nomenclatura es ML-DSA-65 (FIPS 204), no "Dilithium"
- No se usa Substrate, pallets, FRAME, ni Polkadot.js — todo es nativo sobre la arquitectura existente
