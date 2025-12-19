# 05: Target Backend Architecture

**Phase 1 Day 3 - Task 1**  
**Status**: Design Complete  
**Scope**: rust-bc evolution with critical separation of concerns  
**Principles**: Clean architecture, microservices boundaries, zero coupling

---

## 1. Architecture Philosophy: Layered Tiers with Clear Boundaries

### Design Principle
**Each tier has ONE primary responsibility** → Zero responsibility leakage → Easy to test, deploy, scale independently

```
┌─────────────────────────────────────────────────┐
│         API Tier (Public Interface)             │ ← REST/gRPC Gateway
├─────────────────────────────────────────────────┤
│  Identity Tier (User/Entity Management)         │ ← Auth, Credentials, KYC
├─────────────────────────────────────────────────┤
│  Consensus Tier (Distributed Ledger)            │ ← rust-bc Core Evolution
├─────────────────────────────────────────────────┤
│  Storage Tier (Persistent Data)                 │ ← RocksDB, Ledger Logs
└─────────────────────────────────────────────────┘
  ↓ Unidirectional Dependency Flow (downward only)
```

### Coupling Rules (CRITICAL)
- API Tier: Can depend on Identity + Consensus
- Identity Tier: Can depend on Consensus + Storage ONLY
- Consensus Tier: Can depend on Storage ONLY
- Storage Tier: Has ZERO external dependencies

**Violation = Architecture debt**

---

## 2. Tier 1: Storage Layer (Foundation)

**Responsibility**: Persistent data, durability guarantees, immutability

### Components

#### 2.1 Block Store
```rust
// File: src/storage/block_store.rs

pub trait IBlockStore: Send + Sync {
    async fn append_block(&self, block: &Block) -> Result<BlockId, StorageError>;
    async fn get_block(&self, id: BlockId) -> Result<Option<Block>, StorageError>;
    async fn get_block_range(&self, start: u64, end: u64) -> Result<Vec<Block>, StorageError>;
    async fn block_exists(&self, id: BlockId) -> Result<bool, StorageError>;
}

// Implementation: RocksDB-backed
pub struct RocksDbBlockStore {
    db: Arc<rocksdb::DB>,
    // Separation: Only DB logic, no consensus concerns
}
```

**Responsibilities**:
- Append-only block log
- O(1) block lookup by ID
- Batch queries (no consensus filtering)
- Compression + checksums

**NOT Responsible For**:
- Validation (Consensus Tier)
- Identity checks (Identity Tier)
- API serialization (API Tier)

#### 2.2 Transaction Log
```rust
pub trait ITransactionLog: Send + Sync {
    async fn record_tx(&self, tx: &Transaction) -> Result<TxId, StorageError>;
    async fn get_tx(&self, id: TxId) -> Result<Option<Transaction>, StorageError>;
    async fn list_mempool(&self) -> Result<Vec<Transaction>, StorageError>;
}

pub struct TransactionLog {
    mempool_db: Arc<rocksdb::DB>,
    ledger_db: Arc<rocksdb::DB>, // Separate: confirmed vs. pending
}
```

**Separation**: Mempool ≠ Confirmed ledger (different DB instances)

#### 2.3 Merkle Tree Proofs
```rust
pub trait IMerkleProofStore: Send + Sync {
    async fn store_proof(&self, block_id: BlockId, proof: &MerkleProof) -> Result<(), StorageError>;
    async fn get_proof(&self, block_id: BlockId, tx_index: usize) -> Result<MerkleProof, StorageError>;
}

pub struct MerkleProofStore {
    db: Arc<rocksdb::DB>,
    // Stores intermediate nodes + leaf proofs
}
```

**Key Insight**: Proofs are deterministic from block data → Can be regenerated → Not critical path

---

## 3. Tier 2: Consensus Layer (Evolution)

**Responsibility**: Distributed consensus, block validity, chain security

### Current rust-bc State → Target State

| Aspect | Current | Target | Delta |
|--------|---------|--------|-------|
| **Ledger** | Linear chain | DAG-compatible | Add vertex/edge abstraction |
| **Mining** | Sequential | Parallel slots | Add slot-based mining |
| **Consensus** | Longest chain | Weighted DAG | Add weight aggregation |
| **Finality** | Probabilistic | Probabilistic + soft finality | Add finality votes |
| **Fork Handling** | None | Automatic reorg | Add fork resolution |

### 3.1 Block Validation Pipeline

```rust
// File: src/consensus/validation.rs

pub struct ValidationPipeline {
    pow_checker: ProofOfWorkChecker,
    prev_hash_checker: PreviousHashChecker,
    merkle_checker: MerkleRootChecker,
    double_spend_checker: DoubleSpendChecker,
}

impl ValidationPipeline {
    pub async fn validate(&self, block: &Block) -> Result<ValidationReport, ValidationError> {
        // ORDERED: Fail fast, expensive checks last
        
        // 1. Syntax check (cheap, 0.1ms)
        self.pow_checker.validate_syntax(block)?;
        
        // 2. PoW check (medium, ~10ms)
        self.pow_checker.verify_difficulty(block)?;
        
        // 3. Merkle check (medium, ~5ms)
        self.merkle_checker.verify_root(block)?;
        
        // 4. Double-spend check (expensive, ~50ms, uses bloom filter)
        self.double_spend_checker.check(block)?;
        
        // 5. Consensus rules (complex, custom logic)
        self.consensus_checker.apply_rules(block)?;
        
        Ok(ValidationReport {
            is_valid: true,
            elapsed_ms: elapsed,
        })
    }
}

// CRITICAL: Each checker is testable in isolation
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_pow_validation() { /* unit test */ }
    
    #[tokio::test]
    async fn test_merkle_validation() { /* unit test */ }
}
```

**Separation Principle**: Each validator = separate struct, separate test

### 3.2 DAG Evolution (From Linear Chain)

```rust
// File: src/consensus/dag.rs

pub struct DAGVertex {
    pub block_id: BlockId,
    pub parents: Vec<BlockId>,      // Multiple parents = DAG
    pub timestamp: Timestamp,
    pub miner_weight: u32,
}

pub struct DAGEdge {
    pub from: BlockId,
    pub to: BlockId,
    pub weight: f64,  // Probability of causal order
}

pub struct DAGConsensus {
    vertices: Arc<RwLock<HashMap<BlockId, DAGVertex>>>,
    edges: Arc<RwLock<Vec<DAGEdge>>>,
    storage: Arc<dyn IBlockStore>,
}

impl DAGConsensus {
    pub async fn add_block(&self, block: &Block) -> Result<ConsensusState, ConsensusError> {
        let vertex = DAGVertex::from_block(block);
        
        // Determine parent blocks (tip selection algorithm)
        let parents = self.select_parents().await?;
        let vertex = vertex.with_parents(parents);
        
        // Add to DAG
        self.vertices.write().await.insert(block.id, vertex.clone());
        
        // Compute consensus order (SPECRE algorithm or similar)
        let order = self.compute_order(&vertex).await?;
        
        Ok(ConsensusState {
            order,
            finality_confidence: 0.95,
        })
    }
    
    async fn compute_order(&self, vertex: &DAGVertex) -> Result<BlockOrder, ConsensusError> {
        // Complexity: O(V log V) where V = visible vertices
        // Separation: Pure algorithm, no I/O
        todo!("SPECRE or similar")
    }
}
```

**Key**: Linear chain is special case of DAG (all vertices have 1 parent)

### 3.3 Slot-Based Mining (Parallel)

```rust
// File: src/consensus/mining.rs

pub struct SlotScheduler {
    slot_duration_ms: u64,
    validators_per_slot: usize,
    current_slot: Arc<AtomicU64>,
}

pub struct MiningWorker {
    slot: u64,
    difficulty: u32,
    parent_hash: Hash,
    tx_selection: Box<dyn TxSelector>,
}

pub async fn mine_slot(worker: MiningWorker) -> Result<ProposedBlock, MiningError> {
    // Parallel: Multiple workers mine different slots
    // Separation: Mining logic ≠ slot assignment logic
    
    let mut nonce = 0u64;
    let target = difficulty_to_target(worker.difficulty);
    
    loop {
        let candidate = ProposedBlock {
            parent_hash: worker.parent_hash,
            transactions: worker.tx_selection.select()?,
            nonce,
            slot: worker.slot,
        };
        
        let hash = candidate.compute_hash();
        if hash < target {
            return Ok(candidate);
        }
        
        nonce += 1;
        
        // Yield to prevent blocking
        tokio::task::yield_now().await;
    }
}
```

**Parallelization**: Spawn N workers for N slots, coordinate via SlotScheduler

---

## 4. Tier 3: Identity Layer (New)

**Responsibility**: User authentication, entity management, credential issuance

### Design Principle
**Identity is NOT part of blockchain consensus** → Identity tier is independent → Can evolve without blockchain changes

### 4.1 Identity Model

```rust
// File: src/identity/models.rs

pub struct DigitalIdentity {
    pub id: IdentityId,                    // UUID, immutable
    pub did: String,                       // W3C-compliant: "did:neuro:..."
    pub public_key: PublicKey,             // Ed25519 + post-quantum backup
    pub credentials: Vec<Credential>,      // Verifiable credentials
    pub status: IdentityStatus,            // Active, Revoked, Suspended
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

pub struct Credential {
    pub id: CredentialId,
    pub issuer: IdentityId,
    pub subject: IdentityId,
    pub claims: HashMap<String, serde_json::Value>,  // Custom claims
    pub proof: CredentialProof,                      // JWS signature
    pub issued_at: Timestamp,
    pub expires_at: Option<Timestamp>,
}

pub struct CredentialProof {
    pub algorithm: String,             // "JWS", "Ed25519"
    pub signature: Bytes,
    pub key_id: String,
}
```

### 4.2 Identity Verification Pipeline

```rust
// File: src/identity/verification.rs

pub struct IdentityVerifier {
    storage: Arc<dyn IIdentityStore>,
    signature_checker: SignatureChecker,
    revocation_checker: RevocationChecker,
}

impl IdentityVerifier {
    pub async fn verify_identity(&self, did: &str, challenge: &[u8]) -> Result<IdentityProof, VerificationError> {
        // 1. Lookup identity
        let identity = self.storage.get_by_did(did).await?
            .ok_or(VerificationError::NotFound)?;
        
        // 2. Check revocation status
        if identity.status == IdentityStatus::Revoked {
            return Err(VerificationError::Revoked);
        }
        
        // 3. Verify signature
        let proof_bytes = self.signature_checker.verify(challenge, &identity.public_key)?;
        
        Ok(IdentityProof {
            identity_id: identity.id,
            verified_at: now(),
            proof_bytes,
        })
    }
}
```

### 4.3 Credential Issuance (Role-Based)

```rust
// File: src/identity/issuance.rs

pub struct CredentialIssuer {
    issuer_identity: DigitalIdentity,
    storage: Arc<dyn IIdentityStore>,
}

impl CredentialIssuer {
    pub async fn issue_credential(
        &self,
        subject_did: &str,
        claims: HashMap<String, Value>,
        expiry_days: Option<u32>,
    ) -> Result<Credential, IssuanceError> {
        // 1. Validate subject exists
        let subject = self.storage.get_by_did(subject_did).await?
            .ok_or(IssuanceError::SubjectNotFound)?;
        
        // 2. Create credential
        let credential = Credential {
            id: CredentialId::new(),
            issuer: self.issuer_identity.id.clone(),
            subject: subject.id.clone(),
            claims,
            issued_at: now(),
            expires_at: expiry_days.map(|d| now() + Duration::days(d as i64)),
            proof: self.sign_credential(&credential).await?,
        };
        
        // 3. Persist
        self.storage.save_credential(&credential).await?;
        
        Ok(credential)
    }
}
```

**Key Separation**: 
- Issuance = separate concern from verification
- Both testable independently
- Can support multiple issuers

---

## 5. Tier 4: API Layer (Interface)

**Responsibility**: REST/gRPC gateway, serialization, rate limiting, request validation

### 5.1 REST API Routes

```rust
// File: src/api/routes.rs

pub struct ApiGateway {
    consensus: Arc<dyn IConsensus>,
    identity: Arc<dyn IIdentityService>,
    blocks: Arc<dyn IBlockStore>,
}

#[async_trait]
pub trait ApiRoutes {
    // Blockchain queries
    async fn get_block(&self, id: BlockId) -> Result<BlockResponse, ApiError>;
    async fn get_latest_blocks(&self, limit: u32) -> Result<Vec<BlockResponse>, ApiError>;
    async fn submit_transaction(&self, tx: TransactionRequest) -> Result<TxId, ApiError>;
    async fn get_transaction(&self, id: TxId) -> Result<TransactionResponse, ApiError>;
    
    // Identity operations
    async fn register_identity(&self, req: IdentityRegistration) -> Result<DIDResponse, ApiError>;
    async fn verify_identity(&self, did: &str, challenge: &[u8]) -> Result<IdentityProof, ApiError>;
    async fn issue_credential(&self, req: CredentialRequest) -> Result<Credential, ApiError>;
    
    // Consensus state
    async fn get_chain_state(&self) -> Result<ChainState, ApiError>;
    async fn get_consensus_info(&self) -> Result<ConsensusInfo, ApiError>;
}

impl ApiRoutes for ApiGateway {
    async fn submit_transaction(&self, tx: TransactionRequest) -> Result<TxId, ApiError> {
        // Validation
        tx.validate()?;
        
        // Deserialize (single responsibility: API deserialization)
        let transaction = tx.into_domain_tx()?;
        
        // Delegate to consensus
        let tx_id = self.consensus.add_transaction(transaction).await?;
        
        Ok(tx_id)
    }
}
```

### 5.2 Request/Response Models (DTO Pattern)

```rust
// File: src/api/dto.rs

// REQUEST: From client
pub struct TransactionRequest {
    pub inputs: Vec<InputRequest>,
    pub outputs: Vec<OutputRequest>,
    pub signature: String,  // Hex-encoded
}

impl TransactionRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.inputs.is_empty() { return Err(ApiError::NoInputs); }
        if self.outputs.is_empty() { return Err(ApiError::NoOutputs); }
        if self.signature.is_empty() { return Err(ApiError::MissingSignature); }
        Ok(())
    }
    
    pub fn into_domain_tx(self) -> Result<Transaction, ApiError> {
        // Convert DTO → Domain model
        Transaction {
            inputs: self.inputs.into_iter().map(|i| i.into()).collect(),
            outputs: self.outputs.into_iter().map(|o| o.into()).collect(),
            signature: Signature::from_hex(&self.signature)?,
        }
    }
}

// RESPONSE: To client
pub struct TransactionResponse {
    pub id: String,
    pub status: String,
    pub created_at: String,
}
```

**Separation**: DTO ≠ Domain Model
- DTOs: API serialization only
- Domain: Business logic only
- Never mix concerns

### 5.3 Error Handling (Centralized)

```rust
// File: src/api/error.rs

#[derive(Debug)]
pub enum ApiError {
    ValidationError(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl From<ConsensusError> for ApiError {
    fn from(err: ConsensusError) -> Self {
        match err {
            ConsensusError::InvalidBlock => ApiError::ValidationError("Invalid block".into()),
            ConsensusError::ChainFork => ApiError::Conflict("Chain fork detected".into()),
            ConsensusError::Storage(e) => ApiError::Internal(format!("Storage error: {}", e)),
        }
    }
}

impl ApiError {
    pub fn http_status(&self) -> u16 {
        match self {
            ApiError::ValidationError(_) => 400,
            ApiError::NotFound(_) => 404,
            ApiError::Conflict(_) => 409,
            ApiError::Internal(_) => 500,
        }
    }
}
```

**Key**: Single source of truth for error mapping (API ↔ Domain)

---

## 6. Data Flow: Request → Response (Clean Path)

### Example: Submit Transaction

```
Client Request
    ↓
API Gateway (src/api/routes.rs)
    ├─ Deserialize: TransactionRequest → JSON parsing
    ├─ Validate: Check schema + business rules
    ├─ Convert: DTO → Domain model (Transaction)
    ↓
Consensus Tier (src/consensus/)
    ├─ Validate syntax: Check signatures, inputs/outputs
    ├─ Check double-spend: Bloom filter + UTXO set
    ├─ Add to mempool: Store for future blocks
    ↓
Storage Tier (src/storage/)
    ├─ Write TX log: Durable mempool entry
    ↓
Response Path (REVERSE)
    ├─ Create TxId
    ├─ Convert: Domain model → TransactionResponse (DTO)
    ├─ Serialize: TransactionResponse → JSON
    ↓
Client Response (TxId + status)
```

**No leakage between tiers** → Each tier tests independently

---

## 7. Testing Strategy: Pyramid

```
         ┌─────────────────┐
         │  Integration    │ (5% tests, 20% effort)
         │  Tests          │ End-to-end API flows
         ├─────────────────┤
         │  Service Tests  │ (20% tests, 40% effort)
         │  (Mid-level)    │ Consensus + Identity services
         ├─────────────────┤
         │  Unit Tests     │ (75% tests, 40% effort)
         │  (Fast)         │ Storage, Validators, DTOs
         └─────────────────┘

Target: 80%+ code coverage, < 10% flaky tests
```

### 7.1 Unit Tests (Storage Layer)

```rust
#[cfg(test)]
mod block_store_tests {
    #[tokio::test]
    async fn test_append_retrieve_block() {
        let store = RocksDbBlockStore::new_temp();
        let block = create_test_block();
        
        store.append_block(&block).await.unwrap();
        let retrieved = store.get_block(block.id).await.unwrap();
        
        assert_eq!(retrieved.unwrap(), block);
    }
}
```

### 7.2 Service Tests (Consensus Layer)

```rust
#[cfg(test)]
mod consensus_tests {
    #[tokio::test]
    async fn test_dag_consensus_fork_resolution() {
        let consensus = DAGConsensus::new_test();
        
        // Create fork
        let block_a = create_test_block();
        let block_b = create_test_block(); // Same parent as block_a
        
        consensus.add_block(&block_a).await.unwrap();
        consensus.add_block(&block_b).await.unwrap();
        
        // Both should be in DAG
        assert!(consensus.has_block(&block_a.id).await.unwrap());
        assert!(consensus.has_block(&block_b.id).await.unwrap());
    }
}
```

### 7.3 Integration Tests (API Gateway)

```rust
#[cfg(test)]
mod api_integration_tests {
    #[tokio::test]
    async fn test_submit_transaction_end_to_end() {
        let api = ApiGateway::new_test();
        let req = TransactionRequest::test_valid();
        
        let tx_id = api.submit_transaction(req).await.unwrap();
        
        let tx = api.get_transaction(tx_id).await.unwrap();
        assert_eq!(tx.status, "Pending");
    }
}
```

---

## 8. Dependency Injection (IoC)

```rust
// File: src/services.rs

pub struct Services {
    pub block_store: Arc<dyn IBlockStore>,
    pub consensus: Arc<dyn IConsensus>,
    pub identity: Arc<dyn IIdentityService>,
    pub api: Arc<dyn ApiRoutes>,
}

impl Services {
    pub fn new(config: &Config) -> Result<Self, ServiceError> {
        // Build dependency tree (bottom-up)
        let block_store = Arc::new(RocksDbBlockStore::new(&config.db_path)?);
        let consensus = Arc::new(DAGConsensus::new(block_store.clone()));
        let identity = Arc::new(IdentityService::new(block_store.clone()));
        let api = Arc::new(ApiGateway::new(
            consensus.clone(),
            identity.clone(),
            block_store.clone(),
        ));
        
        Ok(Services {
            block_store,
            consensus,
            identity,
            api,
        })
    }
}
```

---

## 9. Operational Concerns

### 9.1 Monitoring & Observability

```rust
// File: src/metrics.rs

pub struct Metrics {
    block_height: Gauge,
    consensus_latency_ms: Histogram,
    validation_errors: Counter,
    identity_registrations: Counter,
}

impl Metrics {
    pub fn record_block_added(&self, height: u64, latency_ms: u64) {
        self.block_height.set(height as f64);
        self.consensus_latency_ms.observe(latency_ms as f64);
    }
}
```

### 9.2 Graceful Shutdown

```rust
pub async fn shutdown(services: Services) {
    // Order matters: Stop accepting requests first
    services.api.stop().await;
    
    // Let in-flight requests complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Flush consensus state
    services.consensus.flush().await;
    
    // Close storage
    services.block_store.close().await;
}
```

---

## 10. Summary: Tier Responsibilities

| Tier | Responsibility | Dependencies | Testability |
|------|---|---|---|
| **API** | REST/gRPC gateway, serialization | Identity + Consensus | HTTP client mocks |
| **Identity** | Auth, credentials, DID | Consensus + Storage | In-memory identity store |
| **Consensus** | DAG, validation, mining | Storage only | Mock storage |
| **Storage** | Persistent data | None | Temp RocksDB instances |

**Principle**: Each tier can be tested with mocks of its dependencies → Zero integration complexity

---

**End of Backend Architecture Design**

*Next: Task 2 - Frontend Architecture (MVVM, client-tier separation)*
