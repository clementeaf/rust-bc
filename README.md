# 🚀 Rust Blockchain - Criptomoneda Completa

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Una implementación completa de blockchain con Proof of Work, red P2P distribuida, sistema de recompensas, y todas las características de una criptomoneda funcional, desarrollada en Rust.

## ✨ Características Principales

- 🔐 **Seguridad Criptográfica**: Firmas digitales Ed25519, validación completa
- ⛏️ **Minería Automática**: Proof of Work con dificultad dinámica y recompensas
- 🌐 **Red P2P Distribuida**: Comunicación entre nodos, sincronización automática
- 💰 **Sistema de Recompensas**: Recompensas automáticas con halving
- 💸 **Fees de Transacción**: Sistema completo con priorización
- 💾 **Persistencia SQLite**: Almacenamiento permanente
- 📡 **API REST Completa**: 15 endpoints para todas las operaciones
- 📊 **Estadísticas en Tiempo Real**: Monitoreo completo del sistema

## 🚀 Quick Start

### Instalación

```bash
# Clonar el repositorio
git clone <repository-url>
cd rust-bc

# Compilar
cargo build --release
```

### Ejecutar

```bash
# Iniciar servidor (API: 8080, P2P: 8081)
cargo run

# Con puertos personalizados
cargo run <api_port> <p2p_port> <db_name>
```

### Primeros Pasos

```bash
# 1. Crear wallet
curl -X POST http://127.0.0.1:8080/api/v1/wallets/create

# 2. Minar bloque (obtener recompensa)
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_address": "TU_DIRECCION", "max_transactions": 10}'

# 3. Ver estadísticas
curl http://127.0.0.1:8080/api/v1/stats
```

## 📚 Documentación

### Para Usuarios
- 📖 [Guía de Usuario Completa](Documents/GUIA_USUARIO.md) - Tutorial paso a paso
- 📡 [Documentación de API](Documents/API_DOCUMENTATION.md) - Referencia completa de endpoints
- 🚀 [README Completo](Documents/README_COMPLETO.md) - Documentación detallada

### Para Desarrolladores
- 🎯 [Resumen Final](Documents/RESUMEN_FINAL.md) - Estado completo del proyecto
- ✅ [Fase 5 Completada](Documents/FASE5_COMPLETADA.md) - Sistema de recompensas
- 🔧 [Mejoras Implementadas](Documents/MEJORAS_IMPLEMENTADAS.md) - Mejoras técnicas
- 🔍 [Verificación del Sistema](Documents/VERIFICACION_SISTEMA.md) - Verificación completa

### Historial de Desarrollo
- [Fase 1](Documents/FASE1_COMPLETADA.md) - Persistencia + API REST
- [Fase 2](Documents/FASE2_COMPLETADA.md) - Firmas Digitales
- [Fase 3](Documents/FASE3_COMPLETADA.md) - Red P2P
- [Fase 4](Documents/FASE4_CONSENSO_DISTRIBUIDO.md) - Consenso Distribuido
- [Fase 5](Documents/FASE5_COMPLETADA.md) - Sistema de Recompensas

## 🏗️ Arquitectura

```
src/
├── main.rs          # Servidor principal (API + P2P)
├── blockchain.rs    # Lógica de blockchain, PoW, dificultad dinámica
├── models.rs        # Transaction, Wallet, WalletManager, Mempool
├── database.rs     # Persistencia SQLite
├── api.rs           # Endpoints REST (15 endpoints)
└── network.rs       # Red P2P y protocolo de mensajería
```

## 📡 API REST - Endpoints Principales

### Minería
- `POST /api/v1/mine` - Minar bloque con recompensas automáticas
- `GET /api/v1/mempool` - Ver transacciones pendientes

### Wallets
- `POST /api/v1/wallets/create` - Crear wallet
- `GET /api/v1/wallets/{address}` - Obtener balance

### Transacciones
- `POST /api/v1/transactions` - Crear transacción (con fee opcional)

### Blockchain
- `GET /api/v1/chain/info` - Información de la blockchain
- `GET /api/v1/stats` - Estadísticas del sistema
- `GET /api/v1/chain/verify` - Verificar cadena

### Red P2P
- `GET /api/v1/peers` - Lista de peers
- `POST /api/v1/peers/{address}/connect` - Conectar a peer
- `POST /api/v1/sync` - Sincronizar blockchain

**Ver [API_DOCUMENTATION.md](Documents/API_DOCUMENTATION.md) para todos los 15 endpoints.**

## 🌐 Red P2P - Múltiples Nodos

```bash
# Terminal 1 - Nodo 1
cargo run 8080 8081 blockchain1

# Terminal 2 - Nodo 2
cargo run 8082 8083 blockchain2

# Terminal 3 - Nodo 3
cargo run 8084 8085 blockchain3

# Conectar nodos
curl -X POST http://127.0.0.1:8082/api/v1/peers/127.0.0.1:8081/connect
```

Los nodos se sincronizan automáticamente y los bloques se propagan a toda la red.

## 🧪 Testing

```bash
# Verificación estructural
./scripts/test_complete.sh

# Prueba funcional (requiere servidor corriendo)
./scripts/test_endpoints.sh

# Prueba con múltiples nodos (requiere 3 nodos)
./scripts/test_multi_node.sh
```

## 📊 Características Técnicas

### Proof of Work
- **Algoritmo**: SHA256
- **Dificultad**: Dinámica (ajuste automático cada 10 bloques)
- **Target**: 60 segundos por bloque
- **Rango**: 1-20 (protección)

### Firmas Digitales
- **Algoritmo**: Ed25519
- **Validación**: Criptográfica completa
- **Mismo algoritmo**: Usado por Solana

### Sistema de Recompensas
- **Recompensa base**: 50 unidades
- **Halving**: Cada 210,000 bloques
- **Fees**: Se suman a la recompensa del minero
- **Cálculo**: Automático

### Límites de Seguridad
- **Máximo transacciones/bloque**: 1000
- **Tamaño máximo de bloque**: 1MB
- **Capacidad mempool**: 1000 transacciones

## ✅ Estado del Proyecto

### Completado (100%)
- ✅ Todas las 5 fases implementadas
- ✅ Dificultad dinámica
- ✅ Fees de transacción
- ✅ Límites de tamaño
- ✅ Endpoint de estadísticas
- ✅ Documentación completa
- ✅ Scripts de testing

### Características
- ✅ Criptomoneda funcional completa
- ✅ Red P2P distribuida
- ✅ Consenso distribuido
- ✅ Sistema de recompensas
- ✅ Mempool con priorización

## 🎓 Casos de Uso

- **Aprendizaje**: Entender blockchain y criptomonedas
- **Desarrollo**: Base para proyectos blockchain
- **Prototipado**: Testing de conceptos
- **Educación**: Enseñanza de conceptos fundamentales

## 📖 Requisitos

- **Rust 1.70+** y Cargo
- **SQLite** (incluido automáticamente)

## 🔧 Configuración

### Variables de Entorno

```bash
export API_PORT=8080      # Puerto de la API REST
export P2P_PORT=8081      # Puerto del servidor P2P
export DB_NAME=blockchain # Nombre de la base de datos
```

### Parámetros de Blockchain

Configurables en el código:
- Dificultad inicial: 4
- Tiempo objetivo: 60 segundos
- Intervalo de ajuste: 10 bloques
- Recompensa base: 50 unidades

## 🤝 Contribuir

Las contribuciones son bienvenidas. Este es un proyecto educativo y de aprendizaje.

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

## 📚 Documentación Completa

Para más información, consulta la documentación completa en la carpeta `Documents/`:

- [Guía de Usuario](Documents/GUIA_USUARIO.md) - Tutorial completo
- [Documentación de API](Documents/API_DOCUMENTATION.md) - Todos los endpoints
- [Resumen Final](Documents/RESUMEN_FINAL.md) - Estado del proyecto
- [Recomendaciones Finales](Documents/RECOMENDACIONES_FINALES.md) - Próximos pasos

---

**¿Preguntas?** Consulta la [Guía de Usuario](Documents/GUIA_USUARIO.md) o la [Documentación de API](Documents/API_DOCUMENTATION.md).

**¡Disfruta usando la blockchain!** 🚀

