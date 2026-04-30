Implementa P12 en rust-bc: canonical binary serialization para signing payload y hashes de consenso.

Objetivo:
Cerrar F-002 del audit: eliminar dependencia de `serde_json` como formato implícito de consenso y reemplazarlo por una serialización binaria canónica, determinística y versionada.

Contexto:
Audit finding:

* F-002 MEDIUM: `serde_json` key ordering no está explícitamente garantizado como contrato de consenso.
* Hoy funciona por `BTreeMap`, pero no debe ser base futura de firmas ni roots.

---

## 1. Crear módulo nuevo

Ubicación sugerida:

```text
src/transaction/canonical.rs
```

Objetivo del módulo:

* convertir `TxCore`
* `TxWitness`
* `BlockHeader`
* `VersionedBlockHeader`
* otros tipos de consenso

en bytes determinísticos.

---

## 2. Reglas de serialización

NO usar `serde_json` para:

* signing payload
* tx hash
* witness hash
* merkle roots
* block hash
* cache key

Usar formato binario propio y explícito:

### Tipos primitivos

```text
u8      → 1 byte
u32     → little-endian fijo
u64     → little-endian fijo
u128    → little-endian fijo
bytes   → length-prefix u32 + raw bytes
string  → length-prefix u32 + UTF-8 bytes
enum    → discriminant u8 explícito
option  → 0x00 None / 0x01 Some + value
```

No depender de layout de structs Rust.

---

## 3. Funciones mínimas

Implementar:

```rust
pub trait CanonicalEncode {
    fn encode_canonical(&self, out: &mut Vec<u8>);
    fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_canonical(&mut out);
        out
    }
}
```

Implementar para:

* `TxCore`
* `TxWitness`
* `SignatureScheme`
* `BlockVersion`
* `VersionedBlockHeader`
* `SegWitBlock` si aplica
* `LegacyBlock` si aplica

---

## 4. Signing payload versionado

Modificar:

```rust
signing_payload_for_version(core, version)
```

Para `SegWitPqcV1`:

```text
domain_separator || version_byte || canonical_encode(TxCore)
```

Para `Legacy`:

* mantener comportamiento legacy si es necesario
* pero documentar que Legacy usa payload histórico

No romper tests legacy existentes.

---

## 5. Roots y hashes

Migrar a canonical bytes:

* `compute_tx_root()` debe usar `TxCore::to_canonical_bytes()`
* `compute_witness_root()` debe usar `TxWitness::to_canonical_bytes()`
* compact block short IDs deben usar canonical bytes
* verification cache key debe usar canonical bytes
* block hash debe usar canonical bytes

---

## 6. Compatibilidad/migración

Si hay tests existentes que dependen de JSON:

* actualizarlos al nuevo hash/payload
* mantener tests explícitos de que legacy sigue funcionando
* si hay riesgo de romper firmas históricas, usar `BlockVersion`:

  * Legacy → old payload
  * SegWitPqcV1 → canonical binary payload

---

## 7. Seguridad

Agregar tests para asegurar:

1. mismo `TxCore` siempre produce mismos bytes
2. cambiar orden de campos en struct no puede alterar encoding lógico
3. cambiar amount cambia bytes
4. cambiar fee cambia bytes
5. cambiar nonce cambia bytes
6. cambiar chain_id cambia bytes
7. cambiar timestamp cambia bytes
8. cambiar kind cambia bytes
9. cambiar signature_scheme cambia witness bytes
10. cambiar signature cambia witness bytes
11. cambiar public_key cambia witness bytes
12. cache key cambia si cambia core o witness
13. short_id cambia si cambia core o witness
14. roots cambian si cambia core o witness
15. signing payload SegWitPqcV1 ya no contiene JSON delimiters como `{`, `}`, `:`, `"`

---

## 8. Audit regression tests

Agregar tests específicos para F-002:

* `canonical_encoding_is_not_json`
* `canonical_encoding_is_stable`
* `canonical_encoding_is_field_order_independent_by_design`
* `signing_payload_uses_canonical_binary_for_segwit_pqc_v1`
* `merkle_roots_use_canonical_binary`
* `cache_keys_use_canonical_binary`
* `compact_short_ids_use_canonical_binary`

---

## 9. Documentación

Actualizar:

```text
docs/pqc-consensus-invariants.md
README_PQC.md
docs/architecture/security/SEGWIT-PQC-AUDIT.md
```

Marcar F-002 como:

```text
FIXED — canonical binary serialization introduced for SegWitPqcV1 consensus bytes.
```

Documentar que:

* JSON no es formato de consenso para SegWitPqcV1
* Legacy mantiene comportamiento histórico solo por compatibilidad

---

## 10. Quality gate

Ejecutar:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo test --features acvp-tests
```

---

## No implementar todavía

* cambio de algoritmo hash
* nuevo formato de red
* hard fork adicional
* soporte multi-version más allá de Legacy y SegWitPqcV1

Solo P12: canonical binary serialization para cerrar F-002.
