Implementa P3 en rust-bc: compact block propagation para bloques SegWit/PQC.

Contexto:
Ya existe:

* `TxCore`
* `TxWitness`
* `compute_tx_root()`
* `compute_witness_root()`
* `validate_segwit_block()`
* `VerificationCache`
* `validate_segwit_block_with_cache()`
* `validate_segwit_block_parallel()`

Objetivo:
Reducir propagación de red evitando reenviar transacciones y witnesses completos cuando el peer ya los tiene en mempool.

---

## 1. Crear modelo CompactBlock

Ubicación sugerida:
`src/block/compact.rs` o `src/network/compact_block.rs`

Estructuras mínimas:

```rust
pub struct CompactBlock {
    pub header: BlockHeader,
    pub tx_core_short_ids: Vec<ShortId>,
    pub witness_short_ids: Vec<ShortId>,
}

pub struct MissingCompactRequest {
    pub block_hash: Hash,
    pub missing_tx_core_ids: Vec<ShortId>,
    pub missing_witness_ids: Vec<ShortId>,
}

pub struct MissingCompactResponse {
    pub block_hash: Hash,
    pub tx_cores: Vec<TxCore>,
    pub witnesses: Vec<TxWitness>,
}
```

`ShortId` puede ser inicialmente:

```rust
pub type ShortId = [u8; 8];
```

---

## 2. Short ID determinístico

Implementar:

```rust
short_id_tx_core(core: &TxCore) -> ShortId
short_id_witness(witness: &TxWitness) -> ShortId
```

Regla:

* usar hash canónico del objeto serializado
* tomar primeros 8 bytes
* debe ser determinístico entre nodos

Ideal:

```text
ShortId = first_8_bytes(SHA3-256(serialized_object))
```

No usar índices como ID.

---

## 3. Conversión full block → compact block

Implementar:

```rust
CompactBlock::from_segwit_block(block: &SegWitBlock) -> Self
```

Debe:

* copiar header
* convertir cada `TxCore` a `ShortId`
* convertir cada `TxWitness` a `ShortId`
* mantener el orden

---

## 4. Reconstrucción desde mempool

Crear función:

```rust
reconstruct_compact_block(
    compact: &CompactBlock,
    mempool: &SegWitMempool,
) -> Result<SegWitBlock, MissingCompactRequest>
```

Debe:

* buscar cada `tx_core_short_id` en mempool
* buscar cada `witness_short_id` en mempool
* si todo existe, reconstruir bloque completo
* si falta algo, devolver `MissingCompactRequest`

---

## 5. Respuesta de faltantes

Implementar:

```rust
apply_missing_response(
    compact: &CompactBlock,
    partial: PartialSegWitBlock,
    response: MissingCompactResponse,
) -> Result<SegWitBlock, CompactBlockError>
```

Debe:

* insertar tx_cores/witnesses faltantes en la posición correcta
* rechazar objetos cuyo short_id no coincida
* rechazar si response trae objetos extra o irrelevantes
* reconstruir bloque completo solo si queda completo

---

## 6. Seguridad

La reconstrucción compacta NO reemplaza la validación.

Después de reconstruir:

* validar `tx_root`
* validar `witness_root`
* validar length match
* validar firmas con `validate_segwit_block_parallel()` o equivalente

Reglas críticas:

* short IDs solo son optimización de transporte
* nunca confiar en short IDs para consenso
* si hay colisión de short ID, resolver pidiendo objeto completo y validando root/hash completo
* witness swapping debe seguir fallando
* root mismatch debe seguir fallando

---

## 7. Tests obligatorios

Agregar tests para:

1. Full block → compact block conserva cantidad y orden.
2. Reconstrucción completa desde mempool funciona.
3. Si falta un tx_core, devuelve `MissingCompactRequest`.
4. Si falta un witness, devuelve `MissingCompactRequest`.
5. Missing response reconstruye correctamente.
6. Missing response con short_id incorrecto falla.
7. Missing response con objeto extra falla.
8. Witness swap sigue fallando después de reconstrucción.
9. Root incorrecto sigue fallando.
10. Bloque reconstruido valida con `validate_segwit_block_parallel()`.
11. Colisión artificial de short_id no permite aceptar bloque inválido.
12. Compact block reduce tamaño estimado versus full block.

---

## 8. Quality gate

Ejecutar:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

---

## No implementar todavía

* pruning
* fee model weight-based
* networking real entre peers
* persistencia en disco
* relay protocol completo

Solo P3: compact block model + reconstruction + missing object flow.
