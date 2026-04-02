# 05: Target Compliance & Security Architecture

**Phase 1 Day 3 - Task 4**  
**Status**: Design Complete  
**Scope**: EU GDPR, eIDAS, cryptography, audit logging, data handling  
**Principles**: Privacy-by-design, security-first, regulatory compliance

---

## 1. Executive Summary: EU Regulatory Landscape

### Applicable Regulations
| Regulation | Scope | Impact | Status |
|-----------|-------|--------|--------|
| **GDPR** | Data protection, privacy | All personal data handling | MANDATORY |
| **eIDAS** | Electronic ID, signatures | Digital identity verification | TARGET |
| **NIS2** | Cybersecurity requirements | Critical infrastructure | FUTURE |
| **PSD2** | Payment services | Financial transactions | CONDITIONAL |
| **DPA 2018** | UK Data Protection Act | UK compliance if serving UK | CONDITIONAL |

**Bottom Line**: GDPR is HARD REQUIREMENT. eIDAS is opportunity for credibility.

---

## 2. GDPR Compliance Framework

### 2.1 Six Core Principles

```
1. LAWFULNESS          → Legal basis for processing
2. FAIRNESS            → Transparent to data subjects
3. TRANSPARENCY        → Clear privacy policy
4. PURPOSE LIMITATION  → Data used only for stated purpose
5. DATA MINIMIZATION   → Collect only necessary data
6. STORAGE LIMITATION  → Delete after retention period
```

### 2.2 Data Classification

**Personal Data** (GDPR scope):
```
- Name, Email, Phone
- Public Key (if linked to person)
- Biometric data (fingerprint, face ID)
- Transaction history (if identifiable to person)
- IP addresses
- Device identifiers
- Location data
```

**NOT Personal Data** (GDPR exempt):
```
- Anonymous blockchain addresses (if truly anonymous)
- Transaction hashes
- Block data (no personal info)
- Public ledger entries (if anonymized)
```

**Key Insight**: Most blockchain data = personal data if can be linked to individual

### 2.3 Legal Basis for Processing

**Our use case requires TWO bases**:

```
BASIS 1: Contract Performance
- Providing identity service
- Executing transactions
- Legal basis: Article 6(1)(b) GDPR

BASIS 2: Legitimate Interest
- Preventing fraud, money laundering
- Maintaining system security
- Legal basis: Article 6(1)(f) GDPR
  → Must balance against data subject rights

BASIS 3: Consent (OPTIONAL)
- Additional features (analytics, marketing)
- Legal basis: Article 6(1)(a) GDPR
  → Must be explicit, revocable
```

**Never rely on consent alone for core service**

---

## 3. Data Protection Technical Implementation

### 3.1 Encryption at Rest

```rust
// File: src/security/encryption.rs

pub trait IEncryptionService {
    async fn encrypt_at_rest(&self, data: &[u8], context: &str) -> Result<Encrypted, CryptoError>;
    async fn decrypt_at_rest(&self, encrypted: &Encrypted, context: &str) -> Result<Vec<u8>, CryptoError>;
}

pub struct AesGcmEncryptionService {
    // Master key stored in HSM (Hardware Security Module)
    // Context: "identity", "transaction", "credential"
}

impl IEncryptionService for AesGcmEncryptionService {
    pub async fn encrypt_at_rest(&self, data: &[u8], context: &str) -> Result<Encrypted, CryptoError> {
        // AES-256-GCM per NIST SP 800-38D
        // Generate random IV (96 bits)
        // Derive context-specific key from master key
        // Encrypt: ciphertext = AES-GCM(key, IV, data, aad=context)
        
        Ok(Encrypted {
            algorithm: "AES-256-GCM".into(),
            iv: random_iv(),
            ciphertext: todo!(),
            tag: todo!(),  // Authentication tag
        })
    }
}
```

**Key Management**:
```
Master Key:
  ├─ Storage: Hardware Security Module (HSM) or Sealed Enclave
  ├─ Rotation: Annually + on compromise
  ├─ Access: Logging + audit trail
  └─ Backup: Encrypted, geographically separated

Derived Keys (per context):
  ├─ Identity context key: KDF(master_key, "identity")
  ├─ Transaction context key: KDF(master_key, "transaction")
  └─ Credential context key: KDF(master_key, "credential")
```

### 3.2 End-to-End Encryption (E2E)

**User's private key held ONLY on their device**:

```
Frontend (Client):
  └─ Private Key: Stored in device Keychain/Keystore (OS-managed encryption)
     Only accessible with biometric auth
  
Signature Process:
  1. User triggers action (sign transaction)
  2. OS prompts: "Biometric authentication required"
  3. User authenticates (face ID / fingerprint)
  4. Private key released to memory (temporary)
  5. Signature computed locally
  6. Private key immediately destroyed from memory
  7. Signature sent to backend (only signature, never private key)

Result:
  ✓ Server NEVER sees private key
  ✓ Only user can authorize transactions
  ✓ Biometric adds security layer
```

### 3.3 Transport Security (TLS)

```
Requirement: TLS 1.3 (RFC 8446)
Cipher Suites: Only AEAD ciphers
  - TLS_AES_256_GCM_SHA384 (REQUIRED)
  - TLS_CHACHA20_POLY1305_SHA256 (OPTIONAL)
  
Certificate:
  - X.509v3, ECDSA with SHA-256
  - Must include OCSP Stapling
  - Certificate pinning in app (prevent MITM)

HSTS:
  - Strict-Transport-Security: max-age=31536000; includeSubDomains
  - HSTS Preload: https://hstspreload.org/
```

---

## 4. Audit Logging & Accountability

### 4.1 Audit Log Requirements (GDPR Article 25)

**What to log**:
```
✓ WHO: Identity of person performing action (DID)
✓ WHAT: Action taken (register, sign, issue credential)
✓ WHEN: Exact timestamp (UTC)
✓ WHERE: IP address, device info
✓ WHY: Legal basis for processing
✓ HOW: Method used (API endpoint, method signature)
✓ RESULT: Success or failure + any errors
```

**Example audit log entry**:
```json
{
  "log_id": "audit_20251219_001234",
  "timestamp": "2025-12-19T10:46:32.123Z",
  "user_did": "did:neuro:alice",
  "action": "TRANSACTION_SUBMIT",
  "resource": "tx_new123",
  "ip_address": "192.0.2.1",
  "user_agent": "NeuroID-Client/1.0",
  "method": "POST /api/v1/transactions",
  "result": "SUCCESS",
  "fields_accessed": ["inputs", "outputs", "signature"],
  "fields_modified": [],
  "legal_basis": "Contract performance (Article 6.1.b)",
  "retention_until": "2026-12-19T10:46:32Z",
  "notes": "Transaction amount: 1000000000 satoshis"
}
```

### 4.2 Audit Log Implementation

```rust
// File: src/audit/audit_logger.rs

pub trait IAuditLogger {
    async fn log_action(&self, entry: AuditLogEntry) -> Result<(), AuditError>;
    async fn query_logs(&self, criteria: AuditQuery) -> Result<Vec<AuditLogEntry>, AuditError>;
}

pub struct AuditLogger {
    storage: Arc<dyn IAuditStorage>,
    encryption: Arc<dyn IEncryptionService>,
}

impl IAuditLogger for AuditLogger {
    pub async fn log_action(&self, mut entry: AuditLogEntry) -> Result<(), AuditError> {
        // 1. Validate entry completeness
        entry.validate()?;
        
        // 2. Add immutable timestamp + ID
        entry.timestamp = now_utc();
        entry.log_id = generate_log_id();
        
        // 3. Encrypt sensitive fields
        if let Some(ref ip) = entry.ip_address {
            entry.ip_address = Some(self.encryption.hash_ip(ip).await?);
        }
        
        // 4. Store in append-only log
        self.storage.append(&entry).await?;
        
        Ok(())
    }
}
```

### 4.3 Audit Log Integrity

**Merkle chain** to prevent tampering:

```
Log Entry 1
  └─ hash: SHA-256(entry1)
     ↓ (linked)
Log Entry 2
  └─ hash: SHA-256(entry2 + previous_hash)
     ↓ (linked)
Log Entry 3
  └─ hash: SHA-256(entry3 + previous_hash)
  
If entry1 is modified:
  ├─ entry1.hash changes
  └─ All subsequent hashes become invalid
     → Tampering immediately detected
```

---

## 5. Data Retention & Deletion (Right to Be Forgotten)

### 5.1 Retention Schedule

| Data Category | Retention Period | Reason | Legal Basis |
|---|---|---|---|
| **Identity Profile** | Account lifetime + 1 year after deletion | Account recovery | Contract term |
| **Transactions** | 7 years | Tax/audit requirements | Legal obligation |
| **Audit Logs** | 3 years | Fraud investigation | Legitimate interest |
| **Credentials** | Validity period + 1 year after expiry | Verification history | Contract |
| **Biometric Data** | Not stored (deleted after use) | Security | Data minimization |
| **IP Addresses** | 30 days (hashed) | Abuse prevention | Legitimate interest |
| **Device IDs** | Account lifetime | Device management | Contract |

### 5.2 Deletion Implementation

```rust
// File: src/gdpr/deletion_service.rs

pub struct DeletionService {
    identity_db: Arc<dyn IIdentityStore>,
    transaction_db: Arc<dyn ITransactionStore>,
    audit_log: Arc<dyn IAuditLogger>,
}

impl DeletionService {
    pub async fn delete_personal_data(&self, user_did: &str, reason: DeletionReason) -> Result<(), DeletionError> {
        // 1. Verify deletion request (only user or authorized admin can delete)
        let identity = self.identity_db.get_by_did(user_did).await?
            .ok_or(DeletionError::IdentityNotFound)?;
        
        // 2. Find all data associated with identity
        let transactions = self.transaction_db.query_by_user(user_did).await?;
        let credentials = self.get_credentials_for_user(user_did).await?;
        
        // 3. Anonymize transaction history (keep for 7 years per tax law)
        for tx in transactions {
            self.anonymize_transaction(tx).await?;
        }
        
        // 4. Revoke credentials
        for cred in credentials {
            self.revoke_credential(&cred.id).await?;
        }
        
        // 5. Delete identity
        self.identity_db.delete(user_did).await?;
        
        // 6. Schedule audit logs for deletion (3 year retention)
        self.schedule_audit_log_deletion(user_did, Duration::days(365 * 3))?;
        
        // 7. Log deletion request
        self.audit_log.log_action(AuditLogEntry {
            action: "USER_DATA_DELETED",
            user_did: "system",
            resource: user_did.into(),
            reason: format!("Right to be forgotten: {:?}", reason),
            fields_modified: vec!["identity", "transactions", "credentials"],
        }).await?;
        
        Ok(())
    }
    
    async fn anonymize_transaction(&self, mut tx: Transaction) -> Result<(), DeletionError> {
        // Replace identifiable info with pseudonyms
        tx.sender = format!("anon_{}", hash(&tx.sender));
        tx.recipient = format!("anon_{}", hash(&tx.recipient));
        // Keep transaction amount + date for audit trail
        
        self.transaction_db.update(tx).await?;
        Ok(())
    }
}
```

### 5.3 Deletion Workflow

```
User requests deletion
  ↓
Verify authentication (user must prove identity)
  ↓
Generate deletion request with timestamp
  ↓
30-day grace period (user can cancel)
  ├─ Send confirmation email
  ├─ Log request
  └─ Set status = "PENDING_DELETION"
  
After 30 days:
  ↓
Execute deletion
  ├─ Anonymize transaction history (7-year log)
  ├─ Revoke all credentials
  ├─ Delete identity & personal data
  ├─ Remove from indexes
  ├─ Clear biometric data
  └─ Log completion
  
Result:
  ✓ User completely removed from active system
  ✓ Historical data anonymized
  ✓ Audit trail preserved
```

---

## 6. eIDAS Compliance (Digital ID)

### 6.1 eIDAS Regulation (EU 910/2014)

**Requirements for qualified digital signature**:
```
1. Uniquely linked to signer
2. Created using means under signer's sole control
3. Linked to signed data in such way that subsequent change is detectable
4. Must be created using certified tool
5. Must be certified by qualified Trust Service Provider

Our system:
  ✓ #1: Signatures via private key (unique to user)
  ✓ #2: Private key on user's device only
  ✓ #3: Ed25519 signatures detect tampering
  ✗ #4: Need third-party certification
  ✗ #5: Need TSP certification
```

**Path to eIDAS Compliance**:
```
Step 1: Get EU Trust Mark (2-year process)
Step 2: Use qualified timestamp authority for block timestamps
Step 3: Certify Ed25519 signing tool
Step 4: Issue qualified digital signatures
Step 5: Register as qualified TSP with ETSI

Timeline: 2-4 years + €500K-€2M
```

### 6.2 Roadmap Implementation

**Phase 1 (Current)**: Self-signed digital signatures
```
- Valid for internal transactions
- Not legally binding in EU
- Good for MVP
```

**Phase 2 (Year 2)**: Advanced electronic signatures
```
- Certified timestamp server
- Regulatory approval from EU authorities
- Valid for legal contracts
```

**Phase 3 (Year 3+)**: Qualified electronic signatures
```
- Full eIDAS compliance
- Recognized across all EU member states
- Legally binding evidence in court
```

---

## 7. Data Residency & Sovereignty

### 7.1 GDPR Article 44+ (Data Transfers)

**Rule**: Personal data cannot leave EU without safeguards

**Solution**: Data Localization

```
Frontend (Client):
  ├─ Runs on user device (no data transfer)
  ├─ Biometric data: Never leaves device
  ├─ Private keys: Never leaves device
  └─ Only signatures leave device

Backend (Server):
  ├─ Location: EU data center (Germany/France recommended)
  ├─ Jurisdiction: EU law (GDPR applies)
  ├─ Backups: Same region
  ├─ Log files: EU only
  └─ No third-country transfers

If using cloud (AWS, Azure, Google):
  ├─ MUST use EU-only regions
  ├─ MUST have Data Processing Agreement (DPA)
  ├─ MUST enable encryption at rest
  └─ MUST comply with schrems II ruling
```

### 7.2 Schrems II Compliance (GDPR + US Surveillance)

**Problem**: US government can compel tech companies to hand over data

**Solution**: Contractual safeguards

```
Data Processing Agreement (DPA) must include:

1. Standard Contractual Clauses (SCCs)
   - EU Commission-approved contracts
   - Impose strict obligations on processor
   - Provide recourse if breached

2. Supplementary Measures
   - Encryption: Only EU has decryption keys
   - Access logging: Track all data access
   - Audit rights: Regular third-party audits
   - Separation: Different data silos per user

3. Legal Redress
   - Customer can escalate if US government requests data
   - Processor must exhaust legal remedies
   - Customer can enforce contract terms
```

---

## 8. Privacy by Design (GDPR Article 25)

### 8.1 Technical & Organizational Measures (TOMs)

```
DATA MINIMIZATION:
  ✓ Only collect: DID, email, public key, age verification
  ✗ Don't collect: Browsing history, location tracking
  
PSEUDONYMIZATION:
  ✓ Store user DID instead of name
  ✗ Link can be broken if needed (for support)
  
ENCRYPTION:
  ✓ All data at rest: AES-256-GCM
  ✓ All data in transit: TLS 1.3
  ✗ Never store passwords (auth via signatures)
  
ACCESS CONTROL:
  ✓ Role-based access (RBAC)
  ✓ Principle of least privilege
  ✓ Audit all admin access
  ✗ No backdoors or master keys accessible to staff

ANONYMIZATION:
  ✓ For aggregated analytics: Strip identifiers
  ✓ Use differential privacy for statistics
  ✗ Never assume "anonymized" unless irreversible
```

### 8.2 Privacy Impact Assessment (DPIA)

**Required for high-risk processing** (GDPR Article 35):

```
Template:

1. PROCESSING ACTIVITY
   - Identity registration and verification
   - Purpose: Provide digital ID service
   - Legal basis: Contract + Legitimate interest

2. NECESSITY & PROPORTIONALITY
   - Why collect DID? → Unique identifier
   - Why collect email? → Communication channel
   - Why collect public key? → Signature verification
   - Alternative approaches? → None viable

3. RISKS TO DATA SUBJECTS
   - Unauthorized access → Mitigation: Encryption + RBAC
   - Data breach → Mitigation: Incident response plan
   - Identity theft → Mitigation: Biometric auth
   - Profiling → Mitigation: No tracking, analytics separated

4. SAFEGUARDS IMPLEMENTED
   - Technical: Encryption, TLS, HSM
   - Organizational: Access logging, staff training
   - Legal: DPA with processors, retention policy
   - Operational: Incident response, security updates

5. OUTCOME
   - Risk level: MEDIUM → Acceptable with safeguards
   - Approval: Legal + Privacy team
   - Review: Annually + on material changes
```

---

## 9. Compliance Monitoring & Audit

### 9.1 Self-Assessment Checklist

```
✅ Do we have a Privacy Policy?
✅ Do we have a DPA with processors?
✅ Do we have a Data Retention Policy?
✅ Do we conduct DPIAs?
✅ Do we encrypt data at rest + transit?
✅ Do we have incident response plan?
✅ Do we have user consent for optional features?
✅ Do we log data access (audit trail)?
✅ Do we handle deletion requests?
✅ Do we have staff training on GDPR?
```

### 9.2 Annual Compliance Audit

```
Conducted by: External auditor (Big 4 accounting firm)
Frequency: Annual
Cost: €50K-€150K per year
Scope:
  - Technical controls (encryption, access logs)
  - Organizational policies (procedures, training)
  - Legal agreements (DPA, consent forms)
  - Incident response (test scenarios)

Output: Compliance report + certification
```

---

## 10. Incident Response Plan

### 10.1 Data Breach Notification (GDPR Article 33-34)

```
TIMELINE:

T+0 (Discovery): Breach detected
  └─ Activate incident response team

T+0-24h: Immediate Actions
  ├─ Contain breach (stop attacker access)
  ├─ Preserve evidence (for investigation)
  ├─ Assess scope (how much data affected?)
  └─ Activate response playbook

T+24-72h: Notification to Authority
  ├─ Report to local data protection authority
  ├─ Include: What, When, Who affected, Measures taken
  ├─ Note: Must notify even if no customer impact
  └─ Authority decides if public notification needed

T+72h-30 days: Notify Affected Users
  ├─ Send email explaining breach
  ├─ Advise what data was affected
  ├─ Recommend actions (change password, monitor account)
  ├─ Contact phone number for questions
  └─ Offer credit monitoring if sensitive data

T+30-90 days: Root Cause Analysis
  ├─ Hire forensics firm
  ├─ Investigate attacker method
  ├─ Identify system weaknesses
  └─ Implement fixes + controls

Result:
  - Legal exposure: Fines up to 4% of global revenue (€10M-€400M+)
  - Reputation damage: Could be existential
  - Mitigation: Transparency + swift action
```

### 10.2 Response Playbook (Mock Breach)

```
Scenario: 1000 customer records leaked on dark web

T+0: Security team alerts management
  - "Alert: 1000+ records detected on 4chan"
  - Activate incident response (CEO, CTO, Legal, Privacy Officer)

T+1h: Investigation starts
  - Forensics: Check server logs, access patterns
  - Database: Verify actual data exposed (hash check)
  - Scope: 1000 records = 0.1% of users (not massive)
  - Impact: Email + public key exposed (not passwords, no financial data)

T+24h: Decision point
  - Risk assessment: LOW (public keys are meant to be public)
  - Regulatory analysis: Must still notify (conservative approach)
  - Decision: Notify users + authorities

T+72h: Notification sent
  - 1000 emails sent: "Your email was included in a data leak"
  - Authority notified: Email proof attached
  - Media outreach: Proactive statement on website

T+30d: Root cause found
  - AWS S3 bucket was publicly accessible (misconfiguration)
  - Fix: Add bucket encryption + access logging
  - Process change: Automatic security scanner in CI/CD
  - Staff training: Cloud security best practices

Result:
  ✓ Users informed
  ✓ Authorities satisfied
  ✓ Future breaches prevented
  ✓ Trust maintained (transparency = credibility)
```

---

## 11. Summary: Compliance Roadmap

| Phase | Timeline | Milestones | Cost |
|---|---|---|---|
| **Phase 1** | Months 1-3 | GDPR baseline, encryption, audit logs | €100K |
| **Phase 2** | Months 4-9 | eIDAS roadmap, DPA finalized, DPIA completed | €150K |
| **Phase 3** | Months 10-24 | Qualified timestamp authority, advanced signatures | €500K |
| **Phase 4** | Months 24+ | Full eIDAS compliance, TSP certification | €2M |

**Minimum for production**: Phase 1 (GDPR-compliant)
**Recommended for scale**: Phase 2 (eIDAS-ready)
**Enterprise grade**: Phase 3-4 (Qualified signatures)

---

**End of Compliance & Security Architecture**

*Next: Task 5 - Consolidate into Decision Matrix Update*
