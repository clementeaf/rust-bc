# Roadmap rust-bc — Gap Fabric 2.5

Referencia: [Hyperledger Fabric 2.5](https://hyperledger-fabric.readthedocs.io/en/release-2.5/).

Cada tarea es atómica: un archivo, un struct, un método, o un test. Sin evaluaciones
pendientes — cada decisión técnica ya está tomada basándose en lo que existe en el codebase.

---

## Estado actual (2026-04-03, 525 tests — Fase 2 completa)

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

- [ ] **3.1.1** Crear struct `KVRead { key: String, version: u64 }` y `KVWrite { key: String, value: Vec<u8> }` en `src/transaction/rwset.rs`
  - Declarar `mod transaction` en `lib.rs`/`main.rs`
  - Tests: crear, serializar, deserializar
- [ ] **3.1.2** Crear struct `ReadWriteSet { reads: Vec<KVRead>, writes: Vec<KVWrite> }` en mismo archivo
  - Método `fn is_empty(&self) -> bool`
  - Tests: empty rwset, non-empty

### 3.2 Transaction Proposal

- [ ] **3.2.1** Crear struct `TransactionProposal` en `src/transaction/proposal.rs`
  - Campos: `tx: storage::traits::Transaction`, `creator_did: String`, `creator_signature: [u8; 64]`, `rwset: ReadWriteSet`
  - Tests: crear proposal
- [ ] **3.2.2** Crear struct `ProposalResponse` en mismo archivo
  - Campos: `rwset: ReadWriteSet`, `endorsement: Endorsement` (del módulo `endorsement`)
  - Tests: crear response

### 3.3 Endorsed Transaction

- [ ] **3.3.1** Crear struct `EndorsedTransaction` en `src/transaction/endorsed.rs`
  - Campos: `proposal: TransactionProposal`, `endorsements: Vec<Endorsement>`, `rwset: ReadWriteSet`
  - Tests: crear endorsed tx con 2 endorsements

### 3.4 REST API lifecycle

- [ ] **3.4.1** Handler `POST /api/v1/proposals` — recibe `TransactionProposal`, simula, devuelve `ProposalResponse` con endorsement del peer local
  - La simulación es: leer keys mencionados en tx.data, escribir el resultado → RWSet
  - Tests de integración con MemoryStore
- [ ] **3.4.2** Handler `POST /api/v1/transactions/submit` — recibe `EndorsedTransaction`, valida endorsements vs policy, envía a ordering service
  - Tests: TX con endorsements válidos → submitted; TX con endorsements insuficientes → 400

---

## Fase 4 — Channels (multi-ledger)

> Cada channel es un `Arc<dyn BlockStore>` independiente. Se reutiliza todo el storage
> layer existente — solo cambia cómo se indexa en `AppState`.

### 4.1 Channel model

- [ ] **4.1.1** Crear struct `Channel` en `src/channel/mod.rs`
  - Campos: `channel_id: String`, `member_org_ids: Vec<String>`, `orderer_org_ids: Vec<String>`, `created_at: u64`, `endorsement_policy: EndorsementPolicy`
  - Declarar `mod channel` en `lib.rs`/`main.rs`
  - Tests: crear channel, agregar org, verificar membership
- [ ] **4.1.2** Crear trait `ChannelRegistry` + `MemoryChannelRegistry`
  - Métodos: `create_channel`, `get_channel`, `list_channels`, `update_channel`
  - Tests: CRUD, channel not found

### 4.2 Multi-store

- [ ] **4.2.1** Cambiar `AppState.store` de `Option<Arc<dyn BlockStore>>` a `HashMap<String, Arc<dyn BlockStore>>`
  - Key `"default"` contiene el store actual
  - Actualizar todos los handlers para obtener store via `state.store.get("default")` — cambio mecánico
  - Tests: compilar, todos los tests existentes pasan sin cambios funcionales
- [ ] **4.2.2** Crear helper `fn get_channel_store(state: &AppState, channel_id: &str) -> Result<Arc<dyn BlockStore>, ApiError>`
  - Lookup en `state.store`, error si channel no existe
  - Tests: get default (ok), get unknown (err)
- [ ] **4.2.3** Para RocksDB: crear store por channel en subdirectorio `{STORAGE_PATH}/{channel_id}/`
  - `fn create_channel_store(channel_id: &str, base_path: &Path) -> StorageResult<RocksDbBlockStore>`
  - Tests con tempdir: crear 2 channel stores, verificar aislamiento

### 4.3 Channel-aware endpoints

- [ ] **4.3.1** Añadir header opcional `X-Channel-Id` a todos los store handlers
  - Si ausente → `"default"`. Si presente → lookup en store map
  - Extraer con `req.headers().get("X-Channel-Id")`
  - Tests: request sin header → default, request con header → channel correcto, channel inexistente → 404
- [ ] **4.3.2** Handler `POST /api/v1/channels` — crear channel, instanciar store, registrar en `AppState.store`
- [ ] **4.3.3** Handler `GET /api/v1/channels` — listar channels

---

## Fase 5 — MSP (Membership Service Provider)

> Usa la PKI interna de `pki.rs` (rcgen + rustls) que ya existe. No introduce
> CouchDB ni nada externo. CRL se persiste en RocksDB.

### 5.1 MSP core

- [ ] **5.1.1** Crear struct `Msp` en `src/msp/mod.rs`
  - Campos: `msp_id: String`, `root_public_keys: Vec<[u8; 32]>`, `revoked_serials: Vec<String>`, `org_id: String`
  - `[u8; 32]` = Ed25519 pubkey, consistente con `identity/keys.rs`
  - Declarar `mod msp` en `lib.rs`/`main.rs`
  - Tests: crear MSP
- [ ] **5.1.2** Crear enum `MspRole` en mismo archivo: `Admin`, `Member`, `Client`, `Peer`, `Orderer`
  - `#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]`
  - Tests: serde roundtrip
- [ ] **5.1.3** Crear struct `MspIdentity` en `src/msp/identity.rs`
  - Campos: `did: String`, `org_id: String`, `role: MspRole`, `public_key: [u8; 32]`
  - Tests: crear identity

### 5.2 Identity validation

- [ ] **5.2.1** Implementar `Msp::validate_identity(&self, public_key: &[u8; 32]) -> Result<(), MspError>`
  - Verificar que `public_key` está firmado por alguna root_public_key (key attestation)
  - Verificar que el serial no está en `revoked_serials`
  - Tests: key válida (pass), key de otro MSP (fail), key revocada (fail)
- [ ] **5.2.2** Implementar `Msp::revoke(&mut self, serial: &str)`
  - Push a `revoked_serials`
  - Tests: revoke → re-validate falla

### 5.3 CRL persistence

- [ ] **5.3.1** Añadir CF `crl` a `adapters.rs`: key = `msp_id`, value = JSON `Vec<String>` (serials)
  - Agregar a `ALL_CFS`, helper `cf_crl()`
  - Tests: write/read CRL roundtrip
- [ ] **5.3.2** Integrar CRL check en `validate_endorsements` (Fase 1.3.3)
  - Antes de aceptar endorsement, consultar CRL del MSP del signer
  - Tests: endorsement de signer revocado → rechazado

### 5.4 REST API

- [ ] **5.4.1** Handler `POST /api/v1/msp/{msp_id}/revoke` — body: `{ "serial": "..." }`, añade a CRL
- [ ] **5.4.2** Handler `GET /api/v1/msp/{msp_id}` — devuelve info del MSP (root keys count, CRL size)

---

## Fase 6 — World State con MVCC

> Añade versionado de keys y validación MVCC en commit. Reutiliza RocksDB con
> una nueva CF. No introduce CouchDB.

### 6.1 Versioned state

- [ ] **6.1.1** Crear CF `world_state` en `adapters.rs`: key = `{key}`, value = JSON `VersionedValue { version: u64, data: Vec<u8> }`
  - Helper `cf_world_state()`, agregar a `ALL_CFS`
  - Tests: put key → version 1, put again → version 2, get → version correcta
- [ ] **6.1.2** Crear trait `WorldState` en `src/storage/world_state.rs`
  - Métodos: `get(&self, key: &str) -> StorageResult<Option<VersionedValue>>`, `put(&self, key: &str, data: &[u8]) -> StorageResult<u64>` (retorna new version), `delete(&self, key: &str) -> StorageResult<()>`, `get_range(&self, start: &str, end: &str) -> StorageResult<Vec<(String, VersionedValue)>>`
  - Implementar para `RocksDbBlockStore` (prefix scan en world_state CF) y `MemoryStore` (BTreeMap para range)
  - Tests: get/put/delete, range query con 10 keys

### 6.2 MVCC validation

- [ ] **6.2.1** Crear `fn validate_rwset(rwset: &ReadWriteSet, state: &dyn WorldState) -> Result<(), MvccConflict>` en `src/transaction/mvcc.rs`
  - Para cada `KVRead`: comparar `read.version` con `state.get(key).version`
  - Si difieren → `MvccConflict { key, read_version, current_version }`
  - Tests: sin conflicto (pass), con conflicto en 1 key (fail)
- [ ] **6.2.2** Integrar MVCC en commit path: al aplicar bloque, validar cada TX
  - TXs inválidas se marcan como `state: "mvcc_conflict"` pero el bloque no se rechaza (como Fabric)
  - Tests: bloque con 3 TXs, 1 conflicto MVCC → 2 committed, 1 marcada conflict

### 6.3 Composite keys

- [ ] **6.3.1** Crear `fn composite_key(object_type: &str, attrs: &[&str]) -> String` en `src/storage/world_state.rs`
  - Formato: `\x00{type}\x00{attr1}\x00{attr2}\x00`
  - `fn parse_composite_key(key: &str) -> Option<(String, Vec<String>)>`
  - Tests: create → parse roundtrip, partial prefix matches
- [ ] **6.3.2** Crear `fn get_by_partial_key(state: &dyn WorldState, object_type: &str, partial: &[&str]) -> StorageResult<Vec<(String, VersionedValue)>>`
  - Usa `get_range` con el prefix construido por `composite_key`
  - Tests: 5 assets, partial query por type devuelve 5, partial por type+attr1 devuelve subset

---

## Fase 7 — Private Data Collections

### 7.1 Core

- [ ] **7.1.1** Crear struct `PrivateDataCollection` en `src/private_data/mod.rs`
  - Campos: `name: String`, `member_org_ids: Vec<String>`, `required_peer_count: usize`, `blocks_to_live: u64`
  - Declarar `mod private_data` en `lib.rs`/`main.rs`
  - Tests: crear collection, verificar membership check
- [ ] **7.1.2** Crear CF `private_{collection_name}` dinámicamente en RocksDB para cada collection
  - Hash SHA-256 del dato va on-chain (en TX data field), dato real en side CF
  - Tests: write private data → hash in block tx, dato en side CF, get private data → matches
- [ ] **7.1.3** Implementar purge: `fn purge_expired(&self, current_height: u64)` borra datos con `blocks_to_live` expirado
  - Tests: dato con blocks_to_live=5, escribir 6 bloques → purge → dato ausente, hash persiste

### 7.2 Access control

- [ ] **7.2.1** En los handlers de private data, verificar que el caller pertenece a una org miembro de la collection
  - Reutilizar `OrgRegistry` + `MspIdentity` de Fases 1 y 5
  - Tests: miembro accede (ok), no-miembro → 403

---

## Fase 8 — Chaincode Lifecycle

> Usa Wasm (Wasmtime) como sandbox. Wasmtime porque tiene soporte nativo async/tokio
> y es el runtime Wasm más usado en Rust (bytecodealliance).

### 8.1 Lifecycle states

- [ ] **8.1.1** Crear enum `ChaincodeStatus` en `src/chaincode/mod.rs`: `Installed`, `Approved`, `Committed`, `Deprecated`
  - Declarar `mod chaincode` en `lib.rs`/`main.rs`
  - Tests: transiciones válidas (Installed→Approved→Committed), inválidas (Installed→Committed → error)
- [ ] **8.1.2** Crear struct `ChaincodeDefinition` en `src/chaincode/definition.rs`
  - Campos: `chaincode_id: String`, `version: String`, `status: ChaincodeStatus`, `endorsement_policy: EndorsementPolicy`, `approvals: HashMap<String, bool>` (org_id → approved)
  - Tests: crear definition

### 8.2 Package storage

- [ ] **8.2.1** Añadir CF `chaincode_packages` a `adapters.rs`: key = `chaincode_id:version`, value = Wasm bytes
  - Tests: store 100KB wasm, read back, bytes match
- [ ] **8.2.2** Handler `POST /api/v1/chaincode/install` — recibe Wasm binary, almacena en CF
- [ ] **8.2.3** Handler `POST /api/v1/chaincode/{id}/approve` — requiere firma de admin de la org; actualiza approvals en definition
- [ ] **8.2.4** Handler `POST /api/v1/chaincode/{id}/commit` — verifica approvals de mayoría de orgs (via `EndorsementPolicy::evaluate`), cambia status a `Committed`

### 8.3 Wasm execution

- [ ] **8.3.1** Agregar `wasmtime = "21"` a `Cargo.toml`
- [ ] **8.3.2** Crear `WasmExecutor` en `src/chaincode/executor.rs`
  - Constructor: `fn new(wasm_bytes: &[u8], fuel_limit: u64) -> Result<Self, ChaincodeError>`
  - Usa `wasmtime::Engine` + `Store` con fuel metering para limitar CPU
  - Tests: cargar wasm válido (ok), wasm inválido (err)
- [ ] **8.3.3** Exponer host functions al Wasm: `get_state(key) -> bytes`, `put_state(key, bytes)`
  - Estas funciones leen/escriben en `WorldState` (Fase 6)
  - Tests: wasm chaincode que hace put("x", "1") + get("x") → retorna "1"
- [ ] **8.3.4** Implementar memory limit: `wasmtime::StoreLimitsBuilder::memory_size(max_bytes)`
  - Tests: wasm que intenta allocar más de max → trapped

---

## Fase 9 — Fabric Gateway

> Gateway es un handler Actix-Web que orquesta el lifecycle completo: endorse → order → commit.
> Reutiliza los building blocks de Fases 1–3.

- [ ] **9.1.1** Crear struct `Gateway` en `src/gateway/mod.rs`
  - Campos: `org_registry: Arc<dyn OrgRegistry>`, `policy_store: Arc<dyn PolicyStore>`, `ordering_service: Arc<OrderingService>`, `store: Arc<dyn BlockStore>`
  - Declarar `mod gateway` en `lib.rs`/`main.rs`
  - Tests: crear gateway con mocks
- [ ] **9.1.2** Implementar `Gateway::submit(&self, tx: Transaction) -> Result<TxResult, GatewayError>`
  - Paso 1: consultar policy del chaincode → determinar qué orgs necesitan endorsar
  - Paso 2: (en esta versión single-node) generar endorsement local
  - Paso 3: enviar endorsed TX a ordering service
  - Paso 4: esperar bloque con la TX → retornar resultado
  - Tests: submit TX completo → bloque cortado → TX committed
- [ ] **9.1.3** Handler `POST /api/v1/gateway/submit` — delega a `Gateway::submit`
  - Tests de integración: request HTTP → TX procesada end-to-end

---

## Fase 10 — Service Discovery

> Fabric permite a los clientes descubrir dinámicamente qué peers endorsan qué chaincode
> y en qué channel. Sin esto, el cliente debe saber de antemano a quién pedir endorsements.

### 10.1 Discovery registry

- [ ] **10.1.1** Crear struct `PeerDescriptor` en `src/discovery/mod.rs`
  - Campos: `peer_address: String`, `org_id: String`, `role: NodeRole`, `chaincodes: Vec<String>`, `channels: Vec<String>`, `last_heartbeat: u64`
  - Declarar `mod discovery` en `lib.rs`/`main.rs`
  - Tests: crear descriptor, serializar/deserializar
- [ ] **10.1.2** Crear struct `DiscoveryService` en `src/discovery/service.rs`
  - Campos: `peers: Mutex<HashMap<String, PeerDescriptor>>`, `org_registry: Arc<dyn OrgRegistry>`, `policy_store: Arc<dyn PolicyStore>`
  - Métodos: `register_peer(&self, desc: PeerDescriptor)`, `unregister_peer(&self, address: &str)`, `heartbeat(&self, address: &str)`
  - Tests: register 3 peers, heartbeat actualiza timestamp, unregister remueve

### 10.2 Endorsement plan

- [ ] **10.2.1** Implementar `DiscoveryService::endorsement_plan(&self, chaincode_id: &str, channel_id: &str) -> Result<Vec<PeerDescriptor>, DiscoveryError>`
  - Consulta policy del chaincode, filtra peers que pertenecen a orgs requeridas y tienen el chaincode instalado
  - Retorna el conjunto mínimo de peers para satisfacer la policy
  - Tests: policy NOutOf{2, [org1, org2, org3]} con 5 peers → retorna 2 peers de 2 orgs distintas
- [ ] **10.2.2** Implementar `DiscoveryService::channel_peers(&self, channel_id: &str) -> Vec<PeerDescriptor>`
  - Filtra peers que participan en el channel dado
  - Tests: 5 peers, 3 en channel "mychannel" → retorna 3

### 10.3 REST API

- [ ] **10.3.1** Handler `GET /api/v1/discovery/endorsers?chaincode={id}&channel={id}` — retorna endorsement plan
- [ ] **10.3.2** Handler `GET /api/v1/discovery/peers?channel={id}` — retorna peers del channel
- [ ] **10.3.3** Handler `POST /api/v1/discovery/register` — un peer se registra en el discovery service (llamado al boot)

### 10.4 Integración con Gateway

- [ ] **10.4.1** En `Gateway::submit` (Fase 9.1.2), usar `DiscoveryService::endorsement_plan` para determinar a qué peers enviar la proposal
  - En vez de hardcodear el peer local, consultar discovery → enviar proposals a peers del plan
  - Tests: gateway usa discovery para encontrar endorsers, submit funciona end-to-end

---

## Fase 11 — Block Event Subscriptions

> Fabric permite a clientes suscribirse a eventos de bloques y TXs vía gRPC streams.
> Implementamos el equivalente con WebSocket (Actix-Web ya soporta WS) para push notifications.

### 11.1 Event bus

- [ ] **11.1.1** Crear struct `EventBus` en `src/events/mod.rs`
  - Usa `tokio::sync::broadcast::channel` para fan-out a múltiples suscriptores
  - Declarar `mod events` en `lib.rs`/`main.rs`
  - Tests: 3 receivers, enviar evento → los 3 lo reciben
- [ ] **11.1.2** Crear enum `BlockEvent` en `src/events/types.rs`
  - Variantes: `BlockCommitted { height: u64, tx_count: usize }`, `TransactionCommitted { tx_id: String, block_height: u64, valid: bool }`, `ChaincodeEvent { chaincode_id: String, event_name: String, payload: Vec<u8> }`
  - Tests: serde roundtrip de cada variante

### 11.2 Emisión de eventos

- [ ] **11.2.1** Añadir `event_bus: Arc<EventBus>` a `AppState`
  - En el commit path (donde el peer escribe el bloque en store), emitir `BlockCommitted` y `TransactionCommitted` por cada TX del bloque
  - Tests: escribir bloque con 3 TXs → event bus recibe 1 BlockCommitted + 3 TransactionCommitted
- [ ] **11.2.2** En `WasmExecutor` (Fase 8.3), exponer host function `set_event(name, payload)` que emite `ChaincodeEvent`
  - Tests: chaincode emite evento → EventBus lo recibe

### 11.3 WebSocket endpoint

- [ ] **11.3.1** Handler `GET /api/v1/events/blocks` — WebSocket que envía `BlockEvent` en JSON a cada suscriptor
  - Usa `actix_web::web::Payload` + `actix_ws` (Actix-Web tiene soporte WS nativo)
  - Cada conexión WS se suscribe al `broadcast::channel` del `EventBus`
  - Tests: conectar WS, escribir bloque, WS recibe evento
- [ ] **11.3.2** Soporte de filtros: el cliente envía un mensaje JSON al conectar con `{ "channel_id": "...", "chaincode_id": "..." }` para filtrar eventos
  - Si no envía filtro → recibe todo
  - Tests: filtro por channel → solo recibe eventos de ese channel

### 11.4 REST fallback (long-polling)

- [ ] **11.4.1** Handler `GET /api/v1/events/blocks?from_height={n}` — retorna bloques desde height N
  - Para clientes que no soportan WebSocket
  - El cliente hace polling periódico con el último height recibido
  - Tests: escribir 3 bloques, query from_height=2 → retorna 2 bloques

---

## Fase 12 — Hardening

### 12.1 Gossip improvements

- [ ] **12.1.1** Añadir push-gossip a `network.rs`: al recibir bloque nuevo, re-enviar `Message::NewBlock` a N peers aleatorios (ya existe parcialmente para contratos en `process_message`)
  - Tests: 3 nodos en-proceso, nodo A recibe bloque → nodo B y C lo reciben via gossip

### 12.2 Observabilidad

- [ ] **12.2.1** Añadir métricas Prometheus en `metrics.rs` para: `endorsement_validations_total`, `ordering_blocks_cut_total`, `mvcc_conflicts_total`, `event_subscriptions_active`, `discovery_peers_registered`
  - Reutilizar `prometheus` crate que ya está en Cargo.toml
  - Tests: incrementar counter, verificar valor

### 12.3 Benchmarks

- [ ] **12.3.1** Benchmark con Criterion: ordering service throughput (TXs/segundo con batch_size=100)
- [ ] **12.3.2** Benchmark: endorsement validation latency (N endorsements contra policy)
- [ ] **12.3.3** Benchmark: event bus fan-out latency (1 evento → N suscriptores)

---

## Dependencias entre fases

```
Fase 1 (Endorsement) ──┬──→ Fase 3 (TX Lifecycle) ──→ Fase 6 (World State + MVCC)
Fase 2 (Ordering)  ────┘         │                  │
                                 │                  ├──→ Fase 7 (Private Data)
Fase 4 (Channels) ←── 1 + 2     │                  └──→ Fase 8 (Chaincode)
Fase 5 (MSP) ←── 1              │
Fase 9 (Gateway) ←── 1 + 2 + 3  │
Fase 10 (Discovery) ←── 1 + 4 + 9
Fase 11 (Events) ←── 3 + 8 (ChaincodeEvent requiere WasmExecutor)
Fase 12 (Hardening) ←── parallelizable desde Fase 3
```

---

*Última revisión: 2026-04-03. Cada decisión técnica está basada en el codebase actual (469 tests).*
