Implementa P7 en rust-bc: documentación de invariantes de consenso para SegWit/PQC.

Objetivo:
Documentar claramente qué partes del sistema son reglas de consenso y cuáles son optimizaciones locales, para evitar que futuros cambios rompan seguridad o compatibilidad entre nodos.

Crear documento:

`docs/pqc-consensus-invariants.md`

Contenido mínimo:

## 1. Propósito

Explicar que rust-bc usa un modelo SegWit/PQC para soportar firmas ML-DSA grandes sin comprometer ejecución, red ni storage.

## 2. Reglas de consenso

Documentar como reglas obligatorias:

* `tx_cores.len() == witnesses.len()`
* `tx_root` debe coincidir con Merkle root de `tx_cores`
* `witness_root` debe coincidir con Merkle root de `witnesses`
* cada `TxWitness[i]` debe validar exactamente `TxCore[i]`
* el signing payload debe derivarse solo de `TxCore`
* `chain_id` debe estar firmado
* `nonce` debe estar firmado
* `fee` debe estar firmado
* `timestamp` debe estar firmado
* `kind` debe estar firmado
* `signature_scheme` debe formar parte del witness validado
* fee mínimo debe calcularse por weight-based fee model
* bloques nuevos no pueden ejecutarse sin witnesses
* bloques podados solo son válidos como histórico local después de `pruning_depth`

## 3. No son reglas de consenso

Documentar explícitamente que NO son consenso:

* `VerificationCache`
* `rayon`
* compact block propagation
* witness pruning local
* mempool policy
* orden interno de cache
* estrategia FIFO/LRU
* benchmarks
* formato de transporte compacto

## 4. Invariantes de seguridad

Incluir:

* cache nunca puede saltarse roots
* cache nunca puede saltarse fees
* cache nunca puede aceptar firmas inválidas
* short IDs no son identidad de consenso
* short IDs son solo optimización de red
* witness swapping debe fallar siempre
* field tampering debe fallar siempre
* pruning no debe borrar `witness_root`
* pruning no debe alterar `tx_root`
* un `PrunedSegWitBlock` no debe validarse como bloque completo

## 5. Pipeline oficial

Documentar que la ruta oficial es:

```rust
validate_pqc_block(block, cache, config)
```

Y el orden obligatorio:

```text
structure → roots → fees → signatures
```

Explicar por qué ese orden importa.

## 6. Riesgos si se rompe cada invariante

Agregar una tabla:

| Invariante                      | Riesgo si se rompe                   |
| ------------------------------- | ------------------------------------ |
| witness[i] valida tx_core[i]    | witness swapping                     |
| chain_id firmado                | replay cross-chain                   |
| fee firmado                     | fee substitution                     |
| tx_root validado antes de cache | cache bypass                         |
| witness_root conservado         | pérdida de verificabilidad histórica |
| short_id usado como consenso    | colisiones explotables               |

## 7. Checklist para futuros PRs

Todo PR que toque:

* `TxCore`
* `TxWitness`
* `SegWitBlock`
* fees
* validation
* mempool
* compact blocks
* pruning

debe responder:

* ¿Cambia alguna regla de consenso?
* ¿Cambia el signing payload?
* ¿Cambia el cálculo de roots?
* ¿Cambia el cálculo de fees?
* ¿Puede afectar witness swapping?
* ¿Puede hacer que un bloque podado se acepte como completo?
* ¿Puede hacer que cache salte validaciones?
* ¿Requiere migración de versión de bloque?

## 8. Tests relacionados

Referenciar los módulos de tests existentes:

* `segwit.rs`
* `verification_cache.rs`
* `compact_block.rs`
* `witness_pruning.rs`
* `weight_fee.rs`
* `pqc_validation.rs`

No implementar lógica nueva.
No cambiar código de consenso.
Solo documentación técnica clara y precisa.
