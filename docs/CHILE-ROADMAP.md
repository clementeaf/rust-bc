# Roadmap Chile — Producto

Plan para posicionar rust-bc como alternativa a Hyperledger Fabric en el mercado chileno.

Last updated: 2026-04-07

---

## Estado técnico

El core blockchain está completo: ~95% paridad funcional con Fabric 2.5, ~80% enterprise. 2040+ tests, CI verde, Docker deployment con 4 nodos, persistent Raft, X.509 MSP, RocksDB, SDK JS/TS, CLI nativo, block explorer.

Lo que sigue no es más código de blockchain. Es producto.

---

## 1. Documentación en español

**Por qué:** Los equipos técnicos en Chile trabajan en español. Una barrera de idioma frena adopción en bancos, retail, gobierno y startups fintech.

**Alcance:**

| Documento | Fuente (ya existe en inglés) | Entregable |
|-----------|------------------------------|------------|
| Guía de inicio rápido | `docs/QUICK-START.md` | `docs/es/INICIO-RAPIDO.md` |
| Referencia de API | `docs/API-REFERENCE.md` | `docs/es/REFERENCIA-API.md` |
| Guía de deployment | `docs/DEPLOYMENT.md` | `docs/es/DESPLIEGUE.md` |
| Comparación con Fabric | `docs/FABRIC-COMPARISON.md` | `docs/es/COMPARACION-FABRIC.md` |
| README principal | `README.md` | Sección bilingüe o `README.es.md` |

**Criterios de aceptación:**
- Documentación técnicamente precisa (no traducción literal, sino adaptada)
- Ejemplos con contexto chileno donde aplique (RUT, SII, etc.)
- Misma estructura que los docs en inglés para mantener paridad

**Esfuerzo:** 1-2 sesiones

---

## 2. Benchmarks publicados

**Por qué:** Para vender rust-bc vs Fabric necesitas números concretos. "Es más rápido" no basta — necesitas TPS, latencia p50/p99, throughput bajo carga, y comparación directa.

**Alcance:**

| Benchmark | Qué mide | Herramienta |
|-----------|----------|-------------|
| Ordering throughput | TXs/segundo en ordering service (Solo vs Raft) | Criterion (ya existe `benches/ordering_throughput.rs`) |
| Gateway latency | p50/p99 latencia end-to-end (submit → commit) | Script de carga con `wrk` o `hey` |
| Block propagation | Tiempo desde mine en node1 hasta visible en node3 | Script E2E con timestamps |
| Endorsement validation | Latencia por endorsement Ed25519 (1, 3, 5, 10 orgs) | Criterion (ya existe) |
| Concurrent clients | TPS con 10, 50, 100 clientes simultáneos | `hey` contra gateway/submit |
| Storage write throughput | Bloques/segundo en RocksDB | Criterion |
| Comparison vs Fabric | Mismos benchmarks corridos en Fabric 2.5 equivalente | Caliper o scripts propios |

**Entregable:** `docs/BENCHMARKS.md` con números, gráficos, metodología reproducible, y `scripts/benchmark.sh` para que cualquiera los ejecute.

**Criterios de aceptación:**
- Números reproducibles en hardware estándar (documentar specs)
- Comparación honesta con Fabric (no cherry-picking)
- Incluir limitaciones y caveats

**Esfuerzo:** 2-3 sesiones

---

## 3. Flujo de onboarding multi-organización

**Por qué:** En un consorcio (bancos, retail, gobierno), una organización nueva debe poder unirse a la red en minutos, no días. Sin esto, rust-bc no escala como producto.

**Alcance:**

| Paso | Qué hace | Automatización |
|------|----------|----------------|
| 1. Generar identidad | Crear cert + key para la nueva org | `bcctl org init <org_id>` |
| 2. Registrar en la red | POST /store/organizations + políticas | `bcctl org register <org_id> --network <url>` |
| 3. Configurar nodo | Generar docker-compose.yml para el nuevo peer | `bcctl node init --org <org_id> --peer-port 8090` |
| 4. Join a canales | Unirse a canales existentes | `bcctl channel join <channel_id> --org <org_id>` |
| 5. Instalar chaincode | Desplegar chaincode aprobado | `bcctl chaincode install --org <org_id>` |
| 6. Verificar | Health check + sync status | `bcctl org verify <org_id>` |

**Entregable:**
- `scripts/onboard-org.sh` — script all-in-one que ejecuta los 6 pasos
- `docs/es/ONBOARDING.md` — guía paso a paso en español
- Nuevos comandos en `bcctl` (Rust CLI)

**Criterios de aceptación:**
- Una organización nueva se une a la red en < 10 minutos
- No requiere editar archivos de configuración manualmente
- Funciona con la red Docker existente

**Esfuerzo:** 2-3 sesiones

---

## 4. SDK Python

**Por qué:** El ecosistema fintech y gobierno en Chile usa Python extensivamente. Django, FastAPI, scripts de automatización, data pipelines — todos necesitan un SDK nativo.

**Alcance:**

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| `connect(url, tls_config?)` | GET /health | Verificar conexión + TLS |
| `submit_transaction(chaincode_id, channel_id, tx)` | POST /gateway/submit | Pipeline completo |
| `evaluate(chaincode_id, function)` | POST /chaincode/{id}/simulate | Query read-only |
| `register_org(org)` | POST /store/organizations | Registrar organización |
| `set_policy(resource_id, policy)` | POST /store/policies | Configurar endorsement |
| `create_channel(channel_id)` | POST /channels | Crear canal |
| `put_private_data(collection, key, value, org_id)` | PUT /private-data/{c}/{k} | Datos privados |
| `get_private_data(collection, key, org_id)` | GET /private-data/{c}/{k} | Leer datos privados |
| `subscribe_blocks(channel_id)` | WS /events/blocks | Stream de eventos |
| `health()` | GET /health | Health check con dependencias |

**Estructura:**

```
sdk-python/
├── rust_bc/
│   ├── __init__.py
│   ├── client.py        # BlockchainClient class
│   ├── types.py         # Pydantic models
│   └── exceptions.py    # Custom exceptions
├── tests/
│   ├── test_client.py
│   └── test_types.py
├── examples/
│   ├── basic_usage.py
│   └── submit_transaction.py
├── pyproject.toml
└── README.md
```

**Dependencias:** `httpx` (async HTTP), `pydantic` (tipos), `websockets` (eventos)

**Criterios de aceptación:**
- Paridad funcional con SDK JS/TS
- Type hints completos (mypy compatible)
- 80%+ test coverage con pytest
- Publicable en PyPI como `rust-bc`
- Ejemplos funcionales

**Esfuerzo:** 2 sesiones

---

## 5. Audit trail para reguladores

**Por qué:** SII, CMF, y otros reguladores chilenos requieren trazabilidad inmutable. Cada operación debe tener un registro auditable: quién, qué, cuándo, desde dónde.

**Alcance:**

| Componente | Qué registra | Dónde se almacena |
|-----------|-------------|-------------------|
| Request audit log | Cada request HTTP: timestamp, method, path, org_id, IP, status, trace_id | RocksDB CF `audit_log` |
| Transaction audit | Cada TX: submit → endorse → order → commit con timestamps | Campo adicional en Block |
| Identity audit | Creación, rotación, revocación de identidades | RocksDB CF `identity_audit` |
| Config change audit | Cambios en channel config, policies, ACLs | RocksDB CF `config_audit` |
| Export API | GET /audit/transactions, GET /audit/requests | Nuevos endpoints |

**Middleware de auditoría:**

```rust
pub struct AuditMiddleware;
// Captura: timestamp, method, path, org_id (de TlsIdentity),
// source_ip, response_status, trace_id, duration_ms
// Persiste en CF audit_log con key = {timestamp:020}:{trace_id}
```

**Endpoints de consulta:**

| Endpoint | Descripción |
|----------|-------------|
| `GET /api/v1/audit/requests?from=&to=&org_id=` | Listar requests auditados |
| `GET /api/v1/audit/transactions?from=&to=` | Listar transacciones con timeline |
| `GET /api/v1/audit/export?format=csv&from=&to=` | Exportar para reguladores |

**Criterios de aceptación:**
- Cada request HTTP queda registrado con identidad del caller
- Log inmutable (append-only en RocksDB)
- Exportable en CSV para entrega a reguladores
- No impacta latencia > 1ms por request
- Retención configurable via env var `AUDIT_RETENTION_DAYS`

**Esfuerzo:** 2-3 sesiones

---

## Orden de ejecución sugerido

```
1. Documentación en español     ← desbloquea adopción inmediata
2. SDK Python                   ← desbloquea ecosistema fintech/gobierno
3. Onboarding multi-org         ← desbloquea consorcios
4. Audit trail                  ← desbloquea reguladores (SII, CMF)
5. Benchmarks                   ← desbloquea ventas vs Fabric
```

---

## Criterio de éxito

rust-bc es viable como alternativa a Fabric en Chile cuando:

- [ ] Un desarrollador chileno puede ir de cero a transacción en < 10 minutos leyendo docs en español
- [ ] Una organización nueva se une a un consorcio existente en < 10 minutos con un solo comando
- [ ] Un regulador puede recibir un CSV de auditoría de todas las operaciones de un período
- [ ] Benchmarks publicados demuestran rendimiento competitivo vs Fabric 2.5
- [ ] SDKs en JS/TS y Python cubren todos los endpoints con tipos y tests
