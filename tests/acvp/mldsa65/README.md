# ML-DSA-65 ACVP Test Vectors

This directory contains ACVP-format test vectors for ML-DSA-65 (FIPS 204).

## Vector Sources

- `sigVer.json` — Signature verification vectors (valid + invalid cases)
- `sigGen.json` — Signature generation vectors (sign + verify roundtrip)
- `keyGen.json` — Key generation vectors (size validation)

## Format

Vectors follow the NIST ACVP JSON schema. Each file contains a `testGroups`
array with `tests` inside each group.

## Running

```bash
cargo test --features acvp-tests --test mldsa65_acvp
```

## Regenerating

Vectors are generated from the `pqcrypto-mldsa` v0.1.2 library. To regenerate:

```bash
cargo test --features acvp-tests --test mldsa65_acvp generate_vectors -- --ignored
```
