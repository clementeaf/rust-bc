Implementa P8 en rust-bc: versionado de bloque y migración de consenso para SegWit/PQC.

Objetivo:
Introducir versionado explícito de bloques para soportar:

* bloques legacy (pre-SegWit/PQC)
* bloques SegWit/PQC actuales
* futuras evoluciones sin romper consenso

---

## 1. Crear enum de versión

Ubicación sugerida:
`src/block/version.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockVersion {
    Legacy = 0,
    SegWitPqcV1 = 1,
}
```

---

## 2. Extender BlockHeader

Agregar campo:

```rust
pub struct BlockHeader {
    pub version: BlockVersion,
    pub tx_root: Hash,
    pub witness_root: Option<Hash>, // None para Legacy
    // otros campos existentes
}
```

Reglas:

* `Legacy` → `witness_root == None`
* `SegWitPqcV1` → `witness_root == Some(...)`

---

## 3. Separar tipos de bloque

Mantener:

```rust
LegacyBlock
SegWitBlock
```

No mezclar estructuras.

---

## 4. Validación version-aware

Crear:

```rust
pub fn validate_block_versioned(
    block: &AnyBlock,
    cache: &mut VerificationCache,
    config: &PqcValidationConfig,
) -> Result<(), BlockError>
```

Donde:

```rust
pub enum AnyBlock {
    Legacy(LegacyBlock),
    SegWit(SegWitBlock),
}
```

Lógica:

* `Legacy`:

  * usar validación legacy existente
  * ignorar PQC

* `SegWitPqcV1`:

  * usar `validate_pqc_block(...)`

---

## 5. Reglas de consenso por versión

### Legacy

* sin witnesses
* sin weight-based fee
* firma Ed25519

### SegWitPqcV1

* dual merkle roots
* witness obligatorio
* weight-based fee
* ML-DSA o Ed25519 soportados
* pipeline oficial obligatorio

---

## 6. Migración / fork

Definir:

```rust
pub struct ChainConfig {
    pub segwit_pqc_activation_height: u64,
}
```

Regla:

```text
si block_height < activation_height → solo Legacy
si block_height >= activation_height → solo SegWitPqcV1
```

Rechazar bloques con versión incorrecta para su altura.

---

## 7. Seguridad

* nunca aceptar bloque SegWit sin witness_root
* nunca aceptar bloque Legacy con witness_root
* nunca mezclar validaciones
* version debe formar parte del hash del bloque
* version debe formar parte del consenso

---

## 8. Tests obligatorios

1. Legacy block válido antes de activation_height
2. SegWit block rechazado antes de activation_height
3. SegWit block válido después de activation_height
4. Legacy block rechazado después de activation_height
5. witness_root None en SegWit falla
6. witness_root Some en Legacy falla
7. validate_block_versioned enruta correctamente
8. cambiar version invalida bloque
9. block hash cambia si version cambia
10. mezcla de estructuras falla

---

## 9. Compatibilidad

* no romper tests existentes
* mantener validadores legacy funcionales
* permitir migración progresiva

---

## 10. Quality gate

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

---

## No implementar todavía

* multi-version support en mempool
* replay entre versiones
* upgrades dinámicos
* soft forks complejos

Solo P8: versionado + routing de validación + regla de activación.
