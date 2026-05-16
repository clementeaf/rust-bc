# Cerulean Ledger — Block Explorer

Explorador de blockchain institucional con dashboard de integridad, identidad digital, governance y compliance. UI en espanol.

Parte del ecosistema [Cerulean Ledger](https://ceruleanledger.com).

## Stack

- React 19 + TypeScript 6 + Vite 8
- Tailwind CSS 4
- React Router 7
- Axios (HTTP client)

## Setup

```bash
npm install
npm run dev        # http://localhost:5173
```

Requiere un nodo Cerulean corriendo en `http://127.0.0.1:8080` (proxy automatico via Vite).

Para apuntar a otro nodo:

```bash
VITE_API_PROXY_TARGET=https://api.ceruleanledger.com npm run dev
```

## Scripts

| Comando | Descripcion |
|---------|-------------|
| `npm run dev` | Servidor de desarrollo (puerto 5173) |
| `npm run build` | Build de produccion (`tsc -b && vite build`) |
| `npm run preview` | Preview del build local |
| `npm run lint` | ESLint |

## Arquitectura

```
src/
├── pages/              # 20 paginas (lazy-loaded)
├── lib/
│   ├── api.ts          # Cliente HTTP completo (607 lineas)
│   ├── routes.ts       # Configuracion de rutas
│   └── format.ts       # Formatters (timeAgo, shortHash, fmtDate)
├── components/
│   ├── Layout.tsx      # Sidebar + header
│   ├── SearchBar.tsx   # Busqueda global
│   ├── ServerStatus.tsx# Indicador de conexion al nodo
│   ├── ServicesLayout.tsx # Layout para paginas de servicios
│   ├── FieldDemo.tsx   # Componente demo de campos
│   └── PageIntro.tsx   # Intro reutilizable para paginas
└── main.tsx
```

## Rutas

| Ruta | Pagina | Funcion |
|------|--------|---------|
| `/` | Landing | Hero, tesis, verticales, numeros, CTA |
| `/integridad` | Integridad | Dashboard institucional flagship (8 servicios, timeline, stress) |
| `/dashboard` | Home | Stats de red, bloques recientes, hub cards |
| `/demo` | Demo | Demo 5 pasos verificacion credencial RRHH |
| `/identity` | Identity | Lista de identidades + documentos firmados + drawer |
| `/credentials` | Credentials | Credenciales verificables con prueba criptografica |
| `/governance` | Governance | Propuestas, votacion, tally |
| `/compliance` | Compliance | Audit trail, filtros por accion/org, auto-refresh |
| `/chaincode-health` | ChaincodeHealth | Reportes sandbox por chaincode/version |
| `/wallets` | Wallets | Lista de wallets registradas |
| `/wallet/:address` | WalletDetail | Detalle y balance de wallet |
| `/transactions` | Transactions | Mempool y transacciones recientes |
| `/mining` | Mining | Interfaz de minado |
| `/contracts` | Contracts | Smart contracts desplegados |
| `/contract/:address` | ContractDetail | Detalle de contrato |
| `/validators` | Validators | Nodos validadores activos |
| `/staking` | Staking | Delegacion de stake |
| `/channels` | Channels | Canales DLT aislados |
| `/block/:hash` | BlockDetail | Detalle de bloque por hash |

## Paginas destacadas

### Integridad (flagship)

Dashboard institucional con:
- 8 tarjetas horizontales de servicios con drawers de detalle
- Tabla de reportes de integridad
- Timeline de eventos de seguridad
- Tarjetas de control vertical
- Grid de rendimiento stress test
- Auto-refresh cada 30s
- Print-friendly

### Identity

Modulo de identidad digital:
- Lista de DIDs registrados (`did:cerulean:*`)
- Panel de documentos firmados
- Drawer con prueba criptografica (Ed25519/ML-DSA-65)

### Demo

Flujo de verificacion credencial RRHH en 5 pasos compactos (single-card layout).

## Deploy

### Desarrollo con Docker

```bash
docker build -t cerulean-explorer .
docker run -p 5173:80 cerulean-explorer
```

Nginx proxea `/api/` hacia `http://node:8080` y `/api/v1/events` con SSE (long-lived).

### Produccion (S3 + CloudFront)

```bash
npm run build
aws s3 sync dist/ s3://ceruleanledger-explorer/ --delete
aws cloudfront create-invalidation --distribution-id E9QQPJR6KVMFH --paths "/*"
```

URL: https://ceruleanledger.com

## Variables de entorno (Vite)

| Variable | Default | Descripcion |
|----------|---------|-------------|
| `VITE_API_PROXY_TARGET` | `http://127.0.0.1:8080` | Backend API para proxy en dev |
| `VITE_DEV_SERVER_PORT` | `5173` | Puerto del dev server |
