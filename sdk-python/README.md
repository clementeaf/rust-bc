# rust-bc Python SDK

Cliente Python para la red blockchain rust-bc.

## Instalación

```bash
pip install -e sdk-python/
```

O con dependencias de desarrollo:

```bash
pip install -e "sdk-python/[dev]"
```

## Uso rápido

```python
from rust_bc import BlockchainClient, TransactionInput

client = BlockchainClient("https://localhost:8080/api/v1", verify_ssl=False)

# Health check
health = client.health()
print(health.status)  # "healthy"

# Enviar transacción
tx = TransactionInput(
    id="tx-001",
    input_did="did:bc:alice",
    output_recipient="did:bc:bob",
    amount=100,
)
result = client.submit_transaction("mycc", "", tx)
print(result.block_height)  # 3

# Crear canal
channel = client.create_channel("mychannel")

# Datos privados
client.put_private_data("secret", "key1", "valor", "org1")
value = client.get_private_data("secret", "key1", "org1")

client.close()
```

## Métodos disponibles

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| `health()` | GET /health | Health check con dependencias |
| `submit_transaction()` | POST /gateway/submit | Pipeline endorse → order → commit |
| `evaluate()` | POST /chaincode/{id}/simulate | Query read-only de chaincode |
| `register_org()` | POST /store/organizations | Registrar organización |
| `list_orgs()` | GET /store/organizations | Listar organizaciones |
| `set_policy()` | POST /store/policies | Configurar política de endorsement |
| `create_channel()` | POST /channels | Crear canal |
| `list_channels()` | GET /channels | Listar canales |
| `put_private_data()` | PUT /private-data/{c}/{k} | Escribir datos privados |
| `get_private_data()` | GET /private-data/{c}/{k} | Leer datos privados |
| `get_blocks()` | GET /blocks | Blockchain completa |
| `get_block_by_index()` | GET /blocks/index/{i} | Bloque por altura |
| `create_wallet()` | POST /wallets/create | Crear wallet |
| `mine_block()` | POST /mine | Minar bloque |
| `chain_info()` | GET /chain/info | Metadata de la cadena |
| `verify_chain()` | GET /chain/verify | Verificar integridad |

## Tests

```bash
cd sdk-python
pip install -e ".[dev]"
pytest -v
```
