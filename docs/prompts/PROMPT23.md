Implementa P2 en rust-bc: verificación paralela de firmas para bloques SegWit/PQC usando rayon.

Contexto:

* Existe `validate_segwit_block_with_cache()`
* Existe `VerificationCache`
* ML-DSA verify es costoso pero paralelizable
* Cache ya reduce verificaciones redundantes

Objetivo:
Paralelizar la verificación de firmas en bloques grandes sin romper:

* seguridad
* determinismo
* orden lógico

Requisitos:

1. Integrar `rayon`

   * Usar `par_iter()` sobre `(tx_cores, witnesses)`
   * Mantener zip seguro (posición i ↔ i)

2. Nueva función:
   `validate_segwit_block_parallel(...)`

3. Lógica:

   * Validar roots primero (secuencial)
   * Validar length match
   * Iterar en paralelo:

     * si está en cache → skip
     * si no → verificar firma
   * Si alguna falla → abortar bloque completo

4. Cache:

   * NO mutar cache dentro de `par_iter()` directamente
   * recolectar resultados válidos en buffer temporal
   * insertar en cache después (secuencial)

5. Seguridad:

   * ningún cambio en lógica de verificación
   * ningún cambio en signing payload
   * mismo resultado que versión secuencial

6. Tests:

   * bloque válido pasa en paralelo
   * bloque inválido falla
   * resultados idénticos a versión secuencial
   * cache sigue funcionando
   * no hay race conditions

7. Performance test (simple):

   * comparar secuencial vs paralelo con 100–1000 txs
   * demostrar mejora

No implementar todavía:

* networking
* compact blocks
* pruning

Solo paralelización de verify.
