# 🚀 Rust Blockchain - Criptomoneda Completa

Una implementación completa de blockchain con Proof of Work, red P2P distribuida, sistema de recompensas, y todas las características de una criptomoneda funcional, desarrollada en Rust.

## ✨ Características Principales

### 🔐 Seguridad Criptográfica
- ✅ **Firmas Digitales Ed25519** - Autenticación criptográfica robusta
- ✅ **Validación de Transacciones** - Verificación completa antes de agregar a bloques
- ✅ **Prevención de Doble Gasto** - Detección automática de transacciones duplicadas
- ✅ **Wallets Criptográficos** - Generación automática de keypairs

### ⛏️ Minería y Consenso
- ✅ **Proof of Work (PoW)** - Algoritmo de consenso con dificultad ajustable
- ✅ **Dificultad Dinámica** - Ajuste automático para mantener tiempos de bloque consistentes
- ✅ **Sistema de Recompensas** - Recompensas automáticas con halving (cada 210,000 bloques)
- ✅ **Fees de Transacción** - Sistema completo de fees que se suman a recompensas

### 🌐 Red Distribuida
- ✅ **Comunicación P2P** - Protocolo TCP para comunicación entre nodos
- ✅ **Sincronización Automática** - Los nodos se sincronizan automáticamente
- ✅ **Broadcast de Bloques** - Propagación automática de bloques minados
- ✅ **Consenso Distribuido** - Regla de cadena más larga para resolver conflictos
- ✅ **Discovery de Peers** - Conexión y gestión de múltiples nodos

### 💾 Persistencia y API
- ✅ **Base de Datos SQLite** - Persistencia completa de bloques y wallets
- ✅ **API REST Completa** - 15 endpoints para todas las operaciones
- ✅ **Mempool** - Pool de transacciones pendientes con priorización por fees
- ✅ **Estadísticas en Tiempo Real** - Endpoint de métricas del sistema

## 📋 Requisitos

- **Rust 1.70+** y Cargo
- **SQLite** (incluido con `rusqlite` bundled)

## 🚀 Instalación

```bash
# Clonar el repositorio
git clone <repository-url>
cd rust-bc

# Compilar el proyecto
cargo build --release

# El binario estará en: target/release/rust-bc
```

## 🎯 Uso Rápido

### Iniciar un Nodo

```bash
# Modo básico (puertos por defecto: API 8080, P2P 8081)
cargo run

# Con puertos personalizados
cargo run <api_port> <p2p_port> <db_name>

# Ejemplo: API en 8080, P2P en 8081, BD "blockchain"
cargo run 8080 8081 blockchain
```

### Ejemplo Completo

```bash
# 1. Crear un wallet
curl -X POST http://127.0.0.1:8080/api/v1/wallets/create

# Respuesta:
# {
#   "success": true,
#   "data": {
#     "address": "abc123...",
#     "balance": 0,
#     "public_key": "def456..."
#   }
# }

# 2. Minar un bloque para obtener recompensa
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_address": "abc123...", "max_transactions": 10}'

# 3. Verificar balance
curl http://127.0.0.1:8080/api/v1/wallets/abc123...

# 4. Crear una transacción
curl -X POST http://127.0.0.1:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d '{
    "from": "abc123...",
    "to": "xyz789...",
    "amount": 25,
    "fee": 1
  }'

# 5. Ver estadísticas del sistema
curl http://127.0.0.1:8080/api/v1/stats
```

## 📡 Red P2P - Múltiples Nodos

### Iniciar Múltiples Nodos

**Terminal 1 (Nodo 1)**:
```bash
cargo run 8080 8081 blockchain1
```

**Terminal 2 (Nodo 2)**:
```bash
cargo run 8082 8083 blockchain2
```

**Terminal 3 (Nodo 3)**:
```bash
cargo run 8084 8085 blockchain3
```

### Conectar Nodos

```bash
# Desde Nodo 2, conectar a Nodo 1
curl -X POST http://127.0.0.1:8082/api/v1/peers/127.0.0.1:8081/connect

# Desde Nodo 3, conectar a Nodo 1
curl -X POST http://127.0.0.1:8084/api/v1/peers/127.0.0.1:8081/connect

# Ver peers conectados
curl http://127.0.0.1:8080/api/v1/peers
```

### Sincronización Automática

Los nodos se sincronizan automáticamente al conectarse. También puedes sincronizar manualmente:

```bash
curl -X POST http://127.0.0.1:8080/api/v1/sync
```

## 📚 API REST - Endpoints

### Bloques
- `GET /api/v1/blocks` - Listar todos los bloques
- `GET /api/v1/blocks/{hash}` - Obtener bloque por hash
- `GET /api/v1/blocks/index/{index}` - Obtener bloque por índice
- `POST /api/v1/blocks` - Crear nuevo bloque (manual)

### Transacciones
- `POST /api/v1/transactions` - Crear nueva transacción

### Wallets
- `GET /api/v1/wallets/{address}` - Obtener balance de wallet
- `POST /api/v1/wallets/create` - Crear nuevo wallet
- `GET /api/v1/wallets/{address}/transactions` - Transacciones de un wallet

### Minería
- `POST /api/v1/mine` - Minar bloque con recompensas automáticas
- `GET /api/v1/mempool` - Ver transacciones pendientes

### Blockchain
- `GET /api/v1/chain/verify` - Verificar validez de la cadena
- `GET /api/v1/chain/info` - Información de la blockchain
- `GET /api/v1/stats` - Estadísticas del sistema

### Red P2P
- `GET /api/v1/peers` - Lista de peers conectados
- `POST /api/v1/peers/{address}/connect` - Conectar a un peer
- `POST /api/v1/sync` - Sincronizar con todos los peers

Ver [API_DOCUMENTATION.md](API_DOCUMENTATION.md) para documentación completa.

## 🏗️ Arquitectura

```
src/
├── main.rs          # Servidor principal (API + P2P)
├── blockchain.rs    # Lógica de blockchain, PoW, dificultad dinámica
├── models.rs        # Transaction, Wallet, WalletManager, Mempool
├── database.rs      # Persistencia SQLite
├── api.rs           # Endpoints REST
└── network.rs       # Red P2P y protocolo de mensajería
```

## 🔧 Configuración

### Parámetros de Blockchain

- **Dificultad inicial**: 4 (configurable en `main.rs`)
- **Tiempo objetivo de bloque**: 60 segundos
- **Intervalo de ajuste de dificultad**: 10 bloques
- **Recompensa base**: 50 unidades
- **Halving**: Cada 210,000 bloques
- **Máximo de transacciones por bloque**: 1000
- **Tamaño máximo de bloque**: 1MB

### Variables de Entorno

```bash
export API_PORT=8080      # Puerto de la API REST
export P2P_PORT=8081      # Puerto del servidor P2P
export DB_NAME=blockchain # Nombre de la base de datos
```

## 🧪 Testing

### Scripts de Prueba

```bash
# Verificación estructural
./scripts/test_complete.sh

# Prueba funcional de endpoints (requiere servidor corriendo)
./scripts/test_endpoints.sh

# Prueba con múltiples nodos (requiere 3 nodos corriendo)
./scripts/test_multi_node.sh
```

## 📊 Características Técnicas

### Proof of Work
- Algoritmo: SHA256
- Dificultad: Ajuste dinámico automático
- Target: Hash que comienza con N ceros (donde N = dificultad)

### Firmas Digitales
- Algoritmo: Ed25519
- Mismo algoritmo usado por Solana
- Validación criptográfica completa

### Base de Datos
- Motor: SQLite (bundled)
- Tablas: `blocks`, `wallets`
- Persistencia automática

### Red P2P
- Protocolo: TCP
- Mensajería: JSON sobre TCP
- Sincronización: Automática y manual

## 🎓 Casos de Uso

### 1. Aprendizaje y Educación
- Entender cómo funciona una blockchain
- Aprender Proof of Work
- Estudiar redes P2P distribuidas
- Experimentar con criptomonedas

### 2. Desarrollo y Prototipado
- Base para proyectos blockchain
- Testing de conceptos
- Desarrollo de features adicionales

### 3. Aplicaciones Prácticas
- Sistema de logging inmutable
- Notarización digital
- Registro de transacciones
- Auditoría distribuida

## 📈 Estado del Proyecto

### ✅ Completado (100%)
- ✅ Fase 1: Persistencia + API REST
- ✅ Fase 2: Firmas Digitales
- ✅ Fase 3: Red P2P
- ✅ Fase 4: Consenso Distribuido
- ✅ Fase 5: Sistema de Recompensas
- ✅ Dificultad Dinámica
- ✅ Fees de Transacción
- ✅ Límites de Tamaño
- ✅ Estadísticas del Sistema

### 🚀 Características Avanzadas
- ✅ Mempool con priorización por fees
- ✅ Sincronización automática de wallets
- ✅ Validación completa de transacciones
- ✅ Resolución de forks
- ✅ Broadcast automático

## 📖 Documentación Adicional

- [API_DOCUMENTATION.md](API_DOCUMENTATION.md) - Documentación completa de la API
- [GUIA_USUARIO.md](GUIA_USUARIO.md) - Guía de usuario completa
- [FASE5_COMPLETADA.md](FASE5_COMPLETADA.md) - Detalles del sistema de recompensas
- [MEJORAS_IMPLEMENTADAS.md](MEJORAS_IMPLEMENTADAS.md) - Mejoras adicionales
- [VERIFICACION_SISTEMA.md](VERIFICACION_SISTEMA.md) - Verificación del sistema

## 🤝 Contribuir

Este es un proyecto educativo y de aprendizaje. Las contribuciones son bienvenidas:

1. Fork el proyecto
2. Crea una rama para tu feature
3. Commit tus cambios
4. Push a la rama
5. Abre un Pull Request

## 📝 Licencia

Este proyecto es de código abierto y está disponible para uso educativo y de desarrollo.

## 🙏 Agradecimientos

Implementación completa de blockchain desde cero en Rust, incluyendo todas las características esenciales de una criptomoneda funcional.

---

**¿Preguntas?** Consulta la [GUIA_USUARIO.md](GUIA_USUARIO.md) para más detalles.

