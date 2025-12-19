# rust-bc Data Flow Analysis

**Created:** 2025-12-19 Phase 1 Analysis  
**Duration:** 1.5 hours  
**Status:** ✅ Complete

---

## 1. Transaction Flow

### 1.1 User Creates Transaction via API
```
Input:  POST /api/v1/transactions
        {
          sender: "0x...",
          recipient: "0x...",
          amount: 100,
          fee: 1,
          message: "..."
        }

Processing Chain:
  ↓ api.rs (handles_http_request)
  ↓ Validate input (format, balance)
  ↓ models.rs::Transaction::new()
  ↓ transaction_validation.rs::validate()
  ↓ Check signature
  ↓ Check balance against UTXO
  ↓ Add to mempool (in-memory HashMap)
  ↓ Broadcast to peers (network.rs::broadcast_transaction)

Storage: Arc<Mutex<Vec<Transaction>>> in Blockchain state

Output: 
  {
    status: "pending",
    tx_hash: "0x...",
    mempool_position: 42
  }
```

### 1.2 Miner Creates Block from Mempool
```
Input:  POST /api/v1/mine
        { difficulty: 3 }

Processing Chain:
  ↓ api.rs (handles_http_request)
  ↓ blockchain.rs::Blockchain::mine_block()
  ↓ Select top N transactions from mempool (by fee)
  ↓ Calculate Merkle root (blockchain.rs::calculate_merkle_root)
  ↓ Block::new() with:
    - index: last_block.index + 1
    - timestamp: current_time
    - transactions: [tx1, tx2, ...]
    - previous_hash: last_block.hash
    - difficulty: configured
    - nonce: 0 (will be incremented)
  ↓ Block::mine() (PoW)
    - Loop: nonce++, hash = calculate_hash()
    - Until: hash.starts_with("000..." * difficulty)
  ↓ Validate new block (chain_validation.rs)
  ↓ Add to chain (blockchain.rs::add_block)
  ↓ Remove txs from mempool
  ↓ Update balances
  ↓ Broadcast to peers (Message::NewBlock)

Storage: 
  - In memory: blockchain.chain.push(block)
  - On disk: block_storage.rs (optional SQLite or file system)

Output:
  {
    block_hash: "0x...",
    block_index: 42,
    transactions_included: 10,
    mining_time: 2.3 seconds
  }
```

### 1.3 Block Propagation to Peers
```
Input:  Block mined locally

Processing Chain:
  ↓ network.rs::broadcast_message(Message::NewBlock(block))
  ↓ For each peer in peers HashSet:
    ↓ Connect via TCP
    ↓ Serialize block to JSON
    ↓ Send over network
    ↓ Peer receives
    ↓ Deserialize JSON → Block
    ↓ chain_validation.rs::validate_block_for_chain()
      - Check: hash valid
      - Check: previous_hash matches
      - Check: timestamp reasonable
      - Check: difficulty ok
      - Check: all tx valid
    ↓ If valid: add_block()
    ↓ If invalid: ignore + penalize peer

Output: Peer's blockchain now includes new block
```

---

## 2. Data Structures

### 2.1 Block Structure
```rust
pub struct Block {
    pub index: u64,                    // Block number (0-based)
    pub timestamp: u64,                // Unix timestamp (seconds)
    pub transactions: Vec<Transaction>, // Up to ~1MB typically
    pub previous_hash: String,         // SHA256 of previous block
    pub hash: String,                  // SHA256 of this block
    pub nonce: u64,                    // PoW counter
    pub difficulty: u8,                // Number of leading zeros
    pub merkle_root: String,           // Root of transaction tree
}
```

**Size:** ~0.5-2 MB per block (depending on tx count)  
**Storage:** Linear chain, O(n) space

### 2.2 Transaction Structure
```rust
pub struct Transaction {
    pub sender: String,                // Ed25519 public key
    pub recipient: String,             // Public key or address
    pub amount: u64,                   // Satoshis (smallest unit)
    pub fee: u64,                      // Transaction fee
    pub timestamp: u64,                // When created
    pub signature: String,             // Ed25519 signature
    pub nonce: u64,                    // Sender's tx counter
}
```

**Size:** ~0.5-1 KB per transaction  
**Validation:** Signature check + balance check

### 2.3 Account/Wallet Structure
```rust
pub struct Account {
    pub address: String,               // Derived from public key
    pub balance: u64,                  // Total coins
    pub nonce: u64,                    // Transaction counter
    pub public_key: String,            // Ed25519 public key
}
```

**Storage:** HashMap<Address, Account> in memory  
**Persistence:** Loaded at startup from blockchain

---

## 3. Storage Architecture

### 3.1 In-Memory State
```
Blockchain {
    chain: Vec<Block>              // Linear chain of all blocks
    mempool: Vec<Transaction>      // Pending transactions
    accounts: HashMap<Address, Account>  // Balances
    utxo_set: HashMap<Outpoint, Output>  // UTXO model (optional)
}
```

**Pros:** Fast access, simple
**Cons:** Single-threaded, doesn't survive restart

### 3.2 Persistence
```
Option A: File System
  blockchain_blocks/
  ├── block_0.json
  ├── block_1.json
  ├── ...
  └── block_N.json

Option B: SQLite (optional)
  blockchain.db
  ├── blocks table
  ├── transactions table
  ├── accounts table
  └── indices

Option C: Both (hybrid)
  - Files for speed
  - SQLite for queries
```

**Recovery:** On restart, load blocks from disk → rebuild state

---

## 4. Network Architecture

### 4.1 P2P Network Model
```
Bootstrap Phase:
  ↓ Connect to seed_nodes or bootstrap_nodes
  ↓ Request: Message::GetPeers
  ↓ Receive: Message::Peers(vec![...])
  ↓ Connect to peers

Steady State:
  ↓ Listen on TCP port (e.g., 8081)
  ↓ Accept connections from other nodes
  ↓ Send/receive messages (TCP, JSON serialized)
  ↓ Maintain HashSet<String> of peer addresses
```

### 4.2 Message Types
```rust
pub enum Message {
    Ping,                          // Keep-alive
    Pong,                          // Response to ping
    GetBlocks,                     // Request chain sync
    Blocks(Vec<Block>),            // Send blocks
    NewBlock(Block),               // Broadcast new block
    NewTransaction(Transaction),   // Broadcast new tx
    GetPeers,                      // Request peer list
    Peers(Vec<String>),            // Send peer addresses
    Version { ... },               // Node version + state
    // ... smart contract messages
}
```

### 4.3 Consensus Model
```
Current: Longest Chain Rule (LCR)
  ↓ Each node maintains its own chain
  ↓ When receiving a block from peer:
    - If it extends current best chain: accept
    - If it creates a longer fork: switch to fork
    - Otherwise: keep in side-chains (currently ignored)
  ↓ Network converges to longest valid chain

Forks are NOT currently handled well ❌
  - No fork detection logic
  - No reorg support
  - No deep fork resolution
```

---

## 5. API Endpoints (Current)

### 5.1 Core Blockchain
```
GET  /api/v1/health                    # Health check
GET  /api/v1/blocks                    # List all blocks
GET  /api/v1/blocks/{hash}            # Get block by hash
GET  /api/v1/blocks/index/{index}     # Get block by index
POST /api/v1/mine                      # Mine new block
GET  /api/v1/chain/verify             # Verify chain integrity
GET  /api/v1/chain/info               # Chain statistics
```

### 5.2 Transactions
```
POST /api/v1/transactions             # Create transaction
GET  /api/v1/mempool                  # View pending transactions
GET  /api/v1/mempool/stats            # Mempool statistics
```

### 5.3 Wallets
```
POST /api/v1/wallets                  # Create new wallet
GET  /api/v1/wallets/{address}/balance        # Get balance
GET  /api/v1/wallets/{address}/transactions  # Tx history
```

### 5.4 Smart Contracts
```
POST /api/v1/contracts/deploy         # Deploy contract
GET  /api/v1/contracts/{address}      # Get contract info
POST /api/v1/contracts/{address}/execute    # Call function
```

### 5.5 Staking
```
POST /api/v1/staking/stake            # Stake coins
POST /api/v1/staking/unstake          # Unstake coins
GET  /api/v1/staking/validators       # List validators
```

---

## 6. Current Limitations (Bottlenecks)

### 6.1 Architecture
- **Linear Chain Only**: One block at a time (sequential)
  - Impact: Can't process parallel blocks
  - Throughput: Limited to 1 block/period
  - Solution needed: DAG for parallelism

- **No Fork Handling**: Basic longest-chain only
  - Impact: Network partitions cause divergence
  - Risk: Different nodes have different chains
  - Solution needed: Fork resolution algorithm

- **Monolithic Storage**: Everything in memory + disk
  - Impact: Doesn't scale to billions of blocks
  - Solution needed: Sharding, partitioning

### 6.2 Cryptography
- **No Post-Quantum**: Only Ed25519
  - Risk: Future quantum computers break signatures
  - Solution needed: FALCON/ML-DSA support

### 6.3 Performance
- **Sequential Block Mining**: One miner at a time
  - Throughput: ~1 block per 10-30 seconds (depending on difficulty)
  - Solution needed: Parallel mining workers

- **Full State in Memory**: All accounts loaded at startup
  - Scalability: Limited to ~10M accounts
  - Solution needed: State trie, lazy loading

### 6.4 Network
- **Simple TCP Broadcast**: No optimization
  - Propagation: 1-5 seconds per peer
  - Solution needed: Gossip protocol, block propagation tree

---

## 7. Measurement Points

### Current Performance Baseline (Needs Testing)
```
Block Time:        ? seconds (depends on difficulty)
Throughput:        ? transactions/second
Block Size:        ? MB average
Memory Usage:      ? GB for full node
Network Latency:   ? milliseconds between peers
Consensus Finality: ? blocks until immutable
```

**TODO:** Measure these with stress tests

---

## 8. State Recovery Flow

### On Node Startup:
```
1. Check if blockchain.db exists
2. If yes:
   a. Load blocks from disk
   b. Rebuild account state by replaying transactions
   c. Validate all blocks again
   d. Connect to peers
   e. Sync any missing blocks
3. If no:
   a. Create genesis block
   b. Initialize empty state
   c. Connect to peers
   d. Download full chain
```

---

## Next Steps

This data flow analysis will be used to:
1. Identify integration points for DAG
2. Plan post-quantum crypto insertion points
3. Design identity layer hooks
4. Plan UI/API extensions

See **03_CURRENT_LIMITATIONS.md** for gap analysis.
