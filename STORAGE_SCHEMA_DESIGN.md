# RocksDB Storage Schema Design
rust-bc Digital ID System — Week 2 Implementation

**Version:** 1.0  
**Date:** December 19, 2025  
**Status:** Design (Ready for implementation)

---

## Overview

This document defines the RocksDB column family schema for the rust-bc storage tier (WS1). It specifies:
- Column families (conceptual data groupings)
- Key-value encoding formats
- Versioning strategy
- Migration paths for future releases

---

## Column Families

### 1. **blocks** (Primary)
Stores finalized blocks in the canonical chain.

| Field | Type | Purpose |
|-------|------|---------|
| **Key Format** | `b"BLK:{block_height:u64}"` | Immutable identifier |
| **Value Format** | CBOR-encoded Block struct | Full block data |
| **Example Key** | `BLK:000000000001` | Height 1 |
| **Compaction** | Never (immutable) | Historical data |

**Block struct (CBOR):**
```rust
{
  "height": u64,
  "timestamp": u64,
  "parent_hash": [u8; 32],
  "merkle_root": [u8; 32],
  "transactions": Vec<Transaction>,
  "proposer": String,
  "signature": [u8; 64],
}
```

---

### 2. **transactions** (Secondary Index)
Maps transaction IDs to full transaction data.

| Field | Type | Purpose |
|-------|------|---------|
| **Key Format** | `b"TX:{tx_hash}"` | Unique tx identifier |
| **Value Format** | CBOR-encoded Transaction | Full tx data |
| **Example Key** | `TX:abc123def456...` | SHA256 hash |
| **Retention** | All transactions | Audit trail |

**Transaction struct (CBOR):**
```rust
{
  "id": String,
  "block_height": u64,
  "timestamp": u64,
  "input": {"did": String, "signature": [u8; 64]},
  "output": {"recipient": String, "amount": u64},
  "state": "pending|confirmed|failed",
}
```

---

### 3. **identity_records** (Core Data)
Maps DID (Decentralized Identifiers) to identity records.

| Field | Type | Purpose |
|-------|------|---------|
| **Key Format** | `b"DID:{did}"` | DID string |
| **Value Format** | CBOR-encoded Identity | Complete identity doc |
| **Example Key** | `DID:did:bc:abc123...` | W3C-compatible DID |
| **Retention** | All active identities | Core identity data |

**Identity struct (CBOR):**
```rust
{
  "did": String,
  "created_at": u64,
  "updated_at": u64,
  "public_keys": Vec<PublicKey>,
  "credentials": Vec<Credential>,
  "status": "active|revoked|suspended",
  "metadata": {
    "name": String,
    "email": String,
    "jurisdiction": String,
  }
}
```

---

### 4. **credentials** (Linked Index)
Maps credential IDs to credential data (linked from identity_records).

| Field | Type | Purpose |
|-------|------|---------|
| **Key Format** | `b"CRED:{cred_id}"` | Credential identifier |
| **Value Format** | CBOR-encoded Credential | Full credential |
| **Example Key** | `CRED:cred-uuid-1234` | UUID v4 |
| **Retention** | All issued + revoked | Audit trail |

**Credential struct (CBOR):**
```rust
{
  "id": String,
  "issuer_did": String,
  "subject_did": String,
  "type": "eid|driver_license|passport",
  "claims": {key: value},
  "issued_at": u64,
  "expires_at": u64,
  "revoked_at": Option<u64>,
  "signature": [u8; 64],
}
```

---

### 5. **metadata** (System)
Stores schema version, checkpoint hashes, and system state.

| Field | Type | Purpose |
|-------|------|---------|
| **Key Format** | `b"META:schema_version"` | System constants |
| **Value Format** | CBOR-encoded metadata | Version info |
| **Example Key** | `META:schema_version` | Current: v1 |
| **Retention** | Single record | System state |

**Metadata struct:**
```rust
{
  "schema_version": u32,  // e.g., 1
  "last_checkpoint": {
    "block_height": u64,
    "block_hash": [u8; 32],
    "timestamp": u64,
  },
  "next_migration": Option<u32>,
}
```

---

## Key Encoding Strategy

### Rationale
- Prefix-based keys (e.g., `BLK:`, `TX:`, `DID:`) enable efficient range scans
- Human-readable prefixes aid debugging
- Deterministic encoding prevents collisions

### Format Examples

#### Block Key
```
BLK:000000000001234
│    └─── Height as zero-padded u64 (12 digits)
└── "BLK:" (3 bytes)
Total: ~15 bytes
```

#### Transaction Key
```
TX:a1b2c3d4e5f6...
│  └─ SHA256 hash (64 hex chars)
└── "TX:" (3 bytes)
Total: ~67 bytes
```

#### DID Key
```
DID:did:bc:abc123def456
│   └─ Full DID string (variable length)
└── "DID:" (4 bytes)
Total: ~30-50 bytes
```

---

## Value Encoding: CBOR vs Bincode

| Aspect | CBOR | Bincode | Decision |
|--------|------|---------|----------|
| **Human-readable** | Partial | No | CBOR ✓ |
| **Space efficiency** | Good | Best | Bincode, but CBOR acceptable |
| **Speed** | Fast | Faster | Bincode, but negligible difference |
| **Debugging** | Good | Poor | CBOR ✓ |
| **Schema evolution** | Excellent | Poor | CBOR ✓ |

**Decision:** Use **CBOR** for all values to support schema evolution without breaking changes.

---

## Schema Versioning & Migrations

### Current Schema Version
**v1** (Week 2 implementation)

### Versioning Rules
1. Schema version stored in `META:schema_version` key
2. On startup, StorageLayer reads version and applies migrations if needed
3. Migrations are cumulative (v0 → v1 → v2, never skip versions)
4. Each migration includes rollback procedure

### Example: v1 → v2 Migration (Future)
Suppose Week 3 adds a new field `eidas_level` to credentials.

```rust
// src/storage/migrations.rs
pub fn migrate_v1_to_v2(db: &DB) -> Result<(), StorageError> {
    // 1. Read all credentials from v1
    let iter = db.iterator(IteratorMode::From(b"CRED:", Direction::Forward));
    
    // 2. Update each credential with new field (default value)
    for (key, old_value) in iter {
        let mut cred: Credential = cbor::from_slice(&old_value)?;
        cred.eidas_level = Some("notified".to_string()); // Default
        
        let new_value = cbor::to_vec(&cred)?;
        db.put(key, new_value)?;
    }
    
    // 3. Update metadata
    let mut meta = db.get(b"META:schema_version")?;
    meta.schema_version = 2;
    db.put(b"META:schema_version", cbor::to_vec(&meta)?)?;
    
    Ok(())
}

// Rollback
pub fn rollback_v2_to_v1(db: &DB) -> Result<(), StorageError> {
    // Inverse operation: remove `eidas_level` field
    // ... similar logic
}
```

---

## Backup & Disaster Recovery

### Checkpoint Strategy
- **Frequency:** Every 1000 blocks (or ~10 seconds)
- **Location:** `./data/checkpoints/{block_height}.tar.gz`
- **Contents:** RocksDB snapshot + metadata

### Recovery Procedure
1. Detect corruption or data loss
2. Identify last valid checkpoint
3. Restore from checkpoint
4. Replay transactions from checkpoint+1 onward

---

## Performance Characteristics

### Expected Latencies (p95)
| Operation | Target | Notes |
|-----------|--------|-------|
| Single block read | < 1ms | Cache hit likely |
| Single tx write | < 2ms | Includes fsync |
| Batch write (100 txs) | < 50ms | Atomic batch |
| Range scan (1000 items) | < 500ms | Depends on data size |

### Compression
- **Algorithm:** LZ4 (fast, reasonable compression)
- **Level:** Default (4)
- **Rationale:** Balance speed vs. disk usage

---

## Column Family Configuration

```rust
// src/storage/config.rs
pub const COLUMN_FAMILIES: &[&str] = &[
    "blocks",
    "transactions",
    "identity_records",
    "credentials",
    "metadata",
];

pub fn rocksdb_options() -> rocksdb::Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    
    // Compression
    opts.set_compression(rocksdb::Compression::Lz4);
    
    // Cache
    opts.set_block_cache(&rocksdb::Cache::new_lru_cache(256 * 1024 * 1024)); // 256MB
    
    // Write buffer
    opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB
    
    opts
}
```

---

## Testing Strategy

### Unit Tests (80+)
1. **CRUD Operations (20 tests)**
   - Create, read, update, delete for each column family
   - Verify key encoding
   - Verify value encoding (CBOR)

2. **Batch Operations (15 tests)**
   - Multi-write atomicity
   - Partial batch failure handling
   - Rollback semantics

3. **Edge Cases (20 tests)**
   - Key collisions (impossible with prefix strategy)
   - Large values (>1MB)
   - Empty column families
   - Concurrent access

4. **Schema Migration (10 tests)**
   - v1 → v2 migration logic
   - Rollback verification
   - Metadata consistency

5. **Performance (15 tests)**
   - Latency benchmarks
   - Throughput tests
   - Cache effectiveness

---

## Acceptance Criteria

- [x] Schema doc complete (this file)
- [ ] Column family definitions implemented in code
- [ ] Key/value encoding verified with round-trip tests
- [ ] Migration framework scaffolded
- [ ] Checkpoint strategy documented
- [ ] 80+ unit tests passing
- [ ] p95 latency < 2ms validated
- [ ] Code review approved

---

## References

- RocksDB documentation: https://rocksdb.org/
- CBOR spec (RFC 7049): https://tools.ietf.org/html/rfc7049
- W3C DID spec: https://www.w3.org/TR/did-core/
- eIDAS regulation: https://eur-lex.europa.eu/eli/reg/2014/910/oj

---

**Next:** Implement adapters.rs, traits.rs, errors.rs based on this schema.
