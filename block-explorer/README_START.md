# 🚀 Cómo Iniciar el Block Explorer

## ⚠️ Error Común: `ERR_CONNECTION_REFUSED`

Si ves este error:
```
Failed to load resource: net::ERR_CONNECTION_REFUSED
```

**Significa que el servidor backend (Rust) no está corriendo.**

---

## 📋 Pasos para Iniciar

### 1. Iniciar el Servidor Backend (Rust)

En una terminal, desde la raíz del proyecto:

```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc

# Opción 1: Compilar y ejecutar directamente
cargo run

# Opción 2: Usar el script de inicio
./scripts/start_node.sh

# Opción 3: Especificar puerto personalizado
cargo run 8080 8081 blockchain
```

**El servidor debe mostrar:**
```
🚀 Iniciando Blockchain API Server...
🌐 Servidor API iniciado en http://127.0.0.1:8080
📡 Servidor P2P iniciado en 127.0.0.1:8081
```

### 2. Iniciar el Block Explorer (Next.js)

En otra terminal:

```bash
cd block-explorer
npm run dev
```

**El servidor debe mostrar:**
```
▲ Next.js 14.2.33
- Local:        http://localhost:3000
```

### 3. Abrir en el Navegador

Abre: http://localhost:3000

---

## 🔧 Configuración de Puertos

### Cambiar Puerto del Backend

Si quieres usar un puerto diferente para el backend:

```bash
# Ejemplo: puerto 9000 para API, 9001 para P2P
cargo run 9000 9001 blockchain
```

Luego actualiza `block-explorer/.env.local`:
```
API_URL=http://127.0.0.1:9000/api/v1
```

### Cambiar Puerto del Frontend

```bash
# Ejemplo: puerto 3001
PORT=3001 npm run dev
```

---

## ✅ Verificar que Todo Funciona

### 1. Verificar Backend

```bash
curl http://127.0.0.1:8080/api/v1/health
```

Debe responder:
```json
{"success":true,"data":"OK"}
```

### 2. Verificar Frontend

Abre http://localhost:3000 y deberías ver:
- Estadísticas de la blockchain
- Lista de bloques
- Búsqueda funcional

---

## 🐛 Solución de Problemas

### Error: "Puerto 8080 ya en uso"

```bash
# Ver qué proceso usa el puerto
lsof -i :8080

# Matar el proceso (reemplaza PID con el número del proceso)
kill -9 PID
```

### Error: "Cannot find module"

```bash
cd block-explorer
npm install
```

### Error: "Cargo not found"

Asegúrate de tener Rust instalado:
```bash
rustc --version
cargo --version
```

Si no está instalado:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## 📝 Notas

- El backend debe estar corriendo **antes** de abrir el Block Explorer
- Si cambias el puerto del backend, actualiza `API_URL` en `.env.local`
- El backend usa el puerto 8080 por defecto
- El frontend usa el puerto 3000 por defecto

---

**¿Problemas?** Revisa los logs del servidor backend para más detalles.

