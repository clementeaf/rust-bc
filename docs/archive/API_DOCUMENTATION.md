# 📚 Documentación Completa de la API REST

## Base URL
```
http://127.0.0.1:8080/api/v1
```

## 📊 Resumen de Endpoints

**Total: 15 endpoints**

- **Bloques**: 4 endpoints
- **Transacciones**: 1 endpoint
- **Wallets**: 3 endpoints
- **Minería**: 2 endpoints
- **Blockchain**: 3 endpoints
- **Red P2P**: 2 endpoints

---

## 📦 Endpoints Detallados

### Bloques

#### GET /blocks
Obtiene todos los bloques de la blockchain.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "index": 0,
      "timestamp": 1234567890,
      "transactions": [...],
      "previous_hash": "0",
      "hash": "0000...",
      "nonce": 12345,
      "difficulty": 4,
      "merkle_root": "abc123..."
    }
  ]
}
```

#### GET /blocks/{hash}
Obtiene un bloque específico por su hash.

**Parámetros:**
- `hash` (path): Hash del bloque

**Response:**
```json
{
  "success": true,
  "data": {
    "index": 1,
    "timestamp": 1234567890,
    "transactions": [...],
    "hash": "0000...",
    ...
  }
}
```

#### GET /blocks/index/{index}
Obtiene un bloque por su índice.

**Parámetros:**
- `index` (path): Índice del bloque (número)

**Response:**
```json
{
  "success": true,
  "data": {
    "index": 1,
    ...
  }
}
```

#### POST /blocks
Crea un nuevo bloque con transacciones (manual, sin recompensa automática).

**Request Body:**
```json
{
  "transactions": [
    {
      "from": "wallet1",
      "to": "wallet2",
      "amount": 100,
      "fee": 1,
      "data": "Transacción de prueba"
    }
  ]
}
```

**Nota**: Para minería con recompensas automáticas, usa `POST /mine`

**Response:**
```json
{
  "success": true,
  "data": "0000abc123..."
}
```

---

### Transacciones

#### POST /transactions
Crea una nueva transacción (se agrega al mempool).

**Request Body:**
```json
{
  "from": "wallet1",
  "to": "wallet2",
  "amount": 50,
  "fee": 1,
  "data": "Descripción opcional"
}
```

**Parámetros:**
- `from` (required): Dirección del wallet origen
- `to` (required): Dirección del wallet destino
- `amount` (required): Cantidad a transferir
- `fee` (optional): Fee de transacción (default: 0)
- `data` (optional): Datos adicionales

**Notas**:
- La transacción se firma automáticamente si el wallet existe
- Se agrega al mempool para ser minada
- Transacciones con fees más altos se minan primero

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "uuid-v4",
    "from": "wallet1",
    "to": "wallet2",
    "amount": 50,
    "fee": 1,
    "timestamp": 1234567890,
    "signature": "firma_digital..."
  }
}
```

### Wallets

#### GET /wallets/{address}
Obtiene el balance de un wallet.

**Parámetros:**
- `address` (path): Dirección del wallet

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "wallet1",
    "balance": 1000
  }
}
```

#### POST /wallets/create
Crea un nuevo wallet con keypair criptográfico.

**Nota**: No requiere parámetros en la URL. La dirección se genera automáticamente desde la clave pública.

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "a1b2c3d4e5f6...",
    "balance": 0,
    "public_key": "def456..."
  }
}
```

**Nota**: Guarda la dirección (`address`) - la necesitarás para todas las operaciones.

#### GET /wallets/{address}/transactions
Obtiene todas las transacciones de un wallet.

**Parámetros:**
- `address` (path): Dirección del wallet

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "from": "wallet1",
      "to": "wallet2",
      "amount": 100,
      ...
    }
  ]
}
```

### Minería

#### POST /mine
Minera un nuevo bloque con recompensas automáticas.

**Request Body:**
```json
{
  "miner_address": "abc123...",
  "max_transactions": 10
}
```

**Parámetros:**
- `miner_address` (required): Dirección del minero que recibirá la recompensa
- `max_transactions` (optional): Máximo de transacciones a incluir (default: 10)

**Funcionamiento**:
1. Toma transacciones del mempool (ordenadas por fee)
2. Calcula recompensa base (50 unidades, con halving)
3. Suma todos los fees de las transacciones
4. Crea transacción coinbase con recompensa total
5. Mina el bloque
6. Procesa todas las transacciones

**Response:**
```json
{
  "success": true,
  "data": {
    "hash": "0000abc123...",
    "reward": 55,
    "transactions_count": 3
  }
}
```

#### GET /mempool
Obtiene todas las transacciones pendientes en el mempool.

**Response:**
```json
{
  "success": true,
  "data": {
    "count": 5,
    "transactions": [
      {
        "id": "uuid",
        "from": "wallet1",
        "to": "wallet2",
        "amount": 100,
        "fee": 2,
        ...
      }
    ]
  }
}
```

---

### Blockchain

#### GET /chain/verify
Verifica la validez de toda la cadena.

**Response:**
```json
{
  "success": true,
  "data": {
    "valid": true,
    "block_count": 5
  }
}
```

#### GET /chain/info
Obtiene información general de la blockchain.

**Response:**
```json
{
  "success": true,
  "data": {
    "block_count": 5,
    "difficulty": 4,
    "latest_block_hash": "0000...",
    "is_valid": true
  }
}
```

#### GET /stats
Obtiene estadísticas completas del sistema.

**Response:**
```json
{
  "success": true,
  "data": {
    "blockchain": {
      "block_count": 10,
      "total_transactions": 25,
      "difficulty": 4,
      "latest_block_hash": "0000...",
      "latest_block_index": 9,
      "total_coinbase": 500,
      "unique_addresses": 5,
      "avg_block_time_seconds": 58.5,
      "target_block_time": 60,
      "max_transactions_per_block": 1000,
      "max_block_size_bytes": 1000000
    },
    "mempool": {
      "pending_transactions": 3,
      "total_fees_pending": 5
    },
    "network": {
      "connected_peers": 2
    }
  }
}
```

---

### Red P2P

#### GET /peers
Obtiene la lista de peers conectados.

**Response:**
```json
{
  "success": true,
  "data": [
    "127.0.0.1:8083",
    "127.0.0.1:8085"
  ]
}
```

#### POST /peers/{address}/connect
Conecta a un peer en la red P2P.

**Parámetros:**
- `address` (path): Dirección del peer (formato: IP:PUERTO)

**Ejemplo:**
```bash
curl -X POST http://127.0.0.1:8080/api/v1/peers/127.0.0.1:8081/connect
```

**Response:**
```json
{
  "success": true,
  "data": "Conectando a 127.0.0.1:8081"
}
```

#### POST /sync
Sincroniza la blockchain con todos los peers conectados.

**Response:**
```json
{
  "success": true,
  "data": "Sincronización iniciada"
}
```

## Ejemplos de Uso

### cURL

#### Crear un wallet
```bash
curl -X POST http://127.0.0.1:8080/api/v1/wallets/wallet1/create
```

#### Obtener balance
```bash
curl http://127.0.0.1:8080/api/v1/wallets/wallet1
```

#### Crear una transacción
```bash
curl -X POST http://127.0.0.1:8080/api/v1/transactions \
  -H "Content-Type: application/json" \
  -d '{
    "from": "wallet1",
    "to": "wallet2",
    "amount": 100,
    "data": "Pago de prueba"
  }'
```

#### Crear un bloque
```bash
curl -X POST http://127.0.0.1:8080/api/v1/blocks \
  -H "Content-Type: application/json" \
  -d '{
    "transactions": [
      {
        "from": "wallet1",
        "to": "wallet2",
        "amount": 100
      }
    ]
  }'
```

#### Verificar la cadena
```bash
curl http://127.0.0.1:8080/api/v1/chain/verify
```

### JavaScript (Fetch)

```javascript
// Crear transacción
const response = await fetch('http://127.0.0.1:8080/api/v1/transactions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    from: 'wallet1',
    to: 'wallet2',
    amount: 100,
    data: 'Pago'
  })
});

const result = await response.json();
console.log(result);
```

## Códigos de Estado HTTP

- `200 OK`: Operación exitosa
- `201 Created`: Recurso creado exitosamente
- `400 Bad Request`: Solicitud inválida
- `404 Not Found`: Recurso no encontrado
- `500 Internal Server Error`: Error del servidor

## Formato de Respuesta

Todas las respuestas siguen este formato:

```json
{
  "success": boolean,
  "data": any,
  "message": string (opcional, solo en errores)
}
```

---

## 📝 Notas Importantes

### Transacciones
- Todas las transacciones deben ser válidas (from, to, amount > 0)
- Las transacciones se firman automáticamente con Ed25519
- El campo `fee` es opcional (default: 0)
- Transacciones con fees más altos se minan primero
- Las transacciones se agregan automáticamente al mempool

### Minería
- Los bloques se minan automáticamente con Proof of Work
- La dificultad se ajusta dinámicamente cada 10 bloques
- Recompensa base: 50 unidades (con halving cada 210,000 bloques)
- Los fees de las transacciones se suman a la recompensa del minero
- Máximo 1000 transacciones por bloque
- Tamaño máximo de bloque: 1MB

### Wallets
- Los wallets se crean con keypairs criptográficos automáticamente
- Las direcciones se derivan de las claves públicas
- Los balances se calculan desde todas las transacciones históricas
- Los wallets se sincronizan automáticamente al iniciar

### Red P2P
- Los nodos se sincronizan automáticamente al conectarse
- Los bloques se propagan automáticamente a todos los peers
- Consenso: regla de cadena más larga
- Los forks se resuelven automáticamente

### Persistencia
- Los datos se persisten automáticamente en SQLite
- La blockchain se carga automáticamente al iniciar
- Los wallets se sincronizan desde la blockchain al iniciar

---

## 🔒 Seguridad

- **Firmas Digitales**: Todas las transacciones están firmadas con Ed25519
- **Validación Completa**: Transacciones validadas antes de agregar a bloques
- **Prevención de Doble Gasto**: Detección automática
- **Límites de Tamaño**: Protección contra ataques DoS
- **Validación de Saldos**: Verificación antes de procesar transacciones

---

## 📊 Códigos de Estado HTTP

- `200 OK`: Operación exitosa
- `201 Created`: Recurso creado exitosamente
- `400 Bad Request`: Solicitud inválida
- `404 Not Found`: Recurso no encontrado
- `500 Internal Server Error`: Error del servidor
- `503 Service Unavailable`: Servicio no disponible (ej: nodo P2P no disponible)

---

## 📋 Formato de Respuesta

Todas las respuestas siguen este formato estándar:

```json
{
  "success": boolean,
  "data": any,
  "message": string (opcional, solo en errores)
}
```

**Ejemplo de éxito**:
```json
{
  "success": true,
  "data": { ... }
}
```

**Ejemplo de error**:
```json
{
  "success": false,
  "data": null,
  "message": "Descripción del error"
}
```

