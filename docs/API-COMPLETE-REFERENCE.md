# Cerulean Ledger — API Reference

Base URL: `http://<host>:8080/api/v1`

Todas las respuestas siguen el envelope:
```json
{
  "status": "Success",
  "status_code": 200,
  "message": "OK",
  "data": { ... },
  "timestamp": "2026-05-12T...",
  "trace_id": "uuid"
}
```

Headers requeridos en modo strict: `X-Org-Id`, `X-Msp-Role`. En modo permissive (`ACL_MODE=permissive`) no se requieren.

---

## Salud y Estado

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/health` | Estado del nodo: uptime, altura, validadores, storage, peers, ordering |
| GET | `/version` | Versión de la API y del nodo |
| GET | `/stats` | Estadísticas: bloques, transacciones, mempool, peers |
| GET | `/metrics` | Métricas Prometheus (fuera de /api/v1) |

---

## Identidad Digital

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/store/identities` | Registrar identidad (DID) |
| GET | `/store/identities?limit=100&offset=0` | Listar identidades (paginado) |
| GET | `/store/identities/{did}` | Consultar identidad por DID |
| POST | `/identity/create` | Crear identidad con keypair Ed25519 |
| GET | `/identity/{did}` | Leer identidad |
| POST | `/identity/{did}/rotate-key` | Rotar clave del DID |
| POST | `/identity/verify-signature` | Verificar firma Ed25519 |

**Crear identidad:**
```json
POST /store/identities
{
  "did": "did:cerulean:universidad-chile",
  "created_at": 1715000000,
  "updated_at": 1715000000,
  "status": "active"
}
```

---

## Documentos y Credenciales

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/store/credentials` | Emitir documento/credencial |
| GET | `/store/credentials?limit=100&offset=0` | Listar documentos (paginado) |
| GET | `/store/credentials/{id}` | Consultar documento por ID |
| GET | `/store/credentials/by-subject/{did}` | Documentos de un titular |
| GET | `/store/credentials/by-issuer/{did}` | Documentos de un emisor |
| POST | `/credentials/issue` | Emitir credencial (genera ID automático) |
| GET | `/credentials/{id}` | Leer credencial |
| POST | `/credentials/{id}/verify` | Verificar estado y vigencia |
| POST | `/credentials/{id}/revoke` | Revocar credencial |

**Emitir documento:**
```json
POST /store/credentials
{
  "id": "doc-titulo-001",
  "issuer_did": "did:cerulean:universidad-chile",
  "subject_did": "did:cerulean:alice-persona",
  "cred_type": "Titulo Profesional",
  "issued_at": 1715000000,
  "expires_at": 0,
  "claims": {
    "grado": "Ingenieria Civil Informatica",
    "mencion": "Cum Laude"
  },
  "status": "active"
}
```

> El `issuer_did` debe estar registrado previamente. El sistema valida su existencia.

---

## Gobernanza Digital

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/governance/params` | Parámetros del protocolo |
| POST | `/governance/proposals` | Enviar propuesta |
| GET | `/governance/proposals` | Listar propuestas (filtro ?status=Voting) |
| GET | `/governance/proposals/{id}` | Detalle de propuesta |
| POST | `/governance/proposals/{id}/vote` | Votar |
| GET | `/governance/proposals/{id}/tally` | Resultados (público) |
| GET | `/governance/proposals/{id}/votes` | Votos individuales (admin) |
| POST | `/governance/proposals/{id}/execute` | Ejecutar propuesta aprobada |
| POST | `/governance/proposals/{id}/close` | Cerrar votación |
| POST | `/governance/proposals/{id}/veto` | Veto de emergencia |
| POST | `/governance/delegate` | Delegar poder de voto |

**Enviar propuesta:**
```json
POST /governance/proposals
{
  "proposer": "alice",
  "description": "Reducir block time a 2 segundos",
  "deposit": 10000,
  "action": {
    "type": "text",
    "title": "Reducir block time",
    "description": "Mejora UX"
  }
}
```

**Votar:**
```json
POST /governance/proposals/1/vote
{
  "voter": "alice",
  "option": "Yes"
}
```

Opciones: `Yes`, `No`, `Abstain`. Validaciones: voter no vacío (max 256 bytes), no puede votar dos veces, quorum/threshold semántico (1-100).

---

## Audit Trail y Compliance

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/audit/requests?limit=10&action=block_mined&org_id=org1` | Consultar audit log |
| GET | `/audit/export` | Exportar en CSV |
| GET | `/regulatory/checks` | 21 checks regulatorios |
| GET | `/regulatory/report` | Reporte firmado con SHA-256 |

**Acciones auditadas:** `block_mined`, `wallet_created`, `did_registered`, `did_revoked`, `credential_stored`, `credential_revoked`, `proposal_submitted`, `proposal_voted`, `chaincode_installed`, `channel_created`, entre otras.

---

## Seguridad y Forense

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/pentest/report` | 40 escenarios de penetration testing |
| GET | `/stress/report?ops=500` | Stress test de 10 módulos |
| GET | `/forensic/timeline` | Timeline completa de eventos |
| GET | `/forensic/security` | Eventos de seguridad + severidad |
| GET | `/forensic/integrity?from=0&to=100` | Verificar integridad hash chain |
| GET | `/forensic/replay?from=0&to=100` | Replay de bloques |
| POST | `/forensic/export` | Paquete de evidencia firmado |

---

## Oráculos

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/oracle/status` | Health del subsistema oracle |
| GET | `/oracle/feeds` | Precios con metadata de frescura |
| GET | `/oracle/feeds/{symbol}` | Precio específico |
| GET | `/oracle/nodes` | Nodos oracle registrados |
| POST | `/oracle/legal/query` | Consultar oráculo legal |
| GET | `/oracle/legal/records` | Records del oráculo legal |

---

## Inteligencia AML

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/intelligence/anomaly` | Detección de anomalías (z-score) |
| POST | `/intelligence/risk` | Scoring de riesgo AML (6 reglas) |
| POST | `/intelligence/patterns` | Patrones sospechosos (velocity, structuring) |

---

## Blockchain Core

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/blocks` | Listar bloques |
| GET | `/blocks/index/{n}` | Bloque por índice |
| GET | `/blocks/{hash}` | Bloque por hash |
| POST | `/mine` | Minar bloque (503 si mining en curso) |
| POST | `/transactions` | Enviar transacción |
| GET | `/mempool` | Transacciones pendientes |
| POST | `/wallets/create` | Crear wallet |
| GET | `/wallets/{address}` | Consultar wallet |

---

## Compliance ISO 20022

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/compliance/validate/pacs008` | Credit Transfer |
| POST | `/compliance/validate/pacs002` | Payment Status |
| POST | `/compliance/validate/pacs004` | Payment Return |
| POST | `/compliance/validate/pain001` | Credit Transfer Initiation |
| POST | `/compliance/validate/pain002` | Payment Status Report |
| POST | `/compliance/validate/camt053` | Bank Statement |
| POST | `/compliance/validate/camt052` | Intraday Statement |
| GET | `/compliance/countries` | 193 países ISO 3166 |
| GET | `/compliance/currencies` | 64 monedas ISO 4217 |

---

## EVM Compatibility

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/evm/deploy` | Desplegar contrato EVM |
| POST | `/evm/call` | Ejecutar función |
| POST | `/evm/static-call` | Lectura sin estado |
| GET | `/evm/contracts` | Listar contratos |

---

## ZKP (Verificación de atributos)

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/identity/zkp/prove` | Generar prueba de atributo |
| POST | `/identity/zkp/verify` | Verificar prueba |

---

## PIN

| Método | Path | Descripción |
|--------|------|-------------|
| POST | `/pin/generate` | Generar PIN (Argon2id) |
| POST | `/pin/verify` | Verificar PIN |

---

## Rate Limiting

Default: 20 req/s, 100 req/min, 3000 req/hora por IP. Configurable via:
- `RATE_LIMIT_RPS` — requests por segundo
- `RATE_LIMIT_RPM` — requests por minuto
- `RATE_LIMIT_RPH` — requests por hora

Respuesta cuando excedido: `429 Too Many Requests`.

---

## Variables de Entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `API_PORT` | 8080 | Puerto HTTP |
| `ACL_MODE` | strict | `permissive` para sandbox |
| `STORAGE_BACKEND` | memory | `rocksdb` para persistencia |
| `STORAGE_PATH` | ./data/rocksdb | Directorio RocksDB |
| `SIGNING_ALGORITHM` | ed25519 | `ml-dsa-65` para PQC |
| `HASH_ALGORITHM` | sha256 | `sha3-256` alternativo |
| `RATE_LIMIT_RPS` | 20 | Requests por segundo |
| `RATE_LIMIT_RPM` | 100 | Requests por minuto |
| `RATE_LIMIT_RPH` | 3000 | Requests por hora |
