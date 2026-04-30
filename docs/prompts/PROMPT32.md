Implementa P11 en rust-bc: README técnico / whitepaper corto de arquitectura SegWit/PQC.

Objetivo:
Crear un documento técnico claro, preciso y defendible que explique:

* diseño de la blockchain post-cuántica
* decisiones arquitectónicas clave
* amenazas cubiertas
* limitaciones actuales

Debe servir para:

* auditorías externas
* revisión técnica
* potenciales colaboradores o inversores

---

## 1. Ubicación

Crear archivo:

```text
README_PQC.md
```

Opcional:

```text
docs/pqc-architecture.md
```

---

## 2. Estructura del documento

### 1. Overview

Explicar:

* Blockchain en Rust con soporte post-cuántico
* Firma principal: ML-DSA-65 (FIPS 204)
* Problema: firmas grandes (~5KB)
* Solución: arquitectura SegWit adaptada a PQC

---

### 2. Arquitectura

Describir claramente:

#### Modelo de transacción

```text
TxCore      → ejecución (~47 bytes)
TxWitness   → prueba criptográfica (~5KB)
```

#### Modelo de bloque

```text
Block:
  - tx_cores
  - witnesses
  - tx_root
  - witness_root
```

#### Pipeline

```text
structure → roots → fees → signatures
```

---

### 3. Escalabilidad

Explicar cada optimización:

* Verification cache
* Parallel verification (rayon)
* Compact block propagation
* Witness pruning

Incluir números reales:

* reducción de propagación (~85–90%)
* speedup de verificación (~2.7x)
* ahorro de storage (~99%)

---

### 4. Seguridad

Describir:

* ML-DSA-65 (NIST FIPS 204)
* validación contra ACVP vectors
* signing payload completo
* replay protection:

  * chain_id
  * version
  * domain separator
* protección contra:

  * witness swapping
  * field tampering
  * signature malleability
  * replay cross-version

---

### 5. Modelo económico

* weight-based fee model
* CORE_MULTIPLIER vs WITNESS_MULTIPLIER
* ML-DSA paga más por tamaño real, no penalización artificial

---

### 6. Versionado de consenso

* BlockVersion
* activation height
* compatibilidad Legacy → SegWitPqcV1

---

### 7. Validación oficial

```rust
validate_pqc_block(...)
```

Explicar:

* uso de cache
* paralelización
* invariantes de consenso

---

### 8. Amenazas cubiertas

Tabla:

| Amenaza        | Mitigación            |
| -------------- | --------------------- |
| Quantum attack | ML-DSA                |
| Replay         | chain_id + version    |
| Witness swap   | index binding + roots |
| Tampering      | full payload signed   |
| Cache bypass   | roots validated first |

---

### 9. Limitaciones actuales

Ser honesto:

* auditoría externa pendiente
* vectores ACVP completos externos opcionales
* red P2P no completamente implementada
* fee market dinámico no implementado
* no L2 / rollups aún

---

### 10. Roadmap

* auditoría externa
* optimización de red
* mempool priority
* dynamic fee market
* posible integración con otros ecosistemas

---

## 3. Estilo

* técnico, claro, sin marketing exagerado
* usar números reales de tests/benchmarks
* evitar afirmaciones no demostradas
* incluir ejemplos simples

---

## 4. Output esperado

Documento legible que permita a un ingeniero entender:

* qué problema resuelve
* cómo lo resuelve
* qué tan sólido es

---

## 5. Quality

* ortografía correcta
* secciones claras
* coherencia con código actual
* no contradecir invariantes documentadas

---

No modificar código.
Solo documentación técnica precisa.
