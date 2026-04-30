Implementa P10 en rust-bc: integración de vectores oficiales NIST ACVP para ML-DSA-65.

Objetivo:
Validar la implementación de ML-DSA-65 contra vectores oficiales de NIST (ACVP), eliminando la dependencia implícita de PQClean CI y elevando el nivel de confianza criptográfica.

---

## 1. Ubicación

Crear estructura:

```
tests/acvp/
tests/acvp/mldsa65/
```

Archivos esperados (descargados desde NIST ACVP):

* `ML-DSA-65-keyGen.json`
* `ML-DSA-65-sigGen.json`
* `ML-DSA-65-sigVer.json`

NO modificar estos archivos. Deben mantenerse intactos.

---

## 2. Parsing

Crear módulo:

```
tests/acvp_parser.rs
```

Responsabilidades:

* parsear JSON ACVP (puede ser complejo/nested)
* soportar:

  * key generation vectors
  * signature generation vectors
  * signature verification vectors
* mapear a estructuras Rust internas

Ejemplo base:

```rust
struct AcvpSigVerTest {
    message: Vec<u8>,
    public_key: Vec<u8>,
    signature: Vec<u8>,
    expected_valid: bool,
}
```

---

## 3. Tests ACVP

Crear:

```
tests/mldsa65_acvp.rs
```

### 3.1 Signature verification

Para cada vector:

```rust
verify(public_key, message, signature) == expected_valid
```

### 3.2 Signature generation (si aplica)

* generar firma con tu implementación
* verificar que pase validación
* opcional: comparar determinismo si el vector lo exige

### 3.3 Key generation

* validar tamaños correctos
* validar que claves generadas funcionan en sign/verify

---

## 4. Integración con implementación actual

Usar:

* `pqcrypto-mldsa`
* tu wrapper actual en `rust-bc`

NO reimplementar ML-DSA.

---

## 5. Seguridad

* rechazar firmas truncadas explícitamente (`len == 3309`)
* rechazar claves inválidas (`len == 1952`)
* nunca asumir que ACVP solo trae datos válidos
* tests deben fallar explícitamente en mismatch

---

## 6. Performance

* limitar número de vectores si son miles (ej: sample subset)
* permitir feature flag opcional:

```toml
[features]
acvp-tests = []
```

Para correr:

```bash
cargo test --features acvp-tests
```

---

## 7. Tests obligatorios

1. Todos los vectores sigVer pasan correctamente
2. Vectores inválidos son rechazados
3. No hay falsos positivos
4. No hay falsos negativos
5. Firma generada localmente valida correctamente
6. Tamaños incorrectos fallan
7. Integración no rompe tests existentes

---

## 8. Output esperado

Al ejecutar:

```bash
cargo test --features acvp-tests
```

Debe mostrar:

```text
ACVP ML-DSA-65: ALL VECTORS PASSED
```

---

## 9. Quality gate

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo test --features acvp-tests
```

---

## No implementar todavía

* soporte para otros niveles ML-DSA (44/87)
* validación ACVP automática online
* generación de nuevos vectores
* cambios en consenso

Solo P10: validación contra vectores oficiales NIST ACVP.
