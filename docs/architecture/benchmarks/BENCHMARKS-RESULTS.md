# Benchmark Results

Resultados reales medidos con Criterion (micro-benchmarks estadísticos, 100+ iteraciones).

**Hardware:** MacBook Pro Apple Silicon, macOS Darwin 25.3.0
**Rust:** nightly
**Fecha:** 2026-04-15

---

## Ordering throughput

Mide transacciones/segundo a través del Solo ordering service: submit N transacciones y cortar un bloque.

| Batch size | Tiempo medio | Throughput |
|---|---|---|
| 100 TXs | ~5.3 ms | **~18,700 TX/s** |

## Endorsement validation

Latencia de verificación de firmas Ed25519 con política AllOf(N orgs).

| N orgs | Latencia media | Por endorsement |
|---|---|---|
| 1 | ~2.7 µs | 2.7 µs |
| 3 | ~8.1 µs | 2.7 µs |
| 5 | ~13.5 µs | 2.7 µs |
| 10 | ~27 µs | 2.7 µs |

Escalamiento lineal. Cada endorsement adicional agrega ~2.7 µs (costo de una verificación Ed25519).

## Event bus fan-out

Latencia de `publish()` al crecer el número de suscriptores.

| Suscriptores | Latencia media | Throughput |
|---|---|---|
| 1 | 307 ns | 3.26 M eventos/s |
| 5 | 331 ns | 15.1 M eventos/s (agregado) |
| 10 | 379 ns | 26.4 M eventos/s (agregado) |
| 50 | 612 ns | 81.8 M eventos/s (agregado) |

Fan-out altamente eficiente: 50 suscriptores solo duplican la latencia respecto a 1 suscriptor.

## RocksDB storage write

Escritura secuencial de bloques a RocksDB con column families.

| Bloques | Tiempo medio | Throughput |
|---|---|---|
| 10 | 593 µs | **16,850 bloques/s** |
| 100 | 969 µs | **103,160 bloques/s** |

El throughput mejora con batch size mayor gracias a amortización de I/O (LSM-tree write amplification).

---

## Comparación con Fabric 2.5

| Métrica | rust-bc | Fabric 2.5 | Notas |
|---|---|---|---|
| Ordering throughput | ~18,700 TX/s | ~2,000-5,000 TX/s | Solo ordering, single node |
| Endorsement validation | ~2.7 µs/firma | ~50-100 µs/firma | Ed25519 vs ECDSA P-256 |
| Memory footprint | ~50 MB/nodo | ~500 MB/peer | Container RSS |
| Startup time | ~2s | ~15-30s | Cold start |
| Storage write | ~103K bloques/s | ~10-30K bloques/s | RocksDB vs LevelDB |

**Caveats:**
- Fabric usa gRPC (protobuf) vs rust-bc usa HTTP/JSON — overhead de serialización diferente
- Los números de Fabric son aproximaciones de la literatura pública, no mediciones propias
- Para una comparación rigurosa, ejecutar Hyperledger Caliper contra ambos sistemas con la misma carga
- Los micro-benchmarks miden componentes aislados, no throughput end-to-end bajo carga real

---

## Reproducir estos benchmarks

```bash
# Micro-benchmarks (no requiere Docker)
cargo bench

# Benchmark específico
cargo bench -- ordering_service
cargo bench -- endorsement_validation
cargo bench -- event_bus_fanout
cargo bench -- rocksdb_storage

# Reportes HTML
open target/criterion/report/index.html
```

Para benchmarks live (requiere red Docker corriendo):

```bash
docker compose up -d node1 node2 node3 orderer1
./scripts/benchmark.sh
```
