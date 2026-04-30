Implementa P5 en rust-bc: weight-based fee model para SegWit/PQC.

Objetivo:
Cobrar fees según costo real de la transacción:

* tamaño del TxCore
* tamaño del TxWitness
* signature_scheme
* peso de almacenamiento/red

Requisitos:

1. Crear cálculo de peso:

   * `core_weight = serialized_size(tx_core)`
   * `witness_weight = serialized_size(tx_witness)`
   * `total_weight = core_weight * CORE_MULTIPLIER + witness_weight * WITNESS_MULTIPLIER`

2. Definir constantes iniciales:

   * `CORE_MULTIPLIER = 4`
   * `WITNESS_MULTIPLIER = 1`
   * `BASE_TX_FEE = ...`
   * `FEE_PER_WEIGHT_UNIT = ...`

3. Implementar:

   * `calculate_tx_weight(core, witness)`
   * `calculate_required_fee(core, witness)`
   * `validate_fee(core, witness)`

4. Reglas:

   * tx inválida si `core.fee < required_fee`
   * ML-DSA naturalmente debe costar más por witness size
   * no hardcodear “ML-DSA paga más”; debe salir del tamaño real

5. Integración:

   * agregar validación opcional en block validation:
     `validate_segwit_block_with_fees(...)`

6. Tests:

   * Ed25519 tiene menor required_fee que ML-DSA
   * ML-DSA paga más por tamaño
   * tx con fee insuficiente falla
   * tx con fee exacto pasa
   * tx con fee superior pasa
   * cambiar witness cambia required_fee
   * fee validation no rompe validación criptográfica

No implementar todavía mercado dinámico de fees ni mempool priority.
