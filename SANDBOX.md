# Sandbox — Demo pública sin cloud

Expone Cerulean Ledger al internet desde tu propia máquina usando Cloudflare Tunnel. Sin AWS, sin GCP, sin cuentas cloud. Tu compute, tu data.

## Quick start (URL temporal, sin cuenta Cloudflare)

```bash
# 1. Instalar cloudflared (una vez)
brew install cloudflare/cloudflare/cloudflared

# 2. Levantar todo
./scripts/sandbox.sh
```

El script:
1. Construye los containers (nodo + explorer + voto)
2. Levanta Docker Compose
3. Abre 3 Cloudflare Quick Tunnels (URLs temporales `*.trycloudflare.com`)
4. Imprime las URLs públicas

Output esperado:
```
  Block Explorer:  https://abc-random.trycloudflare.com
  Cerulean Voto:   https://xyz-random.trycloudflare.com
  API (raw):       https://def-random.trycloudflare.com
```

Las URLs son válidas mientras el script corra. Ctrl+C para detener los tunnels.

```bash
# Apagar todo
./scripts/sandbox.sh stop
```

## Dominio propio (permanente, requiere cuenta Cloudflare gratuita)

Si quieres URLs estables como `sandbox.cerulean.cl`:

```bash
# 1. Login (una vez)
cloudflared tunnel login

# 2. Crear tunnel (una vez)
cloudflared tunnel create cerulean-sandbox

# 3. Configurar DNS (una vez por subdominio)
cloudflared tunnel route dns cerulean-sandbox sandbox.cerulean.cl
cloudflared tunnel route dns cerulean-sandbox voto.cerulean.cl
cloudflared tunnel route dns cerulean-sandbox api.cerulean.cl

# 4. Crear config
cat > ~/.cloudflared/config.yml << 'EOF'
tunnel: cerulean-sandbox
credentials-file: ~/.cloudflared/<TUNNEL-ID>.json

ingress:
  - hostname: sandbox.cerulean.cl
    service: http://localhost:5173
  - hostname: voto.cerulean.cl
    service: http://localhost:5174
  - hostname: api.cerulean.cl
    service: http://localhost:9600
  - service: http_status:404
EOF

# 5. Levantar containers + tunnel
docker compose -f docker-compose.sandbox.yml up -d
cloudflared tunnel run cerulean-sandbox
```

## Qué se expone

| Servicio | Puerto local | Qué es |
|---|---|---|
| Block Explorer | :5173 | Landing + dashboard + demo 5 pasos |
| Cerulean Voto | :5174 | Votación electrónica |
| API | :9600 | Nodo Cerulean (PQC, RocksDB, permissive ACL) |

## Arquitectura

```
Tu Mac
├── Docker Compose
│   ├── cerulean-sandbox-node    (:9600 → :8080)
│   ├── cerulean-sandbox-explorer (:5173 → nginx :80 → proxy /api → node)
│   └── cerulean-sandbox-voto    (:5174 → nginx :80 → proxy /api → node)
│
└── cloudflared
    ├── *.trycloudflare.com → localhost:5173
    ├── *.trycloudflare.com → localhost:5174
    └── *.trycloudflare.com → localhost:9600
```

Cloudflare solo enruta tráfico. No tiene acceso a datos ni claves. El nodo corre en tu máquina.

## Limitaciones

- **No es producción.** Es para demos y evaluación.
- **Datos efímeros.** Docker volumes persisten entre reinicios, pero un `docker compose down -v` borra todo.
- **Single node.** No hay replicación ni tolerancia a fallas.
- **Tu máquina debe estar encendida.** Si la apagas, el sandbox se cae.
- **Quick tunnels cambian de URL** cada vez que reinicias. Para URLs estables, usa dominio propio.
