# Cerulean Ledger — Plataforma DLT Institucional

## Qué es

Cerulean Ledger es una plataforma de tecnología de registro distribuido (DLT) diseñada para instituciones públicas y privadas de Chile. Permite firmar documentos digitalmente, verificar credenciales, votar en procesos de gobernanza y mantener un registro inmutable de todas las operaciones — con seguridad post-cuántica y cumplimiento regulatorio.

## Para quién

| Sector | Uso principal |
|--------|--------------|
| **Universidades** | Emisión y verificación de títulos profesionales |
| **Gobierno** | Identidad digital soberana, trazabilidad de actos administrativos |
| **Banca y finanzas** | KYC verificable, cumplimiento ISO 20022, AML |
| **Salud** | Certificados de vacunación, fichas clínicas verificables |
| **Notarías y registros** | Constitución de sociedades, poderes notariales con firma electrónica |
| **Votación** | Gobernanza on-chain con transparencia criptográfica |

## Qué problema resuelve

| Problema actual | Solución Cerulean |
|----------------|-------------------|
| Verificar un título toma 3-15 días hábiles | Verificación criptográfica en < 5ms |
| Documentos falsificables | Firma electrónica avanzada (ML-DSA-65, FIPS 204) |
| Sin trazabilidad de quién accedió qué | Audit trail inmutable (ISO 27001) |
| Vulnerabilidad a computación cuántica | Criptografía post-cuántica de fábrica |
| Sistemas cerrados sin interoperabilidad | API abierta, SDKs en TypeScript y Python |

## Arquitectura en una imagen

```
┌─────────────────────────────────────────────────────────┐
│                    Cerulean Ledger                       │
├──────────┬──────────┬──────────┬──────────┬─────────────┤
│ Identidad│Documentos│Gobernanza│Compliance│  Oráculos   │
│ Digital  │y Creden. │  Digital │ y Audit  │  de Datos   │
├──────────┴──────────┴──────────┴──────────┴─────────────┤
│              API REST (120 endpoints)                    │
├──────────┬──────────┬──────────┬──────────┬─────────────┤
│Consenso  │ Storage  │   Red    │Criptogr. │Tokenomics   │
│BFT/DPoS  │ RocksDB  │P2P + TLS│PQC FIPS  │NOTA token   │
└──────────┴──────────┴──────────┴──────────┴─────────────┘
```

## Módulos principales

### Identidad Digital
Registro de identidades soberanas (DID). Personas e instituciones obtienen una identidad digital que les permite firmar documentos con validez legal. El sistema genera internamente un identificador `did:cerulean:` — el usuario solo ve su nombre.

### Documentos y Credenciales
Emisión, almacenamiento y verificación de documentos firmados electrónicamente. Cada documento queda sellado con firma ML-DSA-65 y hash SHA-256 en la blockchain. Verificable por cualquier nodo de la red.

### Gobernanza Digital
Propuestas, votación y decisiones colectivas con transparencia criptográfica. Cada voto es inmutable, el conteo es verificable por cualquier participante, y los resultados incluyen quórum y umbral de aprobación.

### Compliance y Audit Trail
Registro inmutable de todas las operaciones del sistema. 21 checks regulatorios (Ley 21.663, ISO 20022, ERC-3643). Exportable en CSV. Compatible ISO 27001.

### Integridad de la Plataforma
Dashboard en tiempo real con el estado de los 8 servicios horizontales: seguridad, forense, cumplimiento, criptografía, almacenamiento, consenso, oráculos e inteligencia AML.

## Números clave

| Métrica | Valor |
|---------|-------|
| Endpoints HTTP | 120 |
| Tests automatizados | 1,705 |
| Escenarios de pentest | 40 (0 vulnerabilidades) |
| Módulos de stress | 10 |
| Torture tests concurrentes | 17 (~15M operaciones) |
| Latencia p95 HTTP | < 5ms |
| Checks regulatorios | 21 |
| Algoritmo de firma | ML-DSA-65 (FIPS 204) |
| Algoritmo de hash | SHA-256 / SHA3-256 (configurable) |
| Key exchange | X25519 + ML-KEM-768 (hybrid PQC) |

## Stack tecnológico

| Componente | Tecnología |
|-----------|------------|
| Backend | Rust (Actix-Web 4) |
| Storage | RocksDB (Column Families) |
| Consenso | HotStuff BFT + DPoS |
| Criptografía | pqc_crypto_module (ML-DSA-65, ML-KEM-768, SHA3-256) |
| Frontend | React + Vite + Tailwind |
| SDKs | TypeScript, Python |
| Containerización | Docker Compose |
| Observabilidad | Prometheus + Grafana |
| Load testing | k6 |

## Cumplimiento normativo

| Normativa | Estado |
|-----------|--------|
| Ley 21.663 (Ciberseguridad Chile) | Cumple |
| Ley 21.180 (Transformación Digital del Estado) | Compatible |
| ISO 27001 (Seguridad de la información) | Audit trail completo |
| ISO 20022 (Mensajería financiera) | 7 tipos de mensaje validados |
| ISO 3166 / 4217 / 8601 | Países, monedas, fechas |
| ERC-3643 (Security tokens) | Implementado con overflow protection |
| FIPS 140-3 (Criptografía) | Módulo PQC con KAT self-tests |
| FIPS 204 (ML-DSA-65) | Firma digital post-cuántica |

## Cómo empezar

```bash
# Clonar y ejecutar
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc
ACL_MODE=permissive cargo run --bin rust-bc

# Seed de datos demo
./scripts/seed-sandbox.sh http://localhost:8080

# Abrir explorer
cd block-explorer-vite && npm install && npm run dev
# → http://localhost:5173/integridad
```

## Contacto

Cerulean Ledger · DLT post-cuántica · Soberanía digital
