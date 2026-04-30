Implementa P4 en rust-bc: witness pruning para bloques SegWit/PQC.

Objetivo:
Permitir que nodos no-archival eliminen witnesses antiguos después de N confirmaciones, manteniendo datos ejecutables, tx_root y witness_root.

Requisitos:

1. Crear representación:

   * `PrunedSegWitBlock`
   * conserva header, tx_cores, tx_root, witness_root
   * elimina `witnesses`

2. Agregar:

   * `prune_witnesses(block, current_height, pruning_depth)`
   * solo poda si `current_height >= block_height + pruning_depth`

3. Validación:

   * bloque completo: valida roots + firmas
   * bloque podado: valida solo estructura, tx_root y presencia de witness_root histórico
   * no debe permitir ejecutar bloques nuevos sin witnesses

4. Tests:

   * no poda antes de depth
   * poda después de depth
   * tx_cores se conservan
   * witness_root se conserva
   * bloque podado no puede pasar como bloque completo
   * bloque completo sigue validando normal
   * pruning no altera tx_root

No implementar todavía fee model ni storage engine completo.
