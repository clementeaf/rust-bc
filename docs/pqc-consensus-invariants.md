# PQC/SegWit Consensus Invariants

## 1. Propósito

rust-bc implementa un modelo de **Segregated Witness** adaptado para firmas post-cuánticas (PQC). La separación `TxCore` / `TxWitness` permite:

- Soportar firmas ML-DSA-65 de 3309 bytes sin inflar la propagación de bloques.
- Permitir pruning de witnesses en nodos no-archival.
- Habilitar compact block propagation basada en short IDs.
- Calcular fees proporcionales al costo real (weight-based).

Este documento define qué invariantes son **reglas de consenso** (deben ser idénticas en todos los nodos) y cuáles son **optimizaciones locales** (pueden variar entre implementaciones).

---

## 2. Reglas de consenso

Las siguientes reglas son obligatorias para todo nodo que valide bloques. Romper cualquiera de ellas invalida el bloque.

1. `tx_cores.len() == witnesses.len()` — cada transacción tiene exactamente un witness.
2. `tx_root` debe coincidir con el Merkle root recomputado de `tx_cores`.
3. `witness_root` debe coincidir con el Merkle root recomputado de `witnesses`.
4. Cada `witnesses[i]` debe verificar criptográficamente contra `tx_cores[i].signing_payload()`.
5. El signing payload se deriva exclusivamente de `TxCore`:
   - `chain_id` — firmado (domain separation).
   - `nonce` — firmado (replay protection).
   - `fee` — firmado (previene fee substitution).
   - `timestamp` — firmado.
   - `kind` — firmado (incluye from, to, amount).
6. `signature_scheme` en `TxWitness` debe ser consistente con el tamaño real de la firma.
7. El fee mínimo se calcula con el weight-based fee model: `BASE_TX_FEE + weight * FEE_PER_WEIGHT_UNIT`.
8. Bloques nuevos no pueden ejecutarse sin witnesses — son necesarios para la verificación criptográfica.
9. Bloques podados (`PrunedSegWitBlock`) solo son válidos como histórico local después de `pruning_depth` confirmaciones; nunca como bloques en proceso de validación.

---

## 3. No son reglas de consenso

Las siguientes son optimizaciones locales que NO afectan la validez inter-nodo:

| Componente | Naturaleza |
|---|---|
| `VerificationCache` | Optimización de CPU local |
| `rayon` (parallel verify) | Estrategia de ejecución local |
| Compact block propagation | Optimización de transporte |
| Witness pruning local | Política de storage local |
| Mempool policy | Política de admisión local |
| Orden interno de cache (FIFO/LRU) | Implementación local |
| Benchmarks | Métricas locales |
| Formato de transporte compacto (`ShortId`) | Optimización de red |

Cualquier nodo puede implementar estas optimizaciones de forma diferente y seguir siendo compatible con la red.

---

## 4. Invariantes de seguridad

Invariantes que deben mantenerse independientemente de las optimizaciones:

- **Cache nunca puede saltarse roots** — `tx_root` y `witness_root` se validan ANTES de consultar cache.
- **Cache nunca puede saltarse fees** — fee validation se ejecuta ANTES de sig verification.
- **Cache nunca puede aceptar firmas inválidas** — solo se insertan pares verificados exitosamente.
- **Short IDs no son identidad de consenso** — son truncaciones de 8 bytes para transporte, no reemplazan hashes completos.
- **Short IDs son solo optimización de red** — después de reconstruir un bloque desde short IDs, se valida completamente.
- **Witness swapping debe fallar siempre** — la key de cache vincula `(core, witness)` como par; un witness en posición incorrecta produce un cache miss y falla verificación.
- **Field tampering debe fallar siempre** — cualquier cambio en amount/fee/nonce/chain_id/timestamp/kind altera el signing payload y el tx_root.
- **Pruning no debe borrar `witness_root`** — se conserva como commitment histórico.
- **Pruning no debe alterar `tx_root`** — los cores permanecen intactos.
- **Un `PrunedSegWitBlock` no debe validarse como bloque completo** — pasar witnesses vacíos a `validate_pqc_block` falla por length mismatch.

---

## 5. Pipeline oficial

La ruta canónica de validación es:

```rust
validate_pqc_block(block, cache, config)
```

Con el orden obligatorio:

```text
structure → roots → fees → signatures
```

### Por qué importa el orden

1. **Structure primero**: detecta bloques malformados sin gastar CPU en hashing o crypto.
2. **Roots segundo**: si el Merkle root no coincide, el bloque es inválido sin importar las firmas. Esto previene que un atacante use el cache para aceptar un bloque con datos modificados.
3. **Fees tercero**: rechaza transacciones que no cubren su costo antes de gastar tiempo en verificación criptográfica (ML-DSA-65 verify es ~100x más lento que Ed25519).
4. **Signatures último**: es la operación más costosa. El cache y el paralelismo solo aplican aquí, nunca antes.

---

## 6. Riesgos si se rompe cada invariante

| Invariante | Riesgo si se rompe |
|---|---|
| `witnesses[i]` valida `tx_cores[i]` | Witness swapping — un atacante reasigna firmas a transacciones diferentes |
| `chain_id` firmado | Replay cross-chain — una tx válida en testnet se ejecuta en mainnet |
| `fee` firmado | Fee substitution — un intermediario reduce el fee para extraer valor |
| `nonce` firmado | Replay intra-chain — la misma tx se ejecuta múltiples veces |
| `tx_root` validado antes de cache | Cache bypass — un bloque con datos modificados pasa porque las firmas originales estaban cacheadas |
| `witness_root` conservado en pruning | Pérdida de verificabilidad histórica — no se puede probar que las firmas existieron |
| Short ID usado como consenso | Colisiones explotables — un atacante fabrica un objeto con el mismo short ID pero contenido diferente |
| Bloques podados aceptados como completos | Ejecución sin verificación — se aceptan transacciones cuyas firmas nunca fueron verificadas |

---

## 7. Checklist para futuros PRs

Todo PR que toque `TxCore`, `TxWitness`, `SegWitBlock`, fees, validation, mempool, compact blocks, o pruning debe responder:

- [ ] ¿Cambia alguna regla de consenso?
- [ ] ¿Cambia el signing payload?
- [ ] ¿Cambia el cálculo de roots (Merkle tree)?
- [ ] ¿Cambia el cálculo de fees (weight model)?
- [ ] ¿Puede afectar witness swapping (binding core↔witness)?
- [ ] ¿Puede hacer que un bloque podado se acepte como completo?
- [ ] ¿Puede hacer que cache salte validaciones (roots, fees)?
- [ ] ¿Requiere migración de versión de bloque?

Si la respuesta a cualquiera es "sí", el PR requiere:
1. Actualización de este documento.
2. Nuevos tests que demuestren que la invariante se mantiene.
3. Review por el equipo de seguridad/consenso.

---

## 8. Tests relacionados

Los invariantes están verificados por tests en los siguientes módulos:

| Módulo | Cobertura |
|---|---|
| `src/transaction/segwit.rs` | Estructura, roots, signing payload, witness swap, field tampering, legacy conversion |
| `src/transaction/verification_cache.rs` | Cache miss/hit, cache invalidation, eviction, parallel verification |
| `src/transaction/compact_block.rs` | Short IDs, reconstruction, missing objects, collision safety, size reduction |
| `src/transaction/witness_pruning.rs` | Pruning depth, root preservation, pruned-as-full rejection |
| `src/transaction/weight_fee.rs` | Weight calculation, fee validation, ML-DSA vs Ed25519 cost |
| `src/transaction/pqc_validation.rs` | Unified pipeline, config flags, ordering guarantees, old validator compat |

Total: **49 tests** cubriendo todas las invariantes de consenso y seguridad documentadas en este archivo.
