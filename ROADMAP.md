# Roadmap rust-bc — Gap Fabric 2.5

Referencia: [Hyperledger Fabric 2.5](https://hyperledger-fabric.readthedocs.io/en/release-2.5/).

Cada tarea es atómica: un archivo, un struct, un método, o un test. Sin evaluaciones
pendientes — cada decisión técnica ya está tomada basándose en lo que existe en el codebase.

---

## Estado actual (2026-04-04, 1612 tests — Fase 12 completa)

| Capa | Archivo(s) | Qué hay |
|------|-----------|---------|
| Storage | `storage/{traits,memory,adapters}.rs` | RocksDB + MemoryStore, CFs: blocks/txs/identities/credentials/meta/tx_by_block/cred_by_subject |
| REST API | `api/handlers/*.rs`, `api/routes.rs` | Actix-Web 4 bajo `/api/v1`, store endpoints CRUD + secondary indexes |
| Consensus | `consensus/{dag,engine,fork_choice,scheduler,validator}.rs` | DAG + HeaviestSubtree fork choice + SlotScheduler round-robin + BlockValidator pipeline |
| Identity | `identity/{did,keys}.rs` | `DidDocument` (did:bc:*), `KeyManager` Ed25519 (sign/verify/rotate) |
| PKI | `pki.rs` | CA interna con `rcgen`, firma certificados X.509 para nodos |
| TLS | `tls.rs` | mTLS con rustls 0.23, OCSP stapling, cert pinning |
| P2P | `network.rs` (2256 ln) | TCP/TLS server, message handling, peer discovery, bootstrap/seed nodes, allowlist, rate limiting, contract gossip |
| Smart Contracts | `smart_contracts.rs` | ERC-20 + ERC-721, ContractManager, validation, integrity hash |
| Multisig | `multisig_contracts.rs` | `MultiSigContract` con threshold N-de-M, `PendingOperation`, `add_signature`, `is_approved` |
| Governance | `governance_contracts.rs` | DAO-like: `Proposal`, votación ponderada, quorum, timelock, ejecución |
| TX Validation | `transaction_validation.rs` | Sequence tracking, fee validation, replay prevention, double-spend check |

---

## Fase 1 — Endorsement Policies

> Reutiliza `multisig_contracts.rs` (N-de-M) + `identity/keys.rs` (Ed25519 verify).
> No se introduce X.509 para endorsements — se usa Ed25519 DID signing, que ya existe.

### 1.1 Organization model

- [x] **1.1.1** Crear `src/endorsement/mod.rs` con re-exports; declarar `mod endorsement` en `lib.rs` y `main.rs`
- [x] **1.1.2** Crear struct `Organization` en `src/endorsement/org.rs`
  - Campos: `org_id: String`, `msp_id: String`, `admin_dids: Vec<String>`, `member_dids: Vec<String>`, `root_public_keys: Vec<[u8; 32]>`
  - Tipo `[u8; 32]` porque `identity/keys.rs` ya usa Ed25519 public keys como `[u8; 32]`
  - Tests: crear org con datos válidos, rechazar org sin admin_dids
- [x] **1.1.3** Crear trait `OrgRegistry` en `src/endorsement/registry.rs`
  - Métodos: `register_org(&self, org: &Organization) -> StorageResult<()>`, `get_org(&self, org_id: &str) -> StorageResult<Organization>`, `list_orgs(&self) -> StorageResult<Vec<Organization>>`, `remove_org(&self, org_id: &str) -> StorageResult<()>`
  - Implementar `MemoryOrgRegistry` con `Mutex<HashMap<String, Organization>>`
  - Tests: register, get, list, remove, get-not-found
- [x] **1.1.4** Añadir CF `organizations` a `adapters.rs`
  - Constante `CF_ORGANIZATIONS`, agregar a `ALL_CFS`, helper `cf_organizations()`
  - Implementar `OrgRegistry` para `RocksDbBlockStore`: key = `org_id`, value = JSON
  - Tests con `tempfile::TempDir`: write/read roundtrip, list, remove

### 1.2 Policy engine

- [x] **1.2.1** Crear enum `EndorsementPolicy` en `src/endorsement/policy.rs`
  - Variantes: `AnyOf(Vec<String>)`, `AllOf(Vec<String>)`, `NOutOf { n: usize, orgs: Vec<String> }`, `And(Box<Self>, Box<Self>)`, `Or(Box<Self>, Box<Self>)`
  - `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]`
  - Tests: serde roundtrip de cada variante con `serde_json`
- [x] **1.2.2** Implementar `EndorsementPolicy::evaluate(&self, signer_orgs: &[&str]) -> bool`
  - `AnyOf`: al menos 1 org presente. `AllOf`: todas. `NOutOf`: >= n. `And`/`Or`: recursivo.
  - Pattern: mismo approach que `MultiSigContract::is_approved` pero generalizado a orgs
  - Tests: 8 casos — AnyOf(0 match, 1 match), AllOf(parcial, completo), NOutOf(n-1, n, n+1), And(true+false, true+true), Or(false+false, false+true)
- [x] **1.2.3** Crear trait `PolicyStore` en `src/endorsement/policy_store.rs`
  - Métodos: `set_policy(&self, resource_id: &str, policy: &EndorsementPolicy) -> StorageResult<()>`, `get_policy(&self, resource_id: &str) -> StorageResult<EndorsementPolicy>`
  - Implementar `MemoryPolicyStore` con `Mutex<HashMap<String, EndorsementPolicy>>`
  - Tests: set/get, override, not-found

### 1.3 Endorsement verification

- [x] **1.3.1** Crear struct `Endorsement` en `src/endorsement/types.rs`
  - Campos: `signer_did: String`, `org_id: String`, `signature: [u8; 64]`, `payload_hash: [u8; 32]`, `timestamp: u64`
  - `[u8; 64]` porque `ed25519_dalek::Signature` es 64 bytes, igual que `DagBlock.signature`
  - Tests: crear endorsement, verificar tamaños
- [x] **1.3.2** Crear `fn verify_endorsement(e: &Endorsement, public_key: &[u8; 32]) -> Result<(), EndorsementError>` en `src/endorsement/validator.rs`
  - Usa `ed25519_dalek::VerifyingKey::from_bytes(public_key)` + `.verify(payload_hash, signature)`
  - Mismo patrón que `KeyManager::verify` en `identity/keys.rs`
  - Tests: firma válida (pass), firma inválida (fail), key incorrecta (fail)
- [x] **1.3.3** Crear `fn validate_endorsements(endorsements: &[Endorsement], policy: &EndorsementPolicy, registry: &dyn OrgRegistry) -> Result<(), EndorsementError>`
  - Para cada endorsement: verificar firma, resolver org_id via registry, colectar orgs únicas
  - Evaluar policy contra orgs colectadas
  - Tests: 3 endorsements de 2 orgs, policy NOutOf{2, [org1, org2, org3]} → pass; 1 endorsement de 1 org → fail

### 1.4 Integración con consensus

- [x] **1.4.1** Añadir campo `endorsements: Vec<Endorsement>` a `storage::traits::Block`
  - Default `vec![]` para backward compat (serde `#[serde(default)]`)
  - Tests: serializar bloque con y sin endorsements
- [x] **1.4.2** En `ConsensusEngine::accept_block`, si hay un `PolicyStore` adjunto, validar endorsements del bloque antes de insertar en DAG
  - Nuevo método `with_policy_store(store: Box<dyn PolicyStore>)` en `ConsensusEngine`
  - Si no hay policy store, skip (backward compat)
  - Tests: engine con policy → bloque sin endorsements rechazado; engine sin policy → bloque aceptado como antes

### 1.5 REST API

- [x] **1.5.1** Handler `POST /api/v1/store/organizations` en `src/api/handlers/organizations.rs`
  - Body: `Organization`, persiste via `OrgRegistry`
  - Respuesta: 201 con `ApiResponse<Organization>`
- [x] **1.5.2** Handler `GET /api/v1/store/organizations` — listar todas
- [x] **1.5.3** Handler `GET /api/v1/store/organizations/{org_id}` — leer una
- [x] **1.5.4** Handler `POST /api/v1/store/policies` — crear/actualizar policy para un resource_id
- [x] **1.5.5** Handler `GET /api/v1/store/policies/{resource_id}` — leer policy
- [x] **1.5.6** Registrar handlers en `routes.rs`: `store_organizations_routes()`, `store_policies_routes()`
- [x] **1.5.7** Añadir `org_registry: Option<Arc<dyn OrgRegistry>>` y `policy_store: Option<Arc<dyn PolicyStore>>` a `AppState`

---

## Fase 2 — Ordering Service

> No se introduce Raft. Se separa el rol de orderer del de peer reutilizando el
> `SlotScheduler` existente para batching. La capa de transporte usa los `Message`
> de `network.rs` + un nuevo variante `OrderedBlock`.

### 2.1 Node roles

- [x] **2.1.1** Crear enum `NodeRole` en `src/ordering/mod.rs`: `Peer`, `Orderer`, `PeerAndOrderer`
  - `impl FromStr` para parsear desde env `NODE_ROLE` (default: `PeerAndOrderer`)
  - Declarar `mod ordering` en `lib.rs`/`main.rs`
  - Tests: parse "peer" → `Peer`, "orderer" → `Orderer`, "" → `PeerAndOrderer`, "invalid" → error
- [x] **2.1.2** Añadir campo `role: NodeRole` a `Node` en `network.rs`
  - Pasar como argumento en `Node::with_role`; `Node::new` lee desde `NODE_ROLE` env
  - Tests: Node con role Peer, Node con role Orderer

### 2.2 Ordering service core

- [x] **2.2.1** Crear struct `OrderingService` en `src/ordering/service.rs`
  - Campos: `pending_txs: Mutex<VecDeque<storage::traits::Transaction>>`, `max_batch_size: usize`, `batch_timeout_ms: u64`
  - Configuración via `ORDERING_BATCH_SIZE` (default 100), `ORDERING_BATCH_TIMEOUT_MS` (default 2000)
  - Tests: crear service con defaults, verificar config
- [x] **2.2.2** Implementar `fn submit_tx(&self, tx: Transaction) -> StorageResult<()>`
  - Push a `pending_txs`
  - Tests: submit 3 txs, verificar pending count = 3
- [x] **2.2.3** Implementar `fn cut_block(&self, height: u64, proposer: &str) -> StorageResult<Option<storage::traits::Block>>`
  - Drain hasta `max_batch_size` txs de la cola, crear `Block` con esos tx ids
  - Retorna `None` si cola vacía
  - Tests: submit 5 txs con batch_size=3 → cut_block devuelve bloque con 3 txs, second cut devuelve 2

### 2.3 Integración con P2P

- [x] **2.3.1** Añadir variante `Message::SubmitTransaction(storage::traits::Transaction)` en `network.rs`
  - Peers envían TXs endorsadas al orderer via este mensaje
  - Tests: serde roundtrip
- [x] **2.3.2** Añadir variante `Message::OrderedBlock(storage::traits::Block)` en `network.rs`
  - Orderer difunde bloques ordenados a peers
  - Tests: serde roundtrip
- [x] **2.3.3** En `process_message`, si `role == Orderer` y recibe `SubmitTransaction`: llamar `ordering_service.submit_tx`
  - Si `role == Peer` y recibe `OrderedBlock`: escribir bloque en store sin re-ordenar
  - Tests: Orderer recibe TX → pending count sube; Peer recibe OrderedBlock → bloque en store

### 2.4 Batch timer

- [x] **2.4.1** Crear `async fn run_batch_loop(service: Arc<OrderingService>, store: Arc<dyn BlockStore>)` en `src/ordering/service.rs`
  - Loop: sleep `batch_timeout_ms`, luego `cut_block` si hay TXs pendientes
  - Se lanza como `tokio::spawn` en `main.rs` si `role == Orderer || role == PeerAndOrderer`
  - Tests: submit tx, esperar timeout, verificar bloque cortado

---

## Fase 3 — Transaction Lifecycle (Propose → Endorse → Order → Commit)

> No introduce MVCC todavía (eso es Fase 6). Solo el flujo de 3 fases con los
> building blocks de Fase 1 (endorsement) y Fase 2 (ordering).

### 3.1 Read-Write Sets

- [x] **3.1.1** Crear struct `KVRead { key: String, version: u64 }` y `KVWrite { key: String, value: Vec<u8> }` en `src/transaction/rwset.rs`
  - Declarar `mod transaction` en `lib.rs`/`main.rs`
  - Tests: crear, serializar, deserializar
- [x] **3.1.2** Crear struct `ReadWriteSet { reads: Vec<KVRead>, writes: Vec<KVWrite> }` en mismo archivo
  - Método `fn is_empty(&self) -> bool`
  - Tests: empty rwset, non-empty

### 3.2 Transaction Proposal

- [x] **3.2.1** Crear struct `TransactionProposal` en `src/transaction/proposal.rs`
  - Campos: `tx: storage::traits::Transaction`, `creator_did: String`, `creator_signature: [u8; 64]`, `rwset: ReadWriteSet`
  - Tests: crear proposal
- [x] **3.2.2** Crear struct `ProposalResponse` en mismo archivo
  - Campos: `rwset: ReadWriteSet`, `endorsement: Endorsement` (del módulo `endorsement`)
  - Tests: crear response

### 3.3 Endorsed Transaction

- [x] **3.3.1** Crear struct `EndorsedTransaction` en `src/transaction/endorsed.rs`
  - Campos: `proposal: TransactionProposal`, `endorsements: Vec<Endorsement>`, `rwset: ReadWriteSet`
  - Tests: crear endorsed tx con 2 endorsements

### 3.4 REST API lifecycle

- [x] **3.4.1** Handler `POST /api/v1/proposals` — recibe `TransactionProposal`, simula, devuelve `ProposalResponse` con endorsement del peer local
  - La simulación es: leer keys mencionados en tx.data, escribir el resultado → RWSet
  - Tests de integración con MemoryStore
- [x] **3.4.2** Handler `POST /api/v1/transactions/submit` — recibe `EndorsedTransaction`, valida endorsements vs policy, envía a ordering service
  - Tests: TX con endorsements válidos → submitted; TX con endorsements insuficientes → 400

---

## Fase 4 — Channels (multi-ledger)

> Cada channel es un `Arc<dyn BlockStore>` independiente. Se reutiliza todo el storage
> layer existente — solo cambia cómo se indexa en `AppState`.

### 4.1 Channel model

- [x] **4.1.1** Crear struct `Channel` en `src/channel/mod.rs`
  - Campos: `channel_id: String`, `member_org_ids: Vec<String>`, `orderer_org_ids: Vec<String>`, `created_at: u64`, `endorsement_policy: EndorsementPolicy`
  - Declarar `mod channel` en `lib.rs`/`main.rs`
  - Tests: crear channel, agregar org, verificar membership
- [x] **4.1.2** Crear trait `ChannelRegistry` + `MemoryChannelRegistry`
  - Métodos: `create_channel`, `get_channel`, `list_channels`, `update_channel`
  - Tests: CRUD, channel not found

### 4.2 Multi-store

- [x] **4.2.1** Cambiar `AppState.store` de `Option<Arc<dyn BlockStore>>` a `HashMap<String, Arc<dyn BlockStore>>`
  - Key `"default"` contiene el store actual
  - Actualizar todos los handlers para obtener store via `state.store.get("default")` — cambio mecánico
  - Tests: compilar, todos los tests existentes pasan sin cambios funcionales
- [x] **4.2.2** Crear helper `fn get_channel_store(state: &AppState, channel_id: &str) -> Result<Arc<dyn BlockStore>, ApiError>`
  - Lookup en `state.store`, error si channel no existe
  - Tests: get default (ok), get unknown (err)
- [x] **4.2.3** Para RocksDB: crear store por channel en subdirectorio `{STORAGE_PATH}/{channel_id}/`
  - `fn create_channel_store(channel_id: &str, base_path: &Path) -> StorageResult<RocksDbBlockStore>`
  - Tests con tempdir: crear 2 channel stores, verificar aislamiento

### 4.3 Channel-aware endpoints

- [x] **4.3.1** Añadir header opcional `X-Channel-Id` a todos los store handlers
  - Si ausente → `"default"`. Si presente → lookup en store map
  - Extraer con `req.headers().get("X-Channel-Id")`
  - Tests: request sin header → default, request con header → channel correcto, channel inexistente → 404
- [x] **4.3.2** Handler `POST /api/v1/channels` — crear channel, instanciar store, registrar en `AppState.store`
- [x] **4.3.3** Handler `GET /api/v1/channels` — listar channels

---

## Fase 5 — MSP (Membership Service Provider)

> Usa la PKI interna de `pki.rs` (rcgen + rustls) que ya existe. No introduce
> CouchDB ni nada externo. CRL se persiste en RocksDB.

### 5.1 MSP core

- [x] **5.1.1** Crear struct `Msp` en `src/msp/mod.rs`
  - Campos: `msp_id: String`, `root_public_keys: Vec<[u8; 32]>`, `revoked_serials: Vec<String>`, `org_id: String`
  - `[u8; 32]` = Ed25519 pubkey, consistente con `identity/keys.rs`
  - Declarar `mod msp` en `lib.rs`/`main.rs`
  - Tests: crear MSP
- [x] **5.1.2** Crear enum `MspRole` en mismo archivo: `Admin`, `Member`, `Client`, `Peer`, `Orderer`
  - `#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]`
  - Tests: serde roundtrip
- [x] **5.1.3** Crear struct `MspIdentity` en `src/msp/identity.rs`
  - Campos: `did: String`, `org_id: String`, `role: MspRole`, `public_key: [u8; 32]`
  - Tests: crear identity

### 5.2 Identity validation

- [x] **5.2.1** Implementar `Msp::validate_identity(&self, public_key: &[u8; 32]) -> Result<(), MspError>`
  - Verificar que `public_key` está firmado por alguna root_public_key (key attestation)
  - Verificar que el serial no está en `revoked_serials`
  - Tests: key válida (pass), key de otro MSP (fail), key revocada (fail)
- [x] **5.2.2** Implementar `Msp::revoke(&mut self, serial: &str)`
  - Push a `revoked_serials`
  - Tests: revoke → re-validate falla

### 5.3 CRL persistence

- [x] **5.3.1** Añadir CF `crl` a `adapters.rs`: key = `msp_id`, value = JSON `Vec<String>` (serials)
  - Agregar a `ALL_CFS`, helper `cf_crl()`
  - Tests: write/read CRL roundtrip
- [x] **5.3.2** Integrar CRL check en `validate_endorsements` (Fase 1.3.3)
  - Antes de aceptar endorsement, consultar CRL del MSP del signer
  - Tests: endorsement de signer revocado → rechazado

### 5.4 REST API

- [x] **5.4.1** Handler `POST /api/v1/msp/{msp_id}/revoke` — body: `{ "serial": "..." }`, añade a CRL
- [x] **5.4.2** Handler `GET /api/v1/msp/{msp_id}` — devuelve info del MSP (root keys count, CRL size)

---

## Fase 6 — World State con MVCC

> Añade versionado de keys y validación MVCC en commit. Reutiliza RocksDB con
> una nueva CF. No introduce CouchDB.

### 6.1 Versioned state

- [x] **6.1.1** Crear CF `world_state` en `adapters.rs`: key = `{key}`, value = JSON `VersionedValue { version: u64, data: Vec<u8> }`
  - Helper `cf_world_state()`, agregar a `ALL_CFS`
  - Tests: put key → version 1, put again → version 2, get → version correcta
- [x] **6.1.2** Crear trait `WorldState` en `src/storage/world_state.rs`
  - Métodos: `get(&self, key: &str) -> StorageResult<Option<VersionedValue>>`, `put(&self, key: &str, data: &[u8]) -> StorageResult<u64>` (retorna new version), `delete(&self, key: &str) -> StorageResult<()>`, `get_range(&self, start: &str, end: &str) -> StorageResult<Vec<(String, VersionedValue)>>`
  - Implementar para `RocksDbBlockStore` (prefix scan en world_state CF) y `MemoryStore` (BTreeMap para range)
  - Tests: get/put/delete, range query con 10 keys

### 6.2 MVCC validation

- [x] **6.2.1** Crear `fn validate_rwset(rwset: &ReadWriteSet, state: &dyn WorldState) -> Result<(), MvccConflict>` en `src/transaction/mvcc.rs`
  - Para cada `KVRead`: comparar `read.version` con `state.get(key).version`
  - Si difieren → `MvccConflict { key, read_version, current_version }`
  - Tests: sin conflicto (pass), con conflicto en 1 key (fail)
- [x] **6.2.2** Integrar MVCC en commit path: al aplicar bloque, validar cada TX
  - TXs inválidas se marcan como `state: "mvcc_conflict"` pero el bloque no se rechaza (como Fabric)
  - Tests: bloque con 3 TXs, 1 conflicto MVCC → 2 committed, 1 marcada conflict

### 6.3 Composite keys

- [x] **6.3.1** Crear `fn composite_key(object_type: &str, attrs: &[&str]) -> String` en `src/storage/world_state.rs`
  - Formato: `\x00{type}\x00{attr1}\x00{attr2}\x00`
  - `fn parse_composite_key(key: &str) -> Option<(String, Vec<String>)>`
  - Tests: create → parse roundtrip, partial prefix matches
- [x] **6.3.2** Crear `fn get_by_partial_key(state: &dyn WorldState, object_type: &str, partial: &[&str]) -> StorageResult<Vec<(String, VersionedValue)>>`
  - Usa `get_range` con el prefix construido por `composite_key`
  - Tests: 5 assets, partial query por type devuelve 5, partial por type+attr1 devuelve subset

---

## Fase 7 — Private Data Collections

### 7.1 Core

- [x] **7.1.1** Crear struct `PrivateDataCollection` en `src/private_data/mod.rs`
  - Campos: `name: String`, `member_org_ids: Vec<String>`, `required_peer_count: usize`, `blocks_to_live: u64`
  - Declarar `mod private_data` en `lib.rs`/`main.rs`
  - Tests: crear collection, verificar membership check
- [x] **7.1.2** Crear CF `private_{collection_name}` dinámicamente en RocksDB para cada collection
  - Hash SHA-256 del dato va on-chain (en TX data field), dato real en side CF
  - Tests: write private data → hash in block tx, dato en side CF, get private data → matches
- [x] **7.1.3** Implementar purge: `fn purge_expired(&self, current_height: u64)` borra datos con `blocks_to_live` expirado
  - Tests: dato con blocks_to_live=5, escribir 6 bloques → purge → dato ausente, hash persiste

### 7.2 Access control

- [x] **7.2.1** En los handlers de private data, verificar que el caller pertenece a una org miembro de la collection
  - Reutilizar `OrgRegistry` + `MspIdentity` de Fases 1 y 5
  - Tests: miembro accede (ok), no-miembro → 403

---

## Fase 8 — Chaincode Lifecycle

> Usa Wasm (Wasmtime) como sandbox. Wasmtime porque tiene soporte nativo async/tokio
> y es el runtime Wasm más usado en Rust (bytecodealliance).

### 8.1 Lifecycle states

- [x] **8.1.1** Crear enum `ChaincodeStatus` en `src/chaincode/mod.rs`: `Installed`, `Approved`, `Committed`, `Deprecated`
  - Declarar `mod chaincode` en `lib.rs`/`main.rs`
  - Tests: transiciones válidas (Installed→Approved→Committed), inválidas (Installed→Committed → error)
- [x] **8.1.2** Crear struct `ChaincodeDefinition` en `src/chaincode/definition.rs`
  - Campos: `chaincode_id: String`, `version: String`, `status: ChaincodeStatus`, `endorsement_policy: EndorsementPolicy`, `approvals: HashMap<String, bool>` (org_id → approved)
  - Tests: crear definition

### 8.2 Package storage

- [x] **8.2.1** Añadir CF `chaincode_packages` a `adapters.rs`: key = `chaincode_id:version`, value = Wasm bytes
  - Tests: store 100KB wasm, read back, bytes match
- [x] **8.2.2** Handler `POST /api/v1/chaincode/install` — recibe Wasm binary, almacena en CF
- [x] **8.2.3** Handler `POST /api/v1/chaincode/{id}/approve` — requiere firma de admin de la org; actualiza approvals en definition
- [x] **8.2.4** Handler `POST /api/v1/chaincode/{id}/commit` — verifica approvals de mayoría de orgs (via `EndorsementPolicy::evaluate`), cambia status a `Committed`

### 8.3 Wasm execution

- [x] **8.3.1** Agregar `wasmtime = "21"` a `Cargo.toml`
- [x] **8.3.2** Crear `WasmExecutor` en `src/chaincode/executor.rs`
  - Constructor: `fn new(wasm_bytes: &[u8], fuel_limit: u64) -> Result<Self, ChaincodeError>`
  - Usa `wasmtime::Engine` + `Store` con fuel metering para limitar CPU
  - Tests: cargar wasm válido (ok), wasm inválido (err)
- [x] **8.3.3** Exponer host functions al Wasm: `get_state(key) -> bytes`, `put_state(key, bytes)`
  - Estas funciones leen/escriben en `WorldState` (Fase 6)
  - Tests: wasm chaincode que hace put("x", "1") + get("x") → retorna "1"
- [x] **8.3.4** Implementar memory limit: `wasmtime::StoreLimitsBuilder::memory_size(max_bytes)`
  - Tests: wasm que intenta allocar más de max → trapped

---

## Fase 9 — Fabric Gateway

> Gateway es un handler Actix-Web que orquesta el lifecycle completo: endorse → order → commit.
> Reutiliza los building blocks de Fases 1–3.

- [x] **9.1.1** Crear struct `Gateway` en `src/gateway/mod.rs`
  - Campos: `org_registry: Arc<dyn OrgRegistry>`, `policy_store: Arc<dyn PolicyStore>`, `ordering_service: Arc<OrderingService>`, `store: Arc<dyn BlockStore>`
  - Declarar `mod gateway` en `lib.rs`/`main.rs`
  - Tests: crear gateway con mocks
- [x] **9.1.2** Implementar `Gateway::submit(&self, tx: Transaction) -> Result<TxResult, GatewayError>`
  - Paso 1: consultar policy del chaincode → determinar qué orgs necesitan endorsar
  - Paso 2: (en esta versión single-node) generar endorsement local
  - Paso 3: enviar endorsed TX a ordering service
  - Paso 4: esperar bloque con la TX → retornar resultado
  - Tests: submit TX completo → bloque cortado → TX committed
- [x] **9.1.3** Handler `POST /api/v1/gateway/submit` — delega a `Gateway::submit`
  - Tests de integración: request HTTP → TX procesada end-to-end

---

## Fase 10 — Service Discovery

> Fabric permite a los clientes descubrir dinámicamente qué peers endorsan qué chaincode
> y en qué channel. Sin esto, el cliente debe saber de antemano a quién pedir endorsements.

### 10.1 Discovery registry

- [x] **10.1.1** Crear struct `PeerDescriptor` en `src/discovery/mod.rs`
  - Campos: `peer_address: String`, `org_id: String`, `role: NodeRole`, `chaincodes: Vec<String>`, `channels: Vec<String>`, `last_heartbeat: u64`
  - Declarar `mod discovery` en `lib.rs`/`main.rs`
  - Tests: crear descriptor, serializar/deserializar
- [x] **10.1.2** Crear struct `DiscoveryService` en `src/discovery/service.rs`
  - Campos: `peers: Mutex<HashMap<String, PeerDescriptor>>`, `org_registry: Arc<dyn OrgRegistry>`, `policy_store: Arc<dyn PolicyStore>`
  - Métodos: `register_peer(&self, desc: PeerDescriptor)`, `unregister_peer(&self, address: &str)`, `heartbeat(&self, address: &str)`
  - Tests: register 3 peers, heartbeat actualiza timestamp, unregister remueve

### 10.2 Endorsement plan

- [x] **10.2.1** Implementar `DiscoveryService::endorsement_plan(&self, chaincode_id: &str, channel_id: &str) -> Result<Vec<PeerDescriptor>, DiscoveryError>`
  - Consulta policy del chaincode, filtra peers que pertenecen a orgs requeridas y tienen el chaincode instalado
  - Retorna el conjunto mínimo de peers para satisfacer la policy
  - Tests: policy NOutOf{2, [org1, org2, org3]} con 5 peers → retorna 2 peers de 2 orgs distintas
- [x] **10.2.2** Implementar `DiscoveryService::channel_peers(&self, channel_id: &str) -> Vec<PeerDescriptor>`
  - Filtra peers que participan en el channel dado
  - Tests: 5 peers, 3 en channel "mychannel" → retorna 3

### 10.3 REST API

- [x] **10.3.1** Handler `GET /api/v1/discovery/endorsers?chaincode={id}&channel={id}` — retorna endorsement plan
- [x] **10.3.2** Handler `GET /api/v1/discovery/peers?channel={id}` — retorna peers del channel
- [x] **10.3.3** Handler `POST /api/v1/discovery/register` — un peer se registra en el discovery service (llamado al boot)

### 10.4 Integración con Gateway

- [x] **10.4.1** En `Gateway::submit` (Fase 9.1.2), usar `DiscoveryService::endorsement_plan` para determinar a qué peers enviar la proposal
  - En vez de hardcodear el peer local, consultar discovery → enviar proposals a peers del plan
  - Tests: gateway usa discovery para encontrar endorsers, submit funciona end-to-end

---

## Fase 11 — Block Event Subscriptions

> Fabric permite a clientes suscribirse a eventos de bloques y TXs vía gRPC streams.
> Implementamos el equivalente con WebSocket (Actix-Web ya soporta WS) para push notifications.

### 11.1 Event bus

- [x] **11.1.1** Crear struct `EventBus` en `src/events/mod.rs`
  - Usa `tokio::sync::broadcast::channel` para fan-out a múltiples suscriptores
  - Declarar `mod events` en `lib.rs`/`main.rs`
  - Tests: 3 receivers, enviar evento → los 3 lo reciben
- [x] **11.1.2** Crear enum `BlockEvent` en `src/events/types.rs`
  - Variantes: `BlockCommitted { height: u64, tx_count: usize }`, `TransactionCommitted { tx_id: String, block_height: u64, valid: bool }`, `ChaincodeEvent { chaincode_id: String, event_name: String, payload: Vec<u8> }`
  - Tests: serde roundtrip de cada variante

### 11.2 Emisión de eventos

- [x] **11.2.1** Añadir `event_bus: Arc<EventBus>` a `AppState`
  - En el commit path (donde el peer escribe el bloque en store), emitir `BlockCommitted` y `TransactionCommitted` por cada TX del bloque
  - Tests: escribir bloque con 3 TXs → event bus recibe 1 BlockCommitted + 3 TransactionCommitted
- [x] **11.2.2** En `WasmExecutor` (Fase 8.3), exponer host function `set_event(name, payload)` que emite `ChaincodeEvent`
  - Tests: chaincode emite evento → EventBus lo recibe

### 11.3 WebSocket endpoint

- [x] **11.3.1** Handler `GET /api/v1/events/blocks` — WebSocket que envía `BlockEvent` en JSON a cada suscriptor
  - Usa `actix_web::web::Payload` + `actix_ws` (Actix-Web tiene soporte WS nativo)
  - Cada conexión WS se suscribe al `broadcast::channel` del `EventBus`
  - Tests: conectar WS, escribir bloque, WS recibe evento
- [x] **11.3.2** Soporte de filtros: el cliente envía un mensaje JSON al conectar con `{ "channel_id": "...", "chaincode_id": "..." }` para filtrar eventos
  - Si no envía filtro → recibe todo
  - Tests: filtro por channel → solo recibe eventos de ese channel

### 11.4 REST fallback (long-polling)

- [x] **11.4.1** Handler `GET /api/v1/events/blocks?from_height={n}` — retorna bloques desde height N
  - Para clientes que no soportan WebSocket
  - El cliente hace polling periódico con el último height recibido
  - Tests: escribir 3 bloques, query from_height=2 → retorna 2 bloques

---

## Fase 12 — Hardening

### 12.1 Gossip improvements

- [x] **12.1.1** Añadir push-gossip a `network.rs`: al recibir bloque nuevo, re-enviar `Message::NewBlock` a N peers aleatorios (ya existe parcialmente para contratos en `process_message`)
  - Tests: 3 nodos en-proceso, nodo A recibe bloque → nodo B y C lo reciben via gossip

### 12.2 Observabilidad

- [x] **12.2.1** Añadir métricas Prometheus en `metrics.rs` para: `endorsement_validations_total`, `ordering_blocks_cut_total`, `mvcc_conflicts_total`, `event_subscriptions_active`, `discovery_peers_registered`
  - Reutilizar `prometheus` crate que ya está en Cargo.toml
  - Tests: incrementar counter, verificar valor

### 12.3 Benchmarks

- [x] **12.3.1** Benchmark con Criterion: ordering service throughput (TXs/segundo con batch_size=100)
- [x] **12.3.2** Benchmark: endorsement validation latency (N endorsements contra policy)
- [x] **12.3.3** Benchmark: event bus fan-out latency (1 evento → N suscriptores)

---

## Fase 13 — ACLs + Channel Configuration

> Fabric gobierna cada recurso mediante ACLs (resource → policy) y cada canal mediante
> configuration transactions. Actualmente los canales son structs en memoria sin governance.
> Se reutiliza `EndorsementPolicy` como motor de evaluación y RocksDB para persistencia.

### 13.1 ACL framework

- [x] **13.1.1** Crear struct `AclEntry` en `src/acl/mod.rs`
  - Campos: `resource: String`, `policy_ref: String` (apunta a una `EndorsementPolicy` por nombre)
  - Declarar `mod acl` en `lib.rs`/`main.rs`
  - Tests: crear entry, serde roundtrip
- [x] **13.1.2** Crear trait `AclProvider` en `src/acl/provider.rs`
  - Métodos: `set_acl(&self, resource: &str, policy_ref: &str) -> StorageResult<()>`, `get_acl(&self, resource: &str) -> StorageResult<Option<AclEntry>>`, `list_acls(&self) -> StorageResult<Vec<AclEntry>>`, `remove_acl(&self, resource: &str) -> StorageResult<()>`
  - Implementar `MemoryAclProvider` con `Mutex<HashMap<String, AclEntry>>`
  - Tests: set/get/list/remove, get-not-found
- [x] **13.1.3** Añadir CF `acls` a `adapters.rs`: key = `resource`, value = JSON `AclEntry`
  - Constante `CF_ACLS`, agregar a `ALL_CFS`, helper `cf_acls()`
  - Implementar `AclProvider` para `RocksDbBlockStore`
  - Tests con `tempfile::TempDir`: write/read roundtrip, list, remove
- [x] **13.1.4** Crear `fn check_access(acl_provider: &dyn AclProvider, policy_store: &dyn PolicyStore, resource: &str, caller_orgs: &[&str]) -> Result<(), AclError>` en `src/acl/checker.rs`
  - Resuelve `policy_ref` del ACL entry → obtiene `EndorsementPolicy` del `PolicyStore` → evalúa contra `caller_orgs`
  - Tests: acceso permitido (pass), policy no satisfecha (deny), ACL no definida (deny por defecto)
- [x] **13.1.5** Crear enum `AclResource` con constantes para recursos estándar Fabric en `src/acl/resources.rs`
  - Variantes: `ChaincodeInvoke`, `ChaincodeQuery`, `BlockEvents`, `ChannelConfig`, `PeerDiscovery`, `PrivateDataRead`, `PrivateDataWrite`, `Custom(String)`
  - `fn resource_name(&self) -> &str` — retorna string canónico (e.g., `"peer/ChaincodeToChaincode"`)
  - Tests: resource_name roundtrip

### 13.2 Channel configuration transactions

- [x] **13.2.1** Crear struct `ChannelConfig` en `src/channel/config.rs`
  - Campos: `version: u64`, `member_orgs: Vec<String>`, `orderer_orgs: Vec<String>`, `endorsement_policy: EndorsementPolicy`, `acls: HashMap<String, String>`, `batch_size: usize`, `batch_timeout_ms: u64`, `anchor_peers: HashMap<String, Vec<String>>` (org_id → peer addresses)
  - Tests: crear config, serde roundtrip, default values
- [x] **13.2.2** Crear enum `ConfigUpdateType` en `src/channel/config.rs`
  - Variantes: `AddOrg(String)`, `RemoveOrg(String)`, `SetPolicy(EndorsementPolicy)`, `SetAcl { resource: String, policy_ref: String }`, `SetBatchSize(usize)`, `SetBatchTimeout(u64)`, `SetAnchorPeer { org_id: String, peer_address: String }`
  - Tests: serde roundtrip de cada variante
- [x] **13.2.3** Crear struct `ConfigTransaction` en `src/channel/config.rs`
  - Campos: `tx_id: String`, `channel_id: String`, `updates: Vec<ConfigUpdateType>`, `signatures: Vec<Endorsement>`, `created_at: u64`
  - Tests: crear config tx con múltiples updates
- [x] **13.2.4** Implementar `fn apply_config_update(config: &ChannelConfig, updates: &[ConfigUpdateType]) -> Result<ChannelConfig, ChannelError>` en `src/channel/config.rs`
  - Retorna nuevo `ChannelConfig` (inmutable) con los cambios aplicados
  - Tests: agregar org, cambiar policy, agregar anchor peer, batch size update
- [x] **13.2.5** Implementar `fn validate_config_tx(tx: &ConfigTransaction, current_config: &ChannelConfig, policy_store: &dyn PolicyStore, org_registry: &dyn OrgRegistry) -> Result<(), ChannelError>`
  - Verificar que las firmas satisfacen la modification policy del canal
  - Tests: firmas suficientes (pass), insuficientes (fail)
- [x] **13.2.6** Añadir CF `channel_configs` a `adapters.rs`: key = `{channel_id}:{version:012}`, value = JSON `ChannelConfig`
  - Tests: write/read, list versions por channel

### 13.3 Genesis block

- [x] **13.3.1** Crear `fn create_genesis_block(channel_id: &str, config: &ChannelConfig) -> Block` en `src/channel/genesis.rs`
  - Height 0, transactions contiene serialized `ChannelConfig` como JSON string en tx.id
  - `parent_hash = [0u8; 32]`, `proposer = "genesis"`
  - Tests: crear genesis block, deserializar config desde block
- [x] **13.3.2** En `Channel::new()` o handler `POST /api/v1/channels`, generar genesis block automáticamente y escribirlo en el store del channel
  - Tests: crear channel → store contiene genesis block en height 0

### 13.4 REST API

- [x] **13.4.1** Handler `POST /api/v1/channels/{channel_id}/config` — submit config update transaction
  - Body: `ConfigTransaction`, valida firmas, aplica update, persiste nueva version
  - Respuesta: 200 con `ApiResponse<ChannelConfig>` (nueva config)
- [x] **13.4.2** Handler `GET /api/v1/channels/{channel_id}/config` — retorna config actual (latest version)
- [x] **13.4.3** Handler `GET /api/v1/channels/{channel_id}/config/history` — retorna todas las versiones de config
- [x] **13.4.4** Handler `POST /api/v1/acls` — set ACL entry. Body: `{ "resource": "...", "policy_ref": "..." }`
- [x] **13.4.5** Handler `GET /api/v1/acls` — listar todas las ACL entries
- [x] **13.4.6** Handler `GET /api/v1/acls/{resource}` — obtener ACL de un recurso
- [x] **13.4.7** Añadir `acl_provider: Option<Arc<dyn AclProvider>>` a `AppState`
  - Registrar routes en `routes.rs`

---

## Fase 14 — Chaincode Simulation + Key-level Endorsement

> Fabric ejecuta chaincode en modo simulación (sin commit al state) para generar el rwset.
> Además permite override de endorsement policy a nivel de key individual.
> Se reutiliza `WasmExecutor` y `WorldState`.

### 14.1 Simulation (execute sin commit)

- [x] **14.1.1** Crear struct `SimulationWorldState` en `src/chaincode/simulation.rs` que wrappea un `Arc<dyn WorldState>` de solo lectura + buffer de writes local
  - Campos: `base_state: Arc<dyn WorldState>`, `write_buffer: Mutex<HashMap<String, Vec<u8>>>`, `read_set: Mutex<Vec<KVRead>>`, `delete_set: Mutex<Vec<String>>`
  - `get()`: buscar primero en write_buffer, luego en base_state; registrar KVRead
  - `put()`: escribir solo en write_buffer (NO en base_state); NO incrementa versión real
  - `delete()`: marcar en delete_set
  - Implementar `WorldState` trait
  - Tests: simulate put → base_state no cambia; simulate get registra read; simulate put+get retorna valor local
- [x] **14.1.2** Crear `fn to_rwset(&self) -> ReadWriteSet` en `SimulationWorldState`
  - Construye `ReadWriteSet` desde `read_set` y `write_buffer`
  - Tests: simulate 3 reads + 2 writes → rwset con 3 KVReads + 2 KVWrites
- [x] **14.1.3** Crear `fn simulate(&self, state: Arc<dyn WorldState>, func_name: &str) -> Result<(Vec<u8>, ReadWriteSet), ChaincodeError>` en `WasmExecutor`
  - Crea `SimulationWorldState` wrapping `state`
  - Invoca chaincode normalmente via `invoke()` pero con el simulation wrapper
  - Retorna resultado + rwset generado
  - Tests: chaincode que hace put("a","1")+get("b") → rwset contiene write("a") y read("b"), base state sin cambios
- [x] **14.1.4** Handler `POST /api/v1/chaincode/{id}/simulate` — ejecuta chaincode sin commit
  - Body: `{ "function": "...", "args": [...] }`
  - Respuesta: `{ "result": "...", "rwset": { "reads": [...], "writes": [...] } }`
  - Tests de integración: simulate → state sin cambios, rwset presente en respuesta

### 14.2 Key-level endorsement

- [x] **14.2.1** Crear CF `key_endorsement_policies` en `adapters.rs`: key = `{state_key}`, value = JSON `EndorsementPolicy`
  - Constante `CF_KEY_ENDORSEMENT`, agregar a `ALL_CFS`, helper `cf_key_endorsement()`
  - Tests: write/read policy per key
- [x] **14.2.2** Crear trait `KeyEndorsementStore` en `src/endorsement/key_policy.rs`
  - Métodos: `set_key_policy(&self, key: &str, policy: &EndorsementPolicy) -> StorageResult<()>`, `get_key_policy(&self, key: &str) -> StorageResult<Option<EndorsementPolicy>>`, `delete_key_policy(&self, key: &str) -> StorageResult<()>`
  - Implementar `MemoryKeyEndorsementStore` con `Mutex<HashMap<String, EndorsementPolicy>>`
  - Implementar para `RocksDbBlockStore`
  - Tests: set/get/delete, not-found retorna None
- [x] **14.2.3** Exponer host function `set_key_endorsement_policy(key, policy_json)` en `WasmExecutor`
  - Chaincode puede llamar `set_key_endorsement_policy("asset:123", "{\"NOutOf\":{\"n\":2,\"orgs\":[\"org1\",\"org2\"]}}")`
  - Tests: wasm chaincode llama set_key_endorsement → policy persiste
- [x] **14.2.4** Modificar `validate_endorsements` (Fase 1.3.3) para consultar key-level policy
  - Para cada KVWrite en rwset: si existe key-level policy → usar esa en vez de chaincode-level
  - Prioridad: key-level > collection-level > chaincode-level
  - Tests: key con override policy → requiere endorsements distintos al chaincode-level

### 14.3 Gateway simulation integration

- [x] **14.3.1** En `Gateway::submit`, antes de ordering: si hay `WasmExecutor` disponible, simular TX para generar rwset automáticamente
  - Paso 1: simulate → obtener rwset
  - Paso 2: validar endorsements contra rwset keys (key-level policies)
  - Paso 3: si pasa → submit a ordering
  - Tests: gateway con simulación → rwset generado automáticamente, endorsement validado per-key

---

## Fase 15 — Raft Ordering

> Raft es el consenso recomendado por Fabric para ordering. Reemplaza el single-node
> batching con un cluster de orderers con leader election. Se usa la crate `raft` de
> tikv (tokio-compatible, la más usada en Rust).

### 15.1 Raft core ✅

- [x] **15.1.1** Agregar `raft = "0.7"` y `prost = "0.13"` a `Cargo.toml`
  - `raft` = tikv/raft (port de etcd Raft a Rust), `prost` para serialización protobuf interna de raft
  - Tests: compilar, import `raft::prelude::*`
- [x] **15.1.2** Crear struct `RaftNode` en `src/ordering/raft_node.rs`
  - Campos: `id: u64`, `raw_node: RawNode<MemStorage>`, `pending_proposals: Vec<Vec<u8>>`, `committed_entries: Vec<Entry>`
  - Constructor: `fn new(id: u64, peers: Vec<u64>) -> Self` — configura `Config` con `election_tick=10`, `heartbeat_tick=3`
  - Tests: crear nodo, verificar que está en estado Follower
- [x] **15.1.3** Implementar `RaftNode::propose(&mut self, data: Vec<u8>) -> Result<(), RaftError>`
  - Llama `self.raw_node.propose(vec![], data)`
  - Tests: proponer dato en leader → entry committed
- [x] **15.1.4** Implementar `RaftNode::tick(&mut self)` y `RaftNode::step(&mut self, msg: Message)`
  - `tick()` avanza el reloj lógico; `step()` procesa mensajes Raft entrantes
  - Tests: 3 nodos, tick suficiente → leader elected; step con AppendEntries → follower acepta
- [x] **15.1.5** Implementar `RaftNode::advance(&mut self) -> Vec<CommittedEntry>`
  - Llama `raw_node.ready()`, procesa `committed_entries`, aplica `advance()`
  - Retorna entries committed para que el caller corte bloques
  - Tests: propose 5 entries en leader → advance retorna 5 CommittedEntry en todos los nodos

### 15.2 Raft ordering service ✅

- [x] **15.2.1** Crear struct `RaftOrderingService` en `src/ordering/raft_service.rs`
  - Campos: `raft_node: Mutex<RaftNode>`, `max_batch_size: usize`, `batch_timeout_ms: u64`, `metrics: Option<Arc<MetricsCollector>>`
  - Implementar misma interfaz que `OrderingService`: `submit_tx(&self, tx: Transaction)`, `cut_block(&self, height: u64, proposer: &str) -> Option<Block>`
  - `submit_tx`: serializa TX → `raft_node.propose(serialized)`
  - `cut_block`: drena committed entries → deserializa TXs → corta bloque
  - Tests: submit 3 TXs → cut_block retorna bloque con 3 TXs
- [x] **15.2.2** Crear trait `OrderingBackend` en `src/ordering/mod.rs`
  - Métodos: `submit_tx(&self, tx: Transaction) -> StorageResult<()>`, `cut_block(&self, height: u64, proposer: &str) -> StorageResult<Option<Block>>`, `pending_count(&self) -> usize`
  - Implementar para `OrderingService` (existente) y `RaftOrderingService`
  - Tests: trait object `Box<dyn OrderingBackend>` funciona con ambas implementaciones
- [x] **15.2.3** Selección de backend en `main.rs` via env `ORDERING_BACKEND=raft|solo`
  - `solo` = `OrderingService` actual (default, backward compat)
  - `raft` = `RaftOrderingService` con peers de `RAFT_PEERS` env (comma-separated `id:address`)
  - Tests: env `solo` → OrderingService, env `raft` → RaftOrderingService

### 15.3 Raft network transport ✅

- [x] **15.3.1** Añadir variante `RaftMessage(Vec<u8>)` a `Message` enum en `network.rs`
  - Serde: serializa como `{ "type": "raft_message", "data": "<base64>" }`
  - Tests: serde roundtrip de RaftMessage
- [x] **15.3.2** Crear `fn raft_transport_loop(raft_node: Arc<Mutex<RaftNode>>, peers: Arc<Mutex<HashSet<String>>>, tick_ms: u64)`
  - Cada `tick_ms` milisegundos: `raft_node.tick()`, procesa `Ready.messages` → envía via `Message::RaftMessage` a peers
  - Recibe `RaftMessage` en `process_message()` → deserializa → `raft_node.step(msg)`
  - Tests: 3 nodos in-process, proponer entry en nodo 1 → committed en los 3
- [x] **15.3.3** Implementar snapshot para catch-up de nodos rezagados
  - `RaftNode::create_snapshot(&self) -> Snapshot` — serializa estado actual (latest height + pending TXs)
  - `RaftNode::apply_snapshot(&mut self, snap: Snapshot)` — restore desde snapshot
  - Tests: nodo A tiene 100 entries, nodo B nuevo → snapshot transfer → nodo B sincronizado

### 15.4 Orderer block signing ✅

- [x] **15.4.1** Añadir campo `orderer_signature: Option<[u8; 64]>` a `Block` struct en `traits.rs`
  - `#[serde(default)]` para backward compat
  - Tests: bloque con y sin orderer_signature serializa correctamente
- [x] **15.4.2** En `cut_block()` de ambos backends, firmar bloque con la key del orderer
  - Usar `KeyManager::sign(block_hash)` donde `block_hash = sha256(height || parent_hash || merkle_root)`
  - Tests: bloque cortado → orderer_signature presente y verificable
- [x] **15.4.3** En peer commit path, verificar `orderer_signature` antes de aceptar bloque
  - Si orderer_signature presente → verificar contra orderer's known public key
  - Si ausente → aceptar (backward compat con bloques legacy)
  - Tests: bloque con firma válida (accept), firma inválida (reject), sin firma (accept)

---

## Fase 16 — Gossip Protocol Enhancement

> Fabric usa un protocolo gossip completo con pull-based sync, alive messages, state
> transfer, y anchor peers. El push-gossip actual (fanout=3) solo re-envía bloques nuevos.

### 16.1 Alive messages

- [x] **16.1.1** Crear struct `AliveMessage` en `src/network/gossip.rs` (nuevo módulo, extraer de network.rs)
  - Campos: `peer_address: String`, `org_id: String`, `timestamp: u64`, `sequence: u64`, `signature: [u8; 64]`
  - Declarar `mod gossip` en `src/network/mod.rs` (refactor: mover `network.rs` → `network/mod.rs`)
  - Tests: crear alive, serde roundtrip, verificar firma
- [x] **16.1.2** Añadir variante `Alive(AliveMessage)` al enum `Message`
  - Tests: serde roundtrip
- [x] **16.1.3** Implementar alive broadcast loop: cada `ALIVE_INTERVAL_MS` (default 5000), enviar `Alive` a todos los peers
  - Si no se recibe `Alive` de un peer en `ALIVE_TIMEOUT_MS` (default 15000) → marcar como sospechoso
  - Tests: 3 nodos, nodo C deja de enviar alive → nodos A,B lo marcan sospechoso tras timeout

### 16.2 Pull-based state sync

- [x] **16.2.1** Añadir variante `StateRequest { from_height: u64 }` al enum `Message`
  - Peer envía `StateRequest` para pedir bloques desde `from_height`
  - Tests: serde roundtrip
- [x] **16.2.2** Añadir variante `StateResponse { blocks: Vec<Block> }` al enum `Message`
  - Responde con hasta 50 bloques (configurable `STATE_BATCH_SIZE`)
  - Tests: serde roundtrip, limitar a batch size
- [x] **16.2.3** Implementar pull-sync loop: periódicamente (cada `PULL_INTERVAL_MS`, default 10000), comparar height local con peers
  - Si peer tiene height > local → enviar `StateRequest { from_height: local_height + 1 }`
  - Al recibir `StateResponse` → validar y escribir bloques en store
  - Tests: nodo A con 10 bloques, nodo B con 0 → pull sync → nodo B tiene 10 bloques
- [x] **16.2.4** Anti-entropy: al recibir `Alive` con info de height del peer, detectar gaps
  - Añadir `latest_height: u64` a `AliveMessage`
  - Si peer.latest_height > local_height → trigger pull sync
  - Tests: alive con height=20, local=15 → pull sync triggered

### 16.3 Anchor peers

- [x] **16.3.1** Crear struct `AnchorPeer` en `src/network/gossip.rs`
  - Campos: `peer_address: String`, `org_id: String`
  - Config via env `ANCHOR_PEERS` (comma-separated `org_id:address`)
  - Tests: parsear env, crear anchor peers
- [x] **16.3.2** En el bootstrap flow, conectar primero a anchor peers de cada org antes de general discovery
  - Anchor peers sirven como punto de entrada cross-org
  - Tests: 2 orgs con anchor peers, nodo nuevo se conecta via anchors → descubre peers de ambas orgs
- [x] **16.3.3** Integrar anchor peers en `ChannelConfig` (Fase 13.2.1): campo `anchor_peers` ya definido
  - Al recibir config update con nuevo anchor peer → actualizar gossip routing
  - Tests: config update añade anchor → gossip lo usa para nueva org

### 16.4 Leader election per org

- [x] **16.4.1** Crear enum `LeaderElectionMode` en `src/network/gossip.rs`: `Static`, `Dynamic`
  - Config via env `LEADER_ELECTION=static|dynamic` (default `static`)
- [x] **16.4.2** Implementar leader election dinámica: peers de una misma org eligen un leader que tira bloques del orderer
  - Leader: peer con menor `peer_address` (determinístico, sin protocolo de elección complejo)
  - Si leader falla (no alive) → siguiente peer asume
  - Tests: 3 peers de org1, leader = peer con menor address; leader muere → siguiente asume

---

## Fase 17 — Key History + Chaincode-to-Chaincode

> Fabric provee `getHistoryForKey` para trazar cambios de un key. También permite
> que un chaincode invoque a otro. Ambas features se exponen como host functions al Wasm.

### 17.1 Key history

- [x] **17.1.1** Añadir CF `key_history` a `adapters.rs`: key = `{state_key}\x00{version:012}`, value = JSON `HistoryEntry`
  - `HistoryEntry`: `{ version: u64, data: Vec<u8>, tx_id: String, timestamp: u64, is_delete: bool }`
  - Agregar a `ALL_CFS`, helper `cf_key_history()`
  - Tests: write 3 versions de misma key → read history retorna 3 entries ordenados
- [x] **17.1.2** Añadir método `get_history(&self, key: &str) -> StorageResult<Vec<HistoryEntry>>` al trait `WorldState`
  - Implementar para `MemoryWorldState`: mantener `history: HashMap<String, Vec<HistoryEntry>>`
  - Implementar para `RocksDbBlockStore`: prefix scan en CF `key_history` con `{key}\x00`
  - Tests: put 5 veces → history tiene 5 entries; delete → history tiene 6 entries con `is_delete=true`
- [x] **17.1.3** Modificar `WorldState::put()` y `WorldState::delete()` para escribir history entry
  - En cada put: append `HistoryEntry { version: new_version, data, tx_id: "", timestamp: now, is_delete: false }`
  - En cada delete: append con `is_delete: true`, data vacía
  - Tests: put("x","a") → put("x","b") → delete("x") → history = [v1:a, v2:b, v3:deleted]
- [x] **17.1.4** Exponer host function `get_history_for_key(key) -> Vec<HistoryEntry>` en `WasmExecutor`
  - ABI: retorna JSON serializado como bytes (ptr << 32 | len)
  - Tests: chaincode llama get_history → retorna entries correctos

### 17.2 Chaincode-to-chaincode invocation

- [x] **17.2.1** Crear trait `ChaincodeResolver` en `src/chaincode/resolver.rs`
  - Método: `fn resolve(&self, chaincode_id: &str) -> Result<Vec<u8>, ChaincodeError>` — retorna wasm bytes
  - Implementar `StoreBacked` que usa `ChaincodePackageStore::get_package`
  - Tests: resolver con chaincode existente (ok), inexistente (err)
- [x] **17.2.2** Exponer host function `invoke_chaincode(chaincode_id, function, args) -> bytes` en `WasmExecutor`
  - Internamente: resuelve wasm bytes via `ChaincodeResolver`, crea nuevo `WasmExecutor` temporal, invoca, retorna resultado
  - El chaincode invocado comparte el mismo `WorldState` (reads/writes son visibles)
  - Tests: chaincode A llama chaincode B que hace put("x","1") → chaincode A hace get("x") → "1"
- [x] **17.2.3** Añadir ACL check en invocación cross-chaincode
  - Verificar que el caller tiene permiso `ChaincodeInvoke` para el target chaincode (via `AclProvider` de Fase 13)
  - Tests: invocación con ACL permitida (ok), sin permiso (denied)
- [x] **17.2.4** Prevenir recursión infinita: límite de profundidad `MAX_CHAINCODE_DEPTH=8`
  - Pasar depth counter en cada invocación; si depth > max → error
  - Tests: chaincode recursivo → error al llegar a depth 8

---

## Fase 18 — Delivery Service

> Fabric tiene 3 modos de delivery: Deliver (bloques completos), DeliverFiltered (minimal),
> DeliverWithPrivateData. Actualmente solo hay WebSocket con filtro básico por channel/chaincode.

### 18.1 DeliverFiltered

- [x] **18.1.1** Crear struct `FilteredBlock` en `src/events/filtered.rs`
  - Campos: `channel_id: String`, `height: u64`, `tx_summaries: Vec<FilteredTx>`
  - `FilteredTx`: `{ tx_id: String, validation_code: String, chaincode_id: Option<String> }`
  - Omite: payload, rwset, endorsements (privacy)
  - Tests: crear FilteredBlock desde Block, verificar que no contiene datos sensibles
- [x] **18.1.2** Crear `fn to_filtered_block(block: &Block, validations: &HashMap<String, String>) -> FilteredBlock`
  - Convierte bloque completo → filtered (solo IDs + status)
  - Tests: bloque con 3 TXs (2 committed, 1 conflict) → FilteredBlock con 3 summaries
- [x] **18.1.3** Handler `GET /api/v1/events/blocks/filtered` — WebSocket que envía `FilteredBlock` en vez de `BlockEvent`
  - Misma mecánica de suscripción que `events_blocks` pero con payload reducido
  - Tests: conectar WS filtered, escribir bloque → recibe FilteredBlock sin datos sensibles

### 18.2 DeliverWithPrivateData

- [x] **18.2.1** Crear struct `BlockWithPrivateData` en `src/events/private_delivery.rs`
  - Campos: `block: Block`, `private_data: HashMap<String, Vec<(String, Vec<u8>)>>` (collection_name → [(key, value)])
  - Tests: crear BlockWithPrivateData, serde roundtrip
- [x] **18.2.2** Handler `GET /api/v1/events/blocks/private` — WebSocket que envía bloque + private data autorizada
  - Requiere header `X-Org-Id` para verificar membership en collections
  - Solo incluye private data de collections donde caller es member
  - Tests: org1 member de collection "secret" → recibe private data; org2 no member → no recibe

### 18.3 Replay desde height

- [x] **18.3.1** Mejorar el WebSocket handler existente para soportar replay desde un height específico
  - Al conectar, el cliente envía `{ "start_block": N }` junto con filtros
  - El handler primero envía bloques históricos [N, latest] desde el store, luego switchea a live
  - Tests: 10 bloques en store, WS con start_block=5 → recibe bloques 5-10, luego live
- [x] **18.3.2** Implementar bookmark/checkpoint: el cliente puede enviar `{ "ack": height }` para confirmar receipt
  - Servidor trackea último ack por suscriptor
  - Si WS reconecta con mismo client_id → resume desde último ack
  - Tests: recibir 5 bloques, ack height=5, desconectar, reconectar → resume desde 6

---

## Fase 19 — Snapshots + Pagination

> Fabric puede regenerar world state desde la blockchain y soporta snapshots para
> fast-sync de nuevos peers. La API necesita pagination para ser usable en producción.

### 19.1 State snapshots

- [x] **19.1.1** Crear struct `StateSnapshot` en `src/storage/snapshot.rs`
  - Campos: `snapshot_id: String`, `channel_id: String`, `block_height: u64`, `created_at: u64`, `state_hash: [u8; 32]`, `entry_count: u64`
  - Tests: crear snapshot metadata
- [x] **19.1.2** Implementar `fn create_snapshot(store: &dyn BlockStore, state: &dyn WorldState, channel_id: &str) -> StorageResult<StateSnapshot>` en `src/storage/snapshot.rs`
  - Itera todas las keys en world state, serializa a un archivo `snapshots/{channel_id}/{height}.snap`
  - Formato: lineas `{key}\t{version}\t{base64(data)}\n` (simple, streamable)
  - Calcula hash SHA-256 de todo el contenido
  - Tests: crear snapshot con 100 keys → archivo existe, hash verificable
- [x] **19.1.3** Implementar `fn restore_snapshot(path: &Path, state: &dyn WorldState) -> StorageResult<StateSnapshot>`
  - Lee archivo de snapshot → `state.put(key, data)` para cada entry
  - Verifica hash al final
  - Tests: create snapshot → clear state → restore → state idéntico al original
- [x] **19.1.4** Handler `POST /api/v1/snapshots/{channel_id}` — trigger snapshot creation
- [x] **19.1.5** Handler `GET /api/v1/snapshots/{channel_id}` — list available snapshots
- [x] **19.1.6** Handler `GET /api/v1/snapshots/{channel_id}/{snapshot_id}` — download snapshot file (streaming)

### 19.2 State regeneration

- [x] **19.2.1** Implementar `fn regenerate_state(store: &dyn BlockStore, state: &dyn WorldState, channel_id: &str) -> StorageResult<u64>` en `src/storage/snapshot.rs`
  - Itera bloques [0, latest] en orden → para cada TX con rwset → aplica writes al world state
  - Retorna número de keys escritas
  - Tests: 10 bloques con 3 TXs cada uno → regenerate → state contiene todas las keys

### 19.3 Pagination

- [x] **19.3.1** Crear struct `PaginationParams` en `src/api/pagination.rs`
  - Campos: `page: Option<usize>` (default 1), `limit: Option<usize>` (default 20, max 100), `cursor: Option<String>`
  - Implementar `actix_web::FromRequest` via `web::Query<PaginationParams>`
  - Tests: parsear query `?page=2&limit=10`, defaults, limit capping
- [x] **19.3.2** Crear struct `PaginatedResponse<T>` en `src/api/pagination.rs`
  - Campos: `data: Vec<T>`, `pagination: PaginationMeta`
  - `PaginationMeta`: `{ total: usize, page: usize, limit: usize, total_pages: usize, has_next: bool, next_cursor: Option<String> }`
  - Tests: crear PaginatedResponse, serde roundtrip
- [x] **19.3.3** Añadir método `list_blocks(&self, offset: usize, limit: usize) -> StorageResult<(Vec<Block>, usize)>` al trait `BlockStore`
  - Retorna `(blocks, total_count)` para paginación
  - Implementar para `MemoryStore` y `RocksDbBlockStore`
  - Tests: 50 bloques, list(offset=10, limit=5) → 5 bloques, total=50
- [x] **19.3.4** Actualizar handler `GET /api/v1/store/blocks` para aceptar `PaginationParams`
  - Retorna `ApiResponse<PaginatedResponse<Block>>` en vez de lista completa
  - Backward compat: si no hay query params → default page=1, limit=20
  - Tests: 50 bloques, GET ?page=3&limit=10 → bloques 20-29, total=50
- [x] **19.3.5** Actualizar handler `GET /api/v1/store/organizations` con paginación
- [x] **19.3.6** Actualizar handler `GET /api/v1/channels` con paginación
- [x] **19.3.7** Actualizar handler `GET /api/v1/acls` con paginación (Fase 13)
- [x] **19.3.8** Actualizar handler `GET /api/v1/discovery/peers` con paginación

---

## Fase 20 — HSM + OUs + External Chaincode

> Features avanzadas de Fabric: HSM (PKCS#11) para proteger keys privadas, Organizational
> Units para subdividir orgs, y chaincode-as-a-service para ejecución externa.

### 20.1 HSM support (PKCS#11)

- [ ] **20.1.1** Agregar `cryptoki = "0.7"` a `Cargo.toml` (feature-gated `hsm`)
  - `cryptoki` = binding Rust para PKCS#11, mantenido por Parallaxsecond
  - Feature flag: `[features] hsm = ["cryptoki"]`
  - Tests: compilar con `--features hsm`, import `cryptoki::context::Pkcs11`
- [ ] **20.1.2** Crear trait `SigningProvider` en `src/identity/signing.rs`
  - Métodos: `fn sign(&self, data: &[u8]) -> Result<[u8; 64], SigningError>`, `fn public_key(&self) -> [u8; 32]`, `fn verify(&self, data: &[u8], sig: &[u8; 64]) -> Result<bool, SigningError>`
  - Implementar `SoftwareSigningProvider` que wrappea `KeyManager` actual
  - Tests: sign + verify roundtrip con SoftwareSigningProvider
- [ ] **20.1.3** Implementar `HsmSigningProvider` (bajo feature `hsm`) en `src/identity/hsm.rs`
  - Constructor: `fn new(pkcs11_lib: &str, slot_id: u64, pin: &str, key_label: &str) -> Result<Self, HsmError>`
  - Sign via PKCS#11: `C_SignInit` + `C_Sign` con mecanismo `CKM_EDDSA`
  - Config via env: `HSM_PKCS11_LIB`, `HSM_SLOT_ID`, `HSM_PIN`, `HSM_KEY_LABEL`
  - Tests: con SoftHSM2 (si disponible) o mock PKCS#11
- [ ] **20.1.4** Refactorizar `KeyManager` para usar `dyn SigningProvider` internamente
  - `KeyManager::new()` → usa `SoftwareSigningProvider` por defecto
  - `KeyManager::with_hsm(config)` → usa `HsmSigningProvider`
  - Backward compat total: API pública no cambia
  - Tests: KeyManager con SoftwareSigningProvider funciona exactamente igual que antes

### 20.2 Organizational Units (OUs)

- [ ] **20.2.1** Crear struct `OrganizationalUnit` en `src/msp/ou.rs`
  - Campos: `ou_id: String`, `org_id: String`, `description: String`, `parent_ou: Option<String>`
  - Soporte jerárquico: OU puede tener parent OU
  - Tests: crear OU, OU con parent, serde roundtrip
- [ ] **20.2.2** Añadir campo `ou_id: Option<String>` a `MspIdentity`
  - `#[serde(default)]` para backward compat
  - Tests: identity con OU, identity sin OU (backward compat)
- [ ] **20.2.3** Crear trait `OuRegistry` en `src/msp/ou.rs`
  - Métodos: `register_ou(&self, ou: &OrganizationalUnit) -> StorageResult<()>`, `get_ou(&self, ou_id: &str) -> StorageResult<OrganizationalUnit>`, `list_ous(&self, org_id: &str) -> StorageResult<Vec<OrganizationalUnit>>`, `get_hierarchy(&self, ou_id: &str) -> StorageResult<Vec<OrganizationalUnit>>`
  - Implementar `MemoryOuRegistry` con `Mutex<HashMap<String, OrganizationalUnit>>`
  - Tests: register, get, list por org, hierarchy traversal
- [ ] **20.2.4** Añadir CF `organizational_units` a `adapters.rs`: key = `ou_id`, value = JSON
  - Implementar `OuRegistry` para `RocksDbBlockStore`
  - Tests: write/read, list por org_id via prefix scan
- [ ] **20.2.5** Extender `EndorsementPolicy` con variante `OuBased { ou_ids: Vec<String>, min_count: usize }`
  - Evaluate: contar identities cuyo OU está en `ou_ids`, verificar >= min_count
  - Tests: policy OuBased{["manufacturing"], 2} con 3 identities de manufacturing → pass; 1 → fail
- [ ] **20.2.6** REST API: `POST /api/v1/msp/ous`, `GET /api/v1/msp/ous?org_id={id}`, `GET /api/v1/msp/ous/{ou_id}`

### 20.3 External chaincode (chaincode-as-a-service)

- [ ] **20.3.1** Crear enum `ChaincodeRuntime` en `src/chaincode/external.rs`
  - Variantes: `Wasm { fuel_limit: u64, memory_limit: Option<usize> }`, `External { endpoint: String, tls: bool }`
  - Añadir campo `runtime: ChaincodeRuntime` a `ChaincodeDefinition` (`#[serde(default)]` → Wasm por default)
  - Tests: serde roundtrip de ambas variantes
- [ ] **20.3.2** Crear struct `ExternalChaincodeClient` en `src/chaincode/external.rs`
  - Constructor: `fn new(endpoint: &str, tls: bool) -> Result<Self, ChaincodeError>`
  - Método: `async fn invoke(&self, function: &str, args: &[&str], state_context: &str) -> Result<Vec<u8>, ChaincodeError>`
  - Protocolo: HTTP POST a `{endpoint}/invoke` con body `{ "function": "...", "args": [...], "state_context": "..." }`
  - Tests: mock HTTP server, invoke → respuesta parseada
- [ ] **20.3.3** Crear trait `ChaincodeInvoker` en `src/chaincode/invoker.rs`
  - Método: `fn invoke(&self, state: Arc<dyn WorldState>, func_name: &str) -> Result<Vec<u8>, ChaincodeError>`
  - Implementar `WasmInvoker` wrapping `WasmExecutor`
  - Implementar `ExternalInvoker` wrapping `ExternalChaincodeClient`
  - Tests: ambos invokers a través del trait object
- [ ] **20.3.4** Modificar Gateway para usar `dyn ChaincodeInvoker` según `ChaincodeRuntime`
  - Si Wasm → `WasmInvoker`, si External → `ExternalInvoker`
  - Tests: gateway con chaincode externo → invoca endpoint HTTP → resultado correcto
- [ ] **20.3.5** Handler `POST /api/v1/chaincode/install` actualizado para aceptar `runtime: "external"` + endpoint
  - Si runtime=external: no almacena wasm bytes, solo registra endpoint
  - Tests: install external chaincode → definition con runtime External

---

## Dependencias entre fases

```
Fases 1-12 (completadas) ──────────────────────────────────────────────
                                                                       │
Fase 13 (ACLs + Channel Config) ←── 1 (EndorsementPolicy) + 4 (Channels)
Fase 14 (Simulation + Key Endorsement) ←── 6 (WorldState) + 8 (WasmExecutor) + 1 (Endorsement)
Fase 15 (Raft Ordering) ←── 2 (OrderingService) + P2P (network.rs)
Fase 16 (Gossip Enhancement) ←── P2P (network.rs) + 13 (anchor peers en ChannelConfig)
Fase 17 (Key History + CC-to-CC) ←── 6 (WorldState) + 8 (WasmExecutor) + 13 (ACLs)
Fase 18 (Delivery Service) ←── 11 (Events) + 7 (Private Data)
Fase 19 (Snapshots + Pagination) ←── 6 (WorldState) + Storage layer
Fase 20 (HSM + OUs + External CC) ←── 5 (MSP) + 8 (Chaincode) + 1 (EndorsementPolicy)

Paralelizables:
  13, 15, 18, 19 — sin dependencias cruzadas entre sí
  14 depende de 13 (ACLs para key-level)
  16 depende de 13 (anchor peers en ChannelConfig)
  17 depende de 13 (ACLs para cross-chaincode)
  20 es independiente
```

---

*Última revisión: 2026-04-04. Cada decisión técnica está basada en el codebase actual (1612 tests).*
