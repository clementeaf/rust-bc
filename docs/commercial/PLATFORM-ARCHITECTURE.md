# Cerulean Ledger — Arquitectura de Plataforma

Visión ejecutiva de las capas horizontales, lo que cada una contiene, y las líneas de acción que habilitan.

**Fecha:** 2026-05-08

---

## Vista por capas

```
┌─────────────────────────────────────────────────────────────┐
│                    APLICACIONES (verticales)                │
│  Credenciales · Voto electrónico · Supply chain · Finanzas  │
├─────────────────────────────────────────────────────────────┤
│                    INTERFACES DE ACCESO                     │
│  API REST · WebSocket · SDKs (JS, Python) · Light Client   │
├─────────────────────────────────────────────────────────────┤
│                    LÓGICA DE NEGOCIO                        │
│  Smart Contracts (Wasm + EVM) · Gobernanza · Tokenomics    │
├─────────────────────────────────────────────────────────────┤
│                    PRIVACIDAD Y ACCESO                      │
│  Canales · Private Data · ACL · MSP · Identidad (DID)      │
├─────────────────────────────────────────────────────────────┤
│                    CONSENSO Y ORDENAMIENTO                  │
│  Raft · BFT (HotStuff) · DAG · DPoS · Ejecución paralela  │
├─────────────────────────────────────────────────────────────┤
│                    SEGURIDAD CRIPTOGRÁFICA                  │
│  ML-DSA-65 · SHA3-256 · ML-KEM-768 · Dual-signing · mTLS  │
├─────────────────────────────────────────────────────────────┤
│                    ALMACENAMIENTO                           │
│  RocksDB · World State · Block Store · Índices secundarios  │
├─────────────────────────────────────────────────────────────┤
│                    RED Y COMUNICACIÓN                       │
│  P2P Gossip · Peer Discovery · Bridge cross-chain · CSIRT  │
└─────────────────────────────────────────────────────────────┘
```

---

## Detalle por capa

### 1. Seguridad criptográfica (base)

Lo que protege toda la plataforma.

| Componente | Qué hace | Estado |
|---|---|---|
| ML-DSA-65 (FIPS 204) | Firmas digitales post-cuánticas | Producción |
| SHA3-256 (FIPS 202) | Hash de bloques y transacciones | Producción |
| ML-KEM-768 (FIPS 203) | Intercambio de claves post-cuántico | Producción |
| Ed25519 | Firmas clásicas (compatibilidad) | Producción |
| Dual-signing | Firma simultánea clásica + PQC para migración | Producción |
| mTLS | Autenticación mutua entre nodos y clientes | Producción |
| Módulo FIPS 140-3 | Aislamiento criptográfico con FSM, KAT self-tests, zeroización | Pre-lab completo |

**Línea de acción:** Cumplimiento normativo (Ley 21.663), protección a largo plazo, diferenciación competitiva.

---

### 2. Almacenamiento

Donde viven los datos.

| Componente | Qué hace | Estado |
|---|---|---|
| RocksDB | Persistencia duradero con column families | Producción |
| MemoryStore | Backend en memoria para tests y demos | Producción |
| Block Store | Bloques encadenados con hash chain verificable | Producción |
| World State | Estado actual de contratos y datos | Producción |
| Índices secundarios | Búsqueda de transacciones por bloque (prefix scan) | Producción |
| Retention Policy | TTL configurable por canal (bloques, private data, transacciones) | Producción |

**Línea de acción:** Escalabilidad, auditoría histórica, cumplimiento de retención de datos.

---

### 3. Consenso y ordenamiento

Cómo se acuerda la verdad.

| Componente | Qué hace | Estado |
|---|---|---|
| Raft | Tolerancia a fallas por caída (crash-fault) | Producción |
| BFT (HotStuff) | Tolerancia a nodos maliciosos (byzantine-fault) | Producción |
| DAG | Múltiples propuestas simultáneas, alta concurrencia | Producción |
| DPoS | Selección de validadores por stake | Producción |
| Ejecución paralela | Wave scheduler con detección de conflictos RAW/WAW/WAR | Producción |
| MVCC | Validación de lecturas/escrituras por wave al commit | Producción |
| Equivocation detector | Detección de propuestas conflictivas del mismo validador | Producción |
| Slashing | Penalización económica a validadores maliciosos | Producción |

**Línea de acción:** Rendimiento (~18,700 TX/s motor, ~42 TPS E2E), seguridad bizantina, flexibilidad operativa.

---

### 4. Privacidad y acceso

Quién ve qué.

| Componente | Qué hace | Estado |
|---|---|---|
| Canales | Ledger aislado por organización o consorcio | Producción |
| Private data collections | Datos compartidos solo entre partes autorizadas, con TTL | Producción |
| ACL (deny-by-default) | Control de acceso por recurso, denegación por defecto | Producción |
| MSP (roles) | Admin, Peer, Client — extraídos de certificado X.509 | Producción |
| DID (identidad) | Identificadores descentralizados `did:cerulean:` | Producción |
| Credenciales verificables | Emisión y verificación criptográfica de certificados | Producción |
| PIN | Generación CSPRNG + verificación Argon2id para autenticación simple | Producción |

**Línea de acción:** Compliance de privacidad, aislamiento multi-tenant, identidad soberana.

---

### 5. Lógica de negocio

Reglas que se ejecutan automáticamente.

| Componente | Qué hace | Estado |
|---|---|---|
| Chaincode (Wasm) | Contratos en Wasmtime con fuel metering y memory limits | Producción |
| EVM (revm) | Contratos Solidity compatibles con ecosistema Ethereum | Producción |
| Chaincode lifecycle | Install → Approve → Commit (multi-org) | Producción |
| Chaincode upgrade | Propuesta de actualización con aprobación multi-organización | Producción |
| Gobernanza | Propuestas, votación stake-weighted, timelock, ejecución | Producción |
| Tokenomics (NOTA) | Supply cap 100M, halving, EIP-1559 base fee, storage deposits | Producción |
| Bridge cross-chain | Escrow, Merkle proofs, relay con retry | Producción |

**Línea de acción:** Automatización de procesos, interoperabilidad, economía programable.

---

### 6. Interfaces de acceso

Cómo se consume la plataforma.

| Componente | Qué hace | Estado |
|---|---|---|
| API REST (Actix-Web) | 60+ endpoints, envelope `ApiResponse`, rate limiting | Producción |
| WebSocket | Eventos en tiempo real (bloques, transacciones, chaincode) | Producción |
| CSIRT webhook | Forwarding de eventos de seguridad a SIEM/CSIRT externo | Producción |
| SDK JavaScript | Cliente TypeScript con tipos, tests, ejemplos | v1.0 |
| SDK Python | Cliente Python con tipos, excepciones, tests | v1.0 |
| Light client | Verificación de estado via Merkle proofs sin full node | Producción |
| Block Explorer | UI React + Vite (Cerulean Ledger) | Producción |
| Voto electrónico | UI React + Vite (Cerulean Voto) | Producción |
| CLI operador | `bcctl` con 14 comandos | Producción |

**Línea de acción:** Integración con sistemas existentes, demos, adopción por desarrolladores.

---

### 7. Red y comunicación

Cómo se conectan los nodos.

| Componente | Qué hace | Estado |
|---|---|---|
| P2P Gossip | Heartbeat alive + pull-sync + anchor peers | Producción |
| Peer discovery | Registro y descubrimiento de nodos | Producción |
| State sync | Sincronización pull-based para nodos nuevos | Producción |
| Block replication | Broadcast de bloques ordenados a todos los peers | Producción |
| Bridge relayer | Job queue con batch processing y retry para cross-chain | Producción |
| Certificate pinning | SHA-256 fingerprint allowlist por canal | Producción |

**Línea de acción:** Redes multi-nodo, resiliencia, interoperabilidad cross-chain.

---

### 8. Operación y observabilidad

Cómo se administra.

| Componente | Qué hace | Estado |
|---|---|---|
| Docker Compose | Despliegue de red completa (6 nodos + monitoring) | Producción |
| Sandbox | Single-node demo con Cloudflare Tunnel | Producción |
| Prometheus + Grafana | Métricas y dashboards | Producción |
| Health endpoint | `/api/v1/health` con status de todos los subsistemas | Producción |
| Graceful shutdown | SIGTERM/SIGINT con drain de conexiones | Producción |
| Genesis config | Presets testnet/devnet/mainnet con validación | Producción |
| Faucet | Token drip rate-limited para testnets | Producción |
| Stress test | Ramp-up automatizado con reporte de TPS y latencia | Producción |

**Línea de acción:** Despliegue rápido, monitoreo, testing automatizado.

---

## Líneas de acción consolidadas

| Línea de acción | Capas involucradas | Verticales que habilita |
|---|---|---|
| **Compliance regulatorio** | Criptografía + Privacidad + Observabilidad | Todas |
| **Protección a largo plazo** | Criptografía (PQC + dual-signing) | Credenciales, Finanzas |
| **Automatización de procesos** | Lógica de negocio (contratos + gobernanza) | Supply chain, Finanzas, Voto |
| **Privacidad multi-organización** | Privacidad (canales + ACL + private data) | Todas |
| **Rendimiento y escalabilidad** | Consenso (paralelo + MVCC) + Almacenamiento | Supply chain, Finanzas |
| **Integración con sistemas existentes** | Interfaces (API + SDKs + EVM) | Todas |
| **Verificabilidad independiente** | Interfaces (light client) + Criptografía | Credenciales, Voto |
| **Soberanía operacional** | Red + Operación (self-hosted, sin cloud lock-in) | Todas |

---

## Conteo de componentes

| Capa | Componentes | Estado producción |
|---|---|---|
| Seguridad criptográfica | 7 | 6 producción + 1 pre-lab |
| Almacenamiento | 6 | 6 producción |
| Consenso y ordenamiento | 8 | 8 producción |
| Privacidad y acceso | 7 | 7 producción |
| Lógica de negocio | 7 | 7 producción |
| Interfaces de acceso | 9 | 9 producción |
| Red y comunicación | 6 | 6 producción |
| Operación y observabilidad | 8 | 8 producción |
| **Total** | **58** | **57 producción, 1 pre-lab** |

---

## Tests

1,445 tests automatizados. 0 failures. Cobertura de todos los subsistemas.
