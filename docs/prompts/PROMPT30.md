Implementa P9 en rust-bc: replay protection entre Legacy y SegWitPqcV1.

Objetivo:
Evitar que una transacción firmada para una versión de bloque/ruta de consenso pueda reutilizarse válidamente en otra.

Contexto:
Ya existe:

* `BlockVersion::Legacy`
* `BlockVersion::SegWitPqcV1`
* `validate_block_versioned(...)`
* `TxCore`
* `TxWitness`
* `validate_pqc_block(...)`

Requisito principal:
El signing payload de SegWitPqcV1 debe incluir explícitamente:

```text
domain_separator || block_version || chain_id || tx_core_fields
```

Ejemplo de domain separator:

```text
"RUST_BC_SEGWIT_PQC_V1_TX"
```

Reglas:

1. Crear función:

```rust
pub fn signing_payload_for_version(
    core: &TxCore,
    version: BlockVersion,
) -> Vec<u8>
```

2. Para `SegWitPqcV1`, el payload debe incluir:

* domain separator
* version
* from
* to
* amount
* fee
* nonce
* chain_id
* timestamp
* kind

3. La verificación de witness en ruta SegWit/PQC debe usar ese payload versionado.

4. Legacy debe mantener su signing payload existente para no romper compatibilidad.

5. Tests obligatorios:

* transacción SegWitPqcV1 válida pasa con payload versionado
* misma firma falla si se verifica como Legacy
* misma firma falla si se cambia `BlockVersion`
* cambiar `chain_id` falla
* cambiar `nonce` falla
* cambiar `fee` falla
* domain separator diferente falla
* validación versioned block sigue pasando
* tests legacy siguen pasando

6. Seguridad:

* no cambiar formato de `TxCore` si no es necesario
* no romper bloques Legacy
* no permitir replay cross-version
* no permitir replay cross-chain

Quality gate:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

No implementar todavía:

* replay protection entre forks dinámicos
* multi-chain bridge
* mempool multi-version
