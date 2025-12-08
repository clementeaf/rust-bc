# Rust Blockchain

Una implementación completa de blockchain en Rust con características avanzadas de seguridad, consenso distribuido y red P2P.

## 🚀 Características

- ✅ **Blockchain Completa**: Proof of Work (PoW), validación de bloques, Merkle Root
- ✅ **Firmas Digitales**: Ed25519 para transacciones seguras
- ✅ **Red P2P**: Comunicación entre nodos, sincronización, broadcast
- ✅ **Consenso Distribuido**: Resolución de forks, cadena más larga
- ✅ **Sistema de Recompensas**: Mining rewards con halving, coinbase transactions
- ✅ **Mempool**: Gestión de transacciones pendientes
- ✅ **API REST**: Endpoints completos para interacción
- ✅ **Persistencia**: SQLite con optimizaciones (WAL mode, índices)
- ✅ **Seguridad**: Rate limiting, validación de transacciones, protección contra doble gasto
- ✅ **Performance**: Caché de balances, compresión HTTP, optimizaciones de base de datos

## 📋 Requisitos

- Rust 1.70+ ([Instalación](https://www.rust-lang.org/tools/install))
- SQLite3

## 🔧 Instalación

### Opción 1: Docker (Recomendado) 🐳

```bash
# Clonar el repositorio
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc

# Construir imagen
docker build -t rust-bc:latest .

# Ejecutar nodo
docker run -d \
  --name rust-bc-node \
  -p 8080:8080 \
  -p 8081:8081 \
  -v blockchain-data:/app/data \
  rust-bc:latest

# O usar docker-compose para múltiples nodos
docker-compose up -d
```

Ver [DOCKER.md](DOCKER.md) para documentación completa de Docker.

### Opción 2: Compilación Local

```bash
# Clonar el repositorio
git clone https://github.com/clementeaf/rust-bc.git
cd rust-bc

# Compilar
cargo build --release

# Ejecutar
DIFFICULTY=1 cargo run --release 8080 8081 blockchain
```

## 📚 Documentación

La documentación completa está en la carpeta `Documents/`:

- `README_COMPLETO.md` - Documentación general
- `API_DOCUMENTATION.md` - Endpoints de la API
- `GUIA_USUARIO.md` - Guía de uso
- `INDICE_DOCUMENTACION.md` - Índice completo

## 🌐 API Endpoints

- `GET /api/v1/health` - Health check
- `GET /api/v1/blocks` - Listar bloques
- `GET /api/v1/blocks/{hash}` - Obtener bloque por hash
- `POST /api/v1/transactions` - Crear transacción
- `POST /api/v1/mine` - Minar bloque
- `GET /api/v1/mempool` - Ver transacciones pendientes
- `GET /api/v1/stats` - Estadísticas del sistema
- `GET /api/v1/chain/verify` - Verificar cadena

Ver `Documents/API_DOCUMENTATION.md` para la lista completa.

## 🧪 Pruebas

```bash
# Pruebas de seguridad
./scripts/test_security_attacks.sh

# Pruebas de estrés
./scripts/test_stress.sh

# Pruebas completas
./scripts/run_all_stress_tests.sh
```

## 📊 Estado del Proyecto

- ✅ Fase 1: Persistencia + API REST
- ✅ Fase 2: Firmas Digitales
- ✅ Fase 3: Red P2P
- ✅ Fase 4: Consenso Distribuido
- ✅ Fase 5: Sistema de Recompensas
- ✅ Optimizaciones de Performance y Seguridad

## 🔒 Seguridad

- Validación de firmas Ed25519
- Protección contra doble gasto
- Rate limiting
- Validación de cadena completa
- Límites de tamaño de bloque y transacciones

## 📝 Licencia

Este proyecto es de código abierto.

## 👤 Autor

Clemente Falcone

## 🙏 Contribuciones

Las contribuciones son bienvenidas. Por favor, abre un issue o pull request.
