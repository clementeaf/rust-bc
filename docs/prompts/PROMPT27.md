Implementa P6 en rust-bc: pipeline único oficial de validación para bloques SegWit/PQC.

Contexto:
Ya existen:

* `TxCore`
* `TxWitness`
* `SegWitBlock`
* `validate_segwit_block()`
* `validate_segwit_block_with_cache()`
* `validate_segwit_block_parallel()`
* `validate_segwit_block_with_fees()`
* `VerificationCache`
* `calculate_required_fee()`
* `validate_fee()`

Objetivo:
Crear una única función oficial de validación que combine:

* estructura
* roots
* fees
* cache
* verificación paralela de firmas

Función sugerida:

```rust
pub fn validate_pqc_block(
    block: &SegWitBlock,
    cache: &mut VerificationCache,
    config: &PqcValidationConfig,
) -> Result<(), NativeTxError>
```

Crear config:

```rust
pub struct PqcValidationConfig {
    pub enforce_fees: bool,
    pub use_cache: bool,
    pub parallel_verify: bool,
}
```

Requisitos:

1. Validar estructura:

   * `tx_cores.len() == witnesses.len()`
   * bloque vacío permitido solo si ya es válido actualmente
   * no aceptar witnesses sin core ni core sin witness

2. Validar roots:

   * recomputar `tx_root`
   * recomputar `witness_root`
   * rechazar mismatch

3. Validar fees:

   * si `config.enforce_fees == true`, aplicar `validate_fee(core, witness)`
   * si es false, omitir solo fees, no firmas ni roots

4. Validar firmas:

   * si `parallel_verify == true`, usar ruta paralela
   * si `use_cache == true`, aprovechar `VerificationCache`
   * no mutar cache desde threads directamente
   * insertar solo firmas válidas

5. Seguridad:

   * nunca permitir que cache salte roots
   * nunca permitir que cache salte fee validation
   * nunca cachear inválidos
   * witness swapping debe seguir fallando
   * cambios en amount/fee/nonce/chain_id/timestamp/kind deben invalidar

6. Mantener compatibilidad:

   * no borrar validadores anteriores todavía
   * los validadores antiguos pueden llamar internamente al nuevo pipeline si es seguro

7. Tests obligatorios:

   * bloque válido pasa con config completa
   * bloque con root incorrecto falla aunque esté cacheado
   * bloque con fee insuficiente falla aunque firma esté cacheada
   * bloque con witness swap falla
   * bloque válido pasa sin fees cuando `enforce_fees=false`
   * cache hit funciona
   * cache miss inserta
   * parallel y sequential devuelven mismo resultado
   * config `use_cache=false` no inserta ni lee cache
   * cambios en amount/fee/nonce/chain_id invalidan
   * validadores antiguos siguen pasando

8. Quality gate:

   * `cargo fmt`
   * `cargo clippy -- -D warnings`
   * `cargo test`

No implementar todavía:

* mempool priority
* dynamic fee market
* storage engine
* networking real
* consensus changes

Solo P6: pipeline único oficial de validación PQC.
