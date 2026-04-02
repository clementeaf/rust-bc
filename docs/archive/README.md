# Blockchain PoW en Rust - API REST

Implementación completa de una blockchain con Proof of Work y API REST desde cero en Rust.

## Características

- ✅ Bloque génesis automático
- ✅ Proof of Work real con dificultad ajustable
- ✅ Sistema de transacciones estructuradas
- ✅ Sistema de wallets y saldos
- ✅ Persistencia con SQLite
- ✅ API REST completa (10 endpoints)
- ✅ Merkle Root para verificación eficiente
- ✅ Verificación automática de la cadena completa

## Requisitos

- Rust 1.70+ y Cargo

## Instalación y Ejecución

```bash
# Compilar el proyecto
cargo build --release

# Ejecutar el servidor API
cargo run

# El servidor estará disponible en:
# http://127.0.0.1:8080
```

## API REST

El proyecto incluye una API REST completa con los siguientes endpoints:

### Bloques
- `GET /api/v1/blocks` - Listar todos los bloques
- `GET /api/v1/blocks/{hash}` - Obtener bloque por hash
- `GET /api/v1/blocks/index/{index}` - Obtener bloque por índice
- `POST /api/v1/blocks` - Crear nuevo bloque con transacciones

### Transacciones
- `POST /api/v1/transactions` - Crear nueva transacción

### Wallets
- `GET /api/v1/wallets/{address}` - Obtener balance
- `POST /api/v1/wallets/{address}/create` - Crear wallet
- `GET /api/v1/wallets/{address}/transactions` - Transacciones del wallet

### Blockchain
- `GET /api/v1/chain/verify` - Verificar cadena
- `GET /api/v1/chain/info` - Información de la blockchain

Ver [API_DOCUMENTATION.md](API_DOCUMENTATION.md) para documentación completa.

## Estructura del Código

```
src/
├── main.rs          # Servidor HTTP principal
├── blockchain.rs     # Lógica de blockchain y bloques
├── models.rs        # Transaction, Wallet, WalletManager
├── database.rs      # Persistencia SQLite
└── api.rs           # Endpoints REST
```

## Ejemplo de Uso

### Crear un wallet y transacción

```bash
# Crear wallet
curl -X POST http://127.0.0.1:8080/api/v1/wallets/wallet1/create

# Crear bloque con transacción
curl -X POST http://127.0.0.1:8080/api/v1/blocks \
  -H "Content-Type: application/json" \
  -d '{
    "transactions": [
      {
        "from": "wallet1",
        "to": "wallet2",
        "amount": 100,
        "data": "Pago de prueba"
      }
    ]
  }'

# Verificar cadena
curl http://127.0.0.1:8080/api/v1/chain/verify
```

## Características Técnicas

- **Persistencia**: SQLite para almacenamiento permanente
- **API**: Actix Web para servidor HTTP
- **Transacciones**: Sistema completo con validación
- **Wallets**: Gestión de saldos y transacciones
- **Merkle Root**: Verificación eficiente de transacciones
- **Proof of Work**: Dificultad configurable (por defecto: 4)

## Documentación Adicional

- [API_DOCUMENTATION.md](API_DOCUMENTATION.md) - Documentación completa de la API
- [ESTRATEGIA_RENTABILIZACION.md](ESTRATEGIA_RENTABILIZACION.md) - Estrategia de monetización
- [FASE1_COMPLETADA.md](FASE1_COMPLETADA.md) - Resumen de implementación

