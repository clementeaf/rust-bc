Implementa P1 en rust-bc: mempool verification cache para el modelo SegWit/PQC.

Contexto:
Ya existe `src/transaction/segwit.rs` con:

* `TxCore`
* `TxWitness`
* `SignatureScheme`
* `compute_tx_root()`
* `compute_witness_root()`
* `validate_segwit_block()`
* conversión `NativeTransaction::to_segwit(pubkey)`

Objetivo:
Evitar verificar repetidamente firmas ML-DSA-65 costosas cuando una transacción ya fue validada previamente en mempool.

Implementar:

1. Crear `VerificationCache`

   * Ubicación sugerida: `src/transaction/verification_cache.rs`
   * Debe usar una key determinística:
     `cache_key = hash(tx_core || tx_witness)`
   * No usar solo `tx_core`, porque permitiría witness swapping.
   * No usar solo `signature`, porque podría reutilizarse mal.
   * Idealmente usar serialización canónica ya existente.

2. API mínima:

   * `new(max_entries: usize) -> Self`
   * `contains_valid(&self, core: &TxCore, witness: &TxWitness) -> bool`
   * `insert_valid(&mut self, core: &TxCore, witness: &TxWitness)`
   * `validate_or_insert(&mut self, core: &TxCore, witness: &TxWitness) -> Result<(), NativeTxError>`

3. Integración:

   * Agregar una variante de validación:
     `validate_segwit_block_with_cache(...)`
   * Debe:

     * validar `tx_root`
     * validar `witness_root`
     * validar `tx_cores.len() == witnesses.len()`
     * por cada par `(core, witness)`:

       * si está en cache como válido, saltar verificación criptográfica
       * si no está, verificar firma y luego insertar en cache
   * La versión sin cache debe seguir existiendo.

4. Seguridad:

   * Nunca cachear resultados inválidos.
   * Si cambia `amount`, `fee`, `nonce`, `chain_id`, `timestamp`, `kind`, `signature`, `public_key` o `signature_scheme`, la key debe cambiar.
   * Witness swapping debe seguir fallando.
   * Root mismatch debe seguir fallando antes o durante validación.
   * El cache no debe poder hacer aceptar un bloque inválido.

5. Evicción:

   * Implementar límite simple `max_entries`.
   * Si se supera, remover entradas antiguas.
   * Puede ser FIFO o LRU simple.
   * No introducir dependencias pesadas salvo que ya existan.

6. Tests obligatorios:

   * Cache miss verifica e inserta.
   * Cache hit evita reverificación.
   * Cambiar `amount` invalida cache.
   * Cambiar `signature` invalida cache.
   * Cambiar `public_key` invalida cache.
   * Witness swap no pasa aunque ambos witnesses estén cacheados.
   * Root incorrecto sigue fallando aunque las firmas estén cacheadas.
   * No se cachean firmas inválidas.
   * Evicción respeta `max_entries`.
   * `validate_segwit_block()` legacy sigue pasando todos los tests actuales.

7. Quality gate:

   * `cargo fmt`
   * `cargo clippy -- -D warnings`
   * `cargo test`

No implementar todavía:

* rayon
* pruning
* compact blocks
* fee model weight-based
* networking

Solo P1: cache de verificaciones para SegWit/PQC.
