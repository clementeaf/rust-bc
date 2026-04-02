# ✅ NFTs Básicos - Implementación Completada

## Resumen

Se ha implementado un sistema completo de NFTs (Non-Fungible Tokens) básicos, similar al estándar ERC-721 simplificado, que permite crear, transferir y gestionar tokens únicos e indivisibles.

---

## Funcionalidades Implementadas

### 1. Funciones de Contrato NFT

#### Mint NFT
- **Función:** `mintNFT(to, token_id, token_uri)`
- **Descripción:** Crea un nuevo NFT y lo asigna a una dirección
- **Validaciones:**
  - Token ID único (no puede duplicarse)
  - URI máximo 2048 caracteres
  - Dirección válida

#### Transfer NFT
- **Función:** `transferNFT(from, to, token_id)`
- **Descripción:** Transfiere un NFT del owner a otra dirección
- **Validaciones:**
  - El caller debe ser el owner o estar aprobado
  - No se puede transferir a sí mismo
  - El token debe existir

#### Approve NFT
- **Función:** `approveNFT(to, token_id)`
- **Descripción:** Aprueba que otra dirección transfiera un NFT específico
- **Validaciones:**
  - El caller debe ser el owner del token
  - No se puede aprobar a sí mismo

#### TransferFrom NFT
- **Función:** `transferFromNFT(from, to, token_id)`
- **Descripción:** Transfiere un NFT usando approval (el spender debe estar aprobado)
- **Validaciones:**
  - El spender debe estar aprobado para el token
  - El token debe pertenecer a `from`
  - Limpia el approval después de transferir

### 2. Funciones de Consulta

#### Owner Of
- **Endpoint:** `GET /api/v1/contracts/{address}/nft/{token_id}/owner`
- **Descripción:** Obtiene el owner de un NFT específico

#### Token URI
- **Endpoint:** `GET /api/v1/contracts/{address}/nft/{token_id}/uri`
- **Descripción:** Obtiene la URI/metadata de un NFT

#### Get Approved
- **Endpoint:** `GET /api/v1/contracts/{address}/nft/{token_id}/approved`
- **Descripción:** Obtiene la dirección aprobada para transferir un NFT

#### Balance Of
- **Endpoint:** `GET /api/v1/contracts/{address}/nft/balance/{wallet}`
- **Descripción:** Obtiene la cantidad de NFTs que posee una dirección

#### Total Supply
- **Endpoint:** `GET /api/v1/contracts/{address}/nft/totalSupply`
- **Descripción:** Obtiene el total de NFTs minteados en el contrato

---

## Estructura de Datos

### ContractState Extendido

```rust
pub struct ContractState {
    // ERC-20 (existente)
    pub balances: HashMap<String, u64>,
    pub allowances: HashMap<String, HashMap<String, u64>>,
    
    // NFT (nuevo)
    pub token_owners: HashMap<u64, String>,        // token_id -> owner
    pub token_uris: HashMap<u64, String>,          // token_id -> URI
    pub token_approvals: HashMap<u64, String>,     // token_id -> approved address
    pub nft_balances: HashMap<String, u64>,       // owner -> count of NFTs
}
```

### ContractFunction Extendido

```rust
pub enum ContractFunction {
    // ... funciones ERC-20 existentes ...
    
    // NFT (nuevo)
    MintNFT { to: String, token_id: u64, token_uri: String },
    TransferNFT { from: String, to: String, token_id: u64 },
    ApproveNFT { to: String, token_id: u64 },
    TransferFromNFT { from: String, to: String, token_id: u64 },
}
```

---

## Características de Seguridad

### 1. Validación de Direcciones
- Todas las direcciones son validadas (longitud, formato)
- Prevención de self-transfer y self-approve

### 2. Unicidad de Tokens
- Cada `token_id` es único dentro del contrato
- No se pueden mintear tokens duplicados

### 3. Control de Permisos
- Solo el owner puede transferir directamente
- Sistema de approvals para transferencias delegadas
- Limpieza automática de approvals después de transferir

### 4. Integridad de Datos
- Hash de integridad incluye datos NFT
- Actualización de secuencia para prevenir race conditions
- Eventos registrados en metadata (con límite de crecimiento)

---

## Endpoints API

### Ejecutar Funciones NFT

```bash
# Mint NFT
POST /api/v1/contracts/{address}/execute
{
  "function": "mintNFT",
  "params": {
    "to": "wallet_address",
    "token_id": 1,
    "token_uri": "https://example.com/nft/1"
  }
}

# Transfer NFT
POST /api/v1/contracts/{address}/execute
{
  "function": "transferNFT",
  "params": {
    "caller": "wallet_address",
    "from": "wallet_address",
    "to": "recipient_address",
    "token_id": 1
  }
}

# Approve NFT
POST /api/v1/contracts/{address}/execute
{
  "function": "approveNFT",
  "params": {
    "caller": "owner_address",
    "to": "approved_address",
    "token_id": 1
  }
}

# TransferFrom NFT
POST /api/v1/contracts/{address}/execute
{
  "function": "transferFromNFT",
  "params": {
    "caller": "spender_address",
    "from": "owner_address",
    "to": "recipient_address",
    "token_id": 1
  }
}
```

### Consultar NFTs

```bash
# Owner of token
GET /api/v1/contracts/{address}/nft/{token_id}/owner

# Token URI
GET /api/v1/contracts/{address}/nft/{token_id}/uri

# Approved address
GET /api/v1/contracts/{address}/nft/{token_id}/approved

# Balance of NFTs
GET /api/v1/contracts/{address}/nft/balance/{wallet}

# Total supply
GET /api/v1/contracts/{address}/nft/totalSupply
```

---

## Rate Limiting

Las funciones NFT están incluidas en el sistema de rate limiting:
- **Límite:** 10 requests/segundo por caller
- **Límite:** 100 requests/minuto por caller
- **Tracking:** Independiente por caller (no afecta a otros usuarios)

---

## Eventos

### Transfer Event
Se registra en metadata con formato:
```
event_nft_transfer_{sequence}: "from:{from}|to:{to}|token_id:{token_id}"
```

### Approval Event
Se registra en metadata con formato:
```
event_nft_approval_{sequence}: "owner:{owner}|approved:{approved}|token_id:{token_id}"
```

**Límite de eventos:** Máximo 1000 eventos, manteniendo los últimos 500.

---

## Testing

### Test Completo
Script: `scripts/test_nft_complete.sh`

**Cubre:**
1. ✅ Mint NFT
2. ✅ Verificar owner
3. ✅ Verificar URI
4. ✅ Verificar balance
5. ✅ Mint múltiples NFTs
6. ✅ Transfer NFT
7. ✅ Approve NFT
8. ✅ TransferFrom NFT (usando approval)
9. ✅ Total Supply
10. ✅ Prevención de mint duplicado

---

## Integración con Sistema Existente

### Compatibilidad
- ✅ Coexiste con contratos ERC-20
- ✅ Mismo sistema de integridad y sincronización P2P
- ✅ Mismo sistema de rate limiting
- ✅ Misma persistencia en base de datos

### Diferencias con ERC-20
- **Tokens únicos:** Cada NFT tiene un `token_id` único
- **No divisibles:** Los NFTs no tienen cantidad (solo ownership)
- **Metadata individual:** Cada NFT tiene su propia URI
- **Approvals por token:** Aprobaciones específicas por token (no por owner)

---

## Limitaciones Actuales

1. **No hay enumeración:** No se puede listar todos los tokens de un owner
2. **URI simple:** Solo almacena string, no valida formato JSON
3. **Sin royalties:** No hay sistema de royalties integrado
4. **Sin metadata on-chain:** La metadata está en URI externa

---

## Próximas Mejoras Posibles

1. **Enumeración de tokens:** `tokensOfOwner(address)`
2. **Metadata on-chain:** Almacenar JSON metadata directamente
3. **Batch operations:** Mint/transfer múltiples NFTs en una transacción
4. **Royalties:** Sistema de royalties para ventas secundarias
5. **Burn NFT:** Función para quemar/destruir NFTs

---

## Estado

**Implementación:** ✅ **COMPLETA**
**Testing:** ✅ **COMPLETO**
**Documentación:** ✅ **COMPLETA**
**Listo para producción:** ✅ **SÍ**

---

**Fecha:** $(date)
**Versión:** 1.0

