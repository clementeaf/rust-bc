# Testnet Manual Runbook

Minimal testnet for Cerulean Ledger. 2-3 nodos TCP, SegWit/PQC blocks, sin TLS.

## 1. Setup

```bash
cargo build --bin testnet_node
```

## 2. Levantar nodos

Abrir 3 terminales. Definir una direccion genesis (40 hex chars):

```bash
ALICE=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
```

### Terminal 1 — Nodo A (puerto 3000)

```bash
cargo run --bin testnet_node -- node \
  --port 3000 \
  --peers 127.0.0.1:3001,127.0.0.1:3002 \
  --genesis "$ALICE:100000"
```

### Terminal 2 — Nodo B (puerto 3001)

```bash
cargo run --bin testnet_node -- node \
  --port 3001 \
  --peers 127.0.0.1:3000 \
  --genesis "$ALICE:100000"
```

### Terminal 3 — Nodo C (puerto 3002)

```bash
cargo run --bin testnet_node -- node \
  --port 3002 \
  --peers 127.0.0.1:3000,127.0.0.1:3001 \
  --genesis "$ALICE:100000"
```

> Todos los nodos deben tener la misma genesis. Peer list bidireccional (A conoce B y C).

## 3. Flujo basico

Abrir una 4a terminal para los comandos CLI:

### Enviar transaccion

```bash
cargo run --bin testnet_node -- send-tx \
  --from auto \
  --to bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \
  --amount 500 \
  --fee 1 \
  --node 127.0.0.1:3000
```

> `--from auto` genera un signer temporal. Para produccion, usar cerulean-wallet.

### Minar bloque

```bash
cargo run --bin testnet_node -- mine-block --node 127.0.0.1:3000
```

Output esperado:
```
[block] mined height=1 txs=1
```

### Consultar balance

```bash
cargo run --bin testnet_node -- show-balance \
  --addr bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \
  --node 127.0.0.1:3000
```

Output esperado:
```
[balance] bbbb...bbbb: 500 NOTA (nonce=0)
```

Verificar en los 3 nodos:

```bash
cargo run --bin testnet_node -- show-balance --addr $ALICE --node 127.0.0.1:3001
cargo run --bin testnet_node -- show-balance --addr $ALICE --node 127.0.0.1:3002
```

> Balances deben ser iguales en todos los nodos.

## 4. Test de resiliencia

1. Detener Nodo B (Ctrl+C en terminal 2)
2. Enviar tx y minar en A:
   ```bash
   cargo run --bin testnet_node -- send-tx --from auto --to cccc...cccc --amount 100 --fee 1 --node 127.0.0.1:3000
   cargo run --bin testnet_node -- mine-block --node 127.0.0.1:3000
   ```
3. Verificar que C recibio el bloque:
   ```bash
   cargo run --bin testnet_node -- show-balance --addr cccc...cccc --node 127.0.0.1:3002
   ```
4. Reiniciar B — conectara y sincronizara automaticamente via `SyncRequest`

## 5. Recovery test (con cerulean-wallet)

```bash
# En cerulean-wallet:
# 1. Crear HD wallet
cargo run -- keygen-hd

# 2. Shamir split
cargo run -- shamir-split

# 3. Borrar wallet
rm ~/.cerulean-wallet/wallet.json

# 4. Recuperar desde 2 shares
cargo run -- shamir-recover

# 5. La misma direccion debe derivarse
cargo run -- address
```

## 6. Duress test (con cerulean-wallet)

```bash
# Derivar direccion duress
cargo run -- duress-address

# La direccion duress debe ser distinta a la principal
# Ambas derivan del mismo seed, distinto path
```

## 7. Validaciones esperadas

| Condicion | Como verificar |
|-----------|---------------|
| Balances iguales en todos los nodos | `show-balance` en cada nodo |
| Nonce monotonico | Balance query muestra nonce incrementando |
| Bloques sincronizados | Minar en A, verificar balance en B y C |
| Txs propagadas | `send-tx` a A, minar en A, ver efecto en C |
| Bloque invalido rechazado | Cubierto por test `invalid_block_rejected` |

## 8. Errores conocidos

| Situacion | Comportamiento |
|-----------|----------------|
| Broadcast a nodo caido | Log `broadcast to X failed: connection refused` — no fatal |
| Mensajes duplicados | Nodo recibe tx ya en mempool — se ignora silenciosamente |
| Nodo reiniciado sin sync | Chain empieza de cero — necesita `SyncRequest` manual o auto |
| `send-tx --from auto` | Genera signer temporal — el nonce siempre es 0. Para txs secuenciales, usar cerulean-wallet con wallet persistente |

## 9. Arquitectura de red

```
        TCP
A ◄────────────► B
│                │
│    TCP         │
└──────► C ◄─────┘
```

- Transporte: TCP plano, JSON length-prefixed
- Sin TLS, sin libp2p, sin NAT traversal
- Validacion: `validate_block_versioned()` + `execute_transfer_checked()`
- Consenso: productor unico (quien mina), sin PoW/PoS
