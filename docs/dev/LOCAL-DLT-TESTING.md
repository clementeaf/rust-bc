# Cerulean Ledger DLT — Entorno de pruebas local

Guía para levantar un nodo Cerulean Ledger DLT y exponerlo a internet para que una app externa pueda integrarse.

## Requisitos

- Docker Desktop
- ngrok (`brew install ngrok` + cuenta gratuita configurada)

## 1. Levantar el nodo

```bash
docker run -d --name cerulean-node \
  -p 9600:8080 \
  -e BIND_ADDR=0.0.0.0 \
  -e API_PORT=8080 \
  -e P2P_PORT=8081 \
  -e STORAGE_BACKEND=rocksdb \
  -e STORAGE_PATH=/app/data/rocksdb \
  -e ACL_MODE=permissive \
  -e RUST_LOG=info \
  -e NETWORK_ID=cerulean-test \
  -e ORG_ID=org1 \
  --platform linux/amd64 \
  ghcr.io/clementeaf/rust-bc:latest
```

Verificar que responde:

```bash
curl -s http://localhost:9600/api/v1/health | jq .
```

## 2. Exponer con ngrok

```bash
ngrok http 9600
```

ngrok muestra la URL pública (ej: `https://xxxx.ngrok-free.app`). Esa es la URL que usa la app externa.

Dashboard local de ngrok: `http://127.0.0.1:4040`

## 3. Endpoints relevantes para integración

### Health check

```
GET /api/v1/health
```

### Registrar hash (submit al ledger)

```
POST /api/v1/gateway/submit
Content-Type: application/json

{
  "chaincode_id": "notarize",
  "transaction": {
    "id": "<uuid>",
    "input_did": "did:cerulean:<firmante>",
    "output_recipient": "did:cerulean:<propietario>",
    "amount": 0
  }
}
```

Respuesta exitosa (200):

```json
{
  "data": {
    "tx_id": "<uuid>",
    "block_height": 1,
    "valid": true
  }
}
```

### Verificar registro

Si `gateway/submit` devolvio `200` con `valid: true`, la transaccion fue endorsed, ordenada y committed en el ledger. El `tx_id` + `block_height` son la prueba de registro.

La app externa solo necesita guardar el `tx_id` en su base de datos. No es necesario consultar de vuelta al nodo.

## 4. Patron de integracion para app de identidad digital

| Operacion en la app | Accion en Cerulean |
|---|---|
| `POST /api/documents` (upsert) | Guardar en PG, luego `POST /gateway/submit` con hash del doc como `id`. Guardar `tx_id` en PG. |
| `POST /api/signature/sign` | Despues de firmar, `POST /gateway/submit` con hash de la firma como `id`. Guardar `tx_id` en PG. |
| `GET /api/signature/verify/{hash}` | Si `dlt_tx_id` existe en PG, el hash fue registrado en Cerulean. El 200 del gateway es la prueba. |

## 5. Configuracion de la app externa

| Parametro | Valor |
|---|---|
| URL base | `https://<ngrok-url>/api/v1` (o `http://localhost:9600/api/v1` si es local) |
| Autenticacion | Ninguna (`ACL_MODE=permissive`) |
| TLS | Terminado por ngrok |
| Protocolo | HTTP REST (no gRPC) |

### Header obligatorio para ngrok

ngrok free tier intercepta la primera request con una pagina HTML de advertencia. Esto rompe el JSON parse y hace que `dlt_tx_id` quede en `None`.

Todas las requests a Cerulean via ngrok deben incluir:

```
ngrok-skip-browser-warning: true
```

Ejemplo con curl:

```bash
curl -s https://<ngrok-url>/api/v1/gateway/submit \
  -H "Content-Type: application/json" \
  -H "ngrok-skip-browser-warning: true" \
  -d '{"chaincode_id":"notarize","transaction":{"id":"hash-xxx","input_did":"did:cerulean:signer","output_recipient":"did:cerulean:owner","amount":0}}'
```

En el HTTP client de la app externa, agregar el header a todas las requests al nodo Cerulean.

## 6. Detener el entorno

```bash
docker stop cerulean-node && docker rm cerulean-node
killall ngrok
```

## Notas

- Este setup es para **pruebas solamente**. Un nodo sin TLS propio, sin ACL, sin cluster de orderers.
- La URL de ngrok cambia cada vez que se reinicia (plan gratuito). Para URL fija, usar ngrok con dominio reservado o desplegar en cloud.
- Para produccion, usar `docker compose up -d` con la red completa (3 peers + 3 orderers + Prometheus + Grafana) y TLS habilitado.
