# 📖 Guía de Usuario Completa - Rust Blockchain

## 🎯 Introducción

Esta guía te ayudará a usar la blockchain completa, desde la instalación hasta operaciones avanzadas con múltiples nodos.

---

## 📦 Instalación

### Requisitos Previos

- **Rust 1.70+**: [Instalar Rust](https://www.rust-lang.org/tools/install)
- **Cargo**: Incluido con Rust
- **SQLite**: Incluido automáticamente (bundled)

### Compilación

```bash
# Clonar y entrar al directorio
cd rust-bc

# Compilar en modo release (optimizado)
cargo build --release

# El binario estará en: target/release/rust-bc
```

---

## 🚀 Inicio Rápido

### 1. Iniciar el Servidor

```bash
# Modo básico (puertos por defecto)
cargo run

# El servidor iniciará:
# - API REST en: http://127.0.0.1:8080
# - Servidor P2P en: 127.0.0.1:8081
```

### 2. Crear tu Primer Wallet

```bash
curl -X POST http://127.0.0.1:8080/api/v1/wallets/create
```

**Respuesta**:
```json
{
  "success": true,
  "data": {
    "address": "a1b2c3d4e5f6...",
    "balance": 0,
    "public_key": "def456..."
  }
```

**Guarda la dirección** (`address`) - la necesitarás para todas las operaciones.

### 3. Minar tu Primer Bloque

```bash
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_address": "TU_DIRECCION_AQUI", "max_transactions": 10}'
```

**Respuesta**:
```json
{
  "success": true,
  "data": {
    "hash": "0000abc123...",
    "reward": 50,
    "transactions_count": 1
  }
}
```

¡Felicidades! Has minado tu primer bloque y recibiste 50 unidades de recompensa.

### 4. Verificar tu Balance

```bash
curl http://127.0.0.1:8080/api/v1/wallets/TU_DIRECCION_AQUI
```

Deberías ver un balance de 50 unidades.

---

## 💰 Operaciones Básicas

### Crear una Transacción

```bash
curl -X POST http://127.0.0.1:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d '{
    "from": "DIRECCION_ORIGEN",
    "to": "DIRECCION_DESTINO",
    "amount": 25,
    "fee": 1
  }'
```

**Notas**:
- `fee` es opcional (default: 0)
- Transacciones con fees más altos se minan primero
- La transacción se agrega automáticamente al mempool

### Ver Transacciones Pendientes (Mempool)

```bash
curl http://127.0.0.1:8080/api/v1/mempool
```

### Minar Bloque con Transacciones

```bash
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{
    "miner_address": "TU_DIRECCION",
    "max_transactions": 10
  }'
```

El sistema automáticamente:
1. Toma transacciones del mempool (ordenadas por fee)
2. Calcula recompensa base (50 unidades)
3. Suma todos los fees de las transacciones
4. Crea transacción coinbase con recompensa total
5. Mina el bloque
6. Procesa todas las transacciones

### Ver Estadísticas del Sistema

```bash
curl http://127.0.0.1:8080/api/v1/stats
```

**Respuesta incluye**:
- Número de bloques
- Total de transacciones
- Dificultad actual
- Tiempo promedio de bloque
- Tamaño del mempool
- Peers conectados
- Y más...

---

## 🌐 Red P2P - Múltiples Nodos

### Configuración de Múltiples Nodos

Para probar la red distribuida, necesitas ejecutar múltiples nodos:

**Terminal 1 - Nodo 1**:
```bash
cargo run 8080 8081 blockchain1
```

**Terminal 2 - Nodo 2**:
```bash
cargo run 8082 8083 blockchain2
```

**Terminal 3 - Nodo 3**:
```bash
cargo run 8084 8085 blockchain3
```

### Conectar Nodos

```bash
# Desde Nodo 2, conectar a Nodo 1
curl -X POST http://127.0.0.1:8082/api/v1/peers/127.0.0.1:8081/connect

# Desde Nodo 3, conectar a Nodo 1
curl -X POST http://127.0.0.1:8084/api/v1/peers/127.0.0.1:8081/connect

# Desde Nodo 3, conectar a Nodo 2
curl -X POST http://127.0.0.1:8084/api/v1/peers/127.0.0.1:8083/connect
```

### Verificar Conexiones

```bash
# Ver peers del Nodo 1
curl http://127.0.0.1:8080/api/v1/peers

# Ver peers del Nodo 2
curl http://127.0.0.1:8082/api/v1/peers
```

### Sincronización

Los nodos se sincronizan automáticamente al conectarse. También puedes sincronizar manualmente:

```bash
# Sincronizar Nodo 2 con todos sus peers
curl -X POST http://127.0.0.1:8082/api/v1/sync
```

### Probar Broadcast

1. Minar un bloque en el Nodo 1
2. Esperar unos segundos
3. Verificar que los otros nodos recibieron el bloque:

```bash
# Verificar Nodo 2
curl http://127.0.0.1:8082/api/v1/chain/info

# Verificar Nodo 3
curl http://127.0.0.1:8084/api/v1/chain/info
```

Todos deberían tener el mismo número de bloques.

---

## 📊 Consultas y Verificación

### Ver Todos los Bloques

```bash
curl http://127.0.0.1:8080/api/v1/blocks
```

### Ver Bloque Específico

```bash
# Por hash
curl http://127.0.0.1:8080/api/v1/blocks/HASH_DEL_BLOQUE

# Por índice
curl http://127.0.0.1:8080/api/v1/blocks/index/0
```

### Ver Transacciones de un Wallet

```bash
curl http://127.0.0.1:8080/api/v1/wallets/DIRECCION/transactions
```

### Verificar Validez de la Cadena

```bash
curl http://127.0.0.1:8080/api/v1/chain/verify
```

### Información de la Blockchain

```bash
curl http://127.0.0.1:8080/api/v1/chain/info
```

---

## 🔧 Configuración Avanzada

### Variables de Entorno

```bash
export API_PORT=8080
export P2P_PORT=8081
export DB_NAME=blockchain
```

### Parámetros de Blockchain

Los parámetros están en `src/main.rs`:

```rust
let difficulty = 4; // Dificultad inicial
```

Y en `src/blockchain.rs`:

```rust
target_block_time: 60,                    // 60 segundos
difficulty_adjustment_interval: 10,       // Cada 10 bloques
max_transactions_per_block: 1000,        // Máximo 1000 transacciones
max_block_size_bytes: 1_000_000,        // 1MB máximo
```

---

## 🧪 Testing

### Scripts de Prueba Disponibles

```bash
# Verificación estructural (no requiere servidor)
./scripts/test_complete.sh

# Prueba funcional (requiere servidor corriendo)
./scripts/test_endpoints.sh

# Prueba con múltiples nodos (requiere 3 nodos)
./scripts/test_multi_node.sh
```

---

## ⚠️ Solución de Problemas

### El servidor no inicia

**Problema**: Puerto ya en uso
**Solución**: Usa puertos diferentes
```bash
cargo run 8086 8087 blockchain
```

### Los nodos no se conectan

**Problema**: Firewall o configuración de red
**Solución**: 
- Verifica que los puertos estén abiertos
- Usa `127.0.0.1` para pruebas locales
- Verifica que los nodos estén corriendo

### Balance incorrecto

**Problema**: Wallets no sincronizados
**Solución**: 
- El sistema sincroniza automáticamente al iniciar
- Si persiste, reinicia el servidor

### Transacciones no se minan

**Problema**: Mempool lleno o validación fallida
**Solución**:
- Verifica que las transacciones sean válidas
- Revisa el mempool: `GET /api/v1/mempool`
- Verifica balances suficientes (incluyendo fees)

---

## 💡 Consejos y Mejores Prácticas

### 1. Guarda tus Direcciones
- Las direcciones de wallets son importantes
- Guárdalas de forma segura
- Sin la dirección, no puedes acceder al wallet

### 2. Usa Fees Apropiados
- Fees más altos = minería más rápida
- Fees bajos o cero = pueden tardar más en minarse
- Recomendado: 1-5 unidades de fee

### 3. Monitorea el Sistema
- Usa `/api/v1/stats` regularmente
- Verifica el tamaño del mempool
- Monitorea la dificultad

### 4. Red P2P
- Conecta múltiples nodos para mejor distribución
- Sincroniza regularmente
- Verifica que todos los nodos tengan la misma cadena

### 5. Seguridad
- No compartas tus claves privadas
- Valida transacciones antes de confiar
- Verifica la cadena regularmente

---

## 📚 Ejemplos Completos

### Flujo Completo: Wallet → Transacción → Minería

```bash
# 1. Crear wallets
WALLET1=$(curl -s -X POST http://127.0.0.1:8080/api/v1/wallets/create | grep -o '"address":"[^"]*' | cut -d'"' -f4)
WALLET2=$(curl -s -X POST http://127.0.0.1:8080/api/v1/wallets/create | grep -o '"address":"[^"]*' | cut -d'"' -f4)

echo "Wallet 1: $WALLET1"
echo "Wallet 2: $WALLET2"

# 2. Minar bloque para Wallet 1
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d "{\"miner_address\":\"$WALLET1\",\"max_transactions\":10}"

# 3. Crear transacción
curl -X POST http://127.0.0.1:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d "{\"from\":\"$WALLET1\",\"to\":\"$WALLET2\",\"amount\":25,\"fee\":1}"

# 4. Minar bloque con la transacción
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d "{\"miner_address\":\"$WALLET1\",\"max_transactions\":10}"

# 5. Verificar balances
curl http://127.0.0.1:8080/api/v1/wallets/$WALLET1
curl http://127.0.0.1:8080/api/v1/wallets/$WALLET2
```

---

## 🎓 Conceptos Importantes

### Proof of Work
- Los mineros buscan un nonce que produzca un hash con N ceros al inicio
- N = dificultad actual
- Mayor dificultad = más tiempo de minado

### Dificultad Dinámica
- Se ajusta automáticamente cada 10 bloques
- Objetivo: mantener ~60 segundos por bloque
- Si muy rápido → aumenta dificultad
- Si muy lento → disminuye dificultad

### Fees de Transacción
- Incentivan a los mineros
- Transacciones con fees más altos se minan primero
- Los fees se suman a la recompensa del minero

### Mempool
- Pool de transacciones pendientes
- Máximo 1000 transacciones
- Se ordena por fee (mayor a menor)

### Consenso Distribuido
- Regla de cadena más larga
- Los nodos aceptan la cadena más larga válida
- Resuelve forks automáticamente

---

## 📞 Soporte

Para más información:
- Consulta [API_DOCUMENTATION.md](API_DOCUMENTATION.md) para detalles de la API
- Revisa [README_COMPLETO.md](README_COMPLETO.md) para información general
- Verifica [VERIFICACION_SISTEMA.md](VERIFICACION_SISTEMA.md) para estado del sistema

---

**¡Disfruta usando la blockchain!** 🚀

