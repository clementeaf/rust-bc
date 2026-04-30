Implementa P0 en rust-bc: separación estructural de transacciones y witnesses para soportar firmas PQC grandes.

Objetivo:
Cambiar el modelo de bloque desde:

Block {
transactions: Vec<NativeTransaction>
}

hacia:

Block {
tx_cores: Vec<TxCore>,
witnesses: Vec<TxWitness>,
tx_root: Hash,
witness_root: Hash
}

Requisitos:

1. Crear `TxCore` con solo datos ejecutables:

   * from
   * to
   * amount
   * fee
   * nonce
   * chain_id
   * timestamp
   * kind si aplica

2. Crear `TxWitness` con:

   * signature
   * public_key
   * signature_scheme: Ed25519 | MLDSA65

3. Mantener compatibilidad temporal:

   * agregar conversión desde `NativeTransaction` hacia `(TxCore, TxWitness)`
   * no romper wallet todavía

4. Validación de bloque:

   * `tx_cores.len() == witnesses.len()`
   * cada `witness[i]` valida `tx_core[i]`
   * calcular `tx_root` sobre `tx_cores`
   * calcular `witness_root` sobre `witnesses`
   * rechazar bloque si algún root no coincide

5. Seguridad:

   * impedir witness swapping
   * la firma debe verificarse contra el signing payload derivado del `TxCore`
   * mantener `chain_id` y `nonce` dentro del core firmado

6. Tests obligatorios:

   * bloque válido con ML-DSA pasa
   * cambiar un witness de posición falla
   * cambiar amount/fee/nonce/chain_id falla
   * witness_root incorrecto falla
   * tx_root incorrecto falla
   * cantidad distinta de tx_cores y witnesses falla
   * conversión legacy NativeTransaction -> TxCore + TxWitness preserva firma válida

No implementes todavía:

* pruning
* compact blocks
* fee model nuevo
* rayon
* cache de verificación
* networking

Solo P0: modelo de bloque + roots + validación.
