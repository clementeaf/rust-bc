# 🧪 Guía de Prueba - Red P2P con Múltiples Nodos

## 📋 Requisitos Previos

- El proyecto debe estar compilado: `cargo build --release`
- Python 3 instalado (para parsear JSON en los scripts)

## 🚀 Método 1: Script Automático (Recomendado)

Ejecuta el script de prueba automático que inicia 3 nodos y prueba la comunicación:

```bash
./test_multi_node.sh
```

Este script:
- ✅ Inicia 3 nodos en puertos diferentes
- ✅ Conecta los nodos entre sí
- ✅ Crea wallets y bloques
- ✅ Verifica la sincronización
- ✅ Muestra el estado final

## 🔧 Método 2: Manual (Paso a Paso)

### Paso 1: Iniciar Nodo 1

**Terminal 1:**
```bash
./start_node.sh 8080 8081 node1
```

O directamente:
```bash
cargo run --release 8080 8081 node1
```

Espera a ver: `✅ Blockchain cargada` y `🌐 Servidor API iniciado`

### Paso 2: Iniciar Nodo 2

**Terminal 2:**
```bash
./start_node.sh 8082 8083 node2
```

### Paso 3: Iniciar Nodo 3

**Terminal 3:**
```bash
./start_node.sh 8084 8085 node3
```

### Paso 4: Conectar los Nodos

**Terminal 4 (o nueva):**

```bash
# Conectar Nodo 1 → Nodo 2
curl -X POST http://127.0.0.1:8080/api/v1/peers/127.0.0.1:8083/connect

# Conectar Nodo 2 → Nodo 3
curl -X POST http://127.0.0.1:8082/api/v1/peers/127.0.0.1:8085/connect

# Verificar peers conectados en Nodo 1
curl http://127.0.0.1:8080/api/v1/peers

# Verificar peers conectados en Nodo 2
curl http://127.0.0.1:8082/api/v1/peers
```

### Paso 5: Crear Wallet y Bloque

```bash
# Crear wallet en Nodo 1
WALLET=$(curl -s -X POST http://127.0.0.1:8080/api/v1/wallets/create | python3 -c "import sys, json; print(json.load(sys.stdin)['data']['address'])")

echo "Wallet creado: $WALLET"

# Crear bloque coinbase en Nodo 1
curl -X POST http://127.0.0.1:8080/api/v1/blocks \
  -H "Content-Type: application/json" \
  -d "{\"transactions\":[{\"from\":\"0\",\"to\":\"$WALLET\",\"amount\":1000}]}"
```

### Paso 6: Verificar Sincronización

Espera 2-3 segundos y luego:

```bash
# Verificar bloques en cada nodo
echo "Nodo 1:"
curl -s http://127.0.0.1:8080/api/v1/chain/info | python3 -m json.tool

echo "Nodo 2:"
curl -s http://127.0.0.1:8082/api/v1/chain/info | python3 -m json.tool

echo "Nodo 3:"
curl -s http://127.0.0.1:8084/api/v1/chain/info | python3 -m json.tool
```

Todos deberían tener el mismo número de bloques.

## ✅ Pruebas a Realizar

### 1. Conexión entre Nodos
- [ ] Nodo 1 puede conectar a Nodo 2
- [ ] Nodo 2 puede conectar a Nodo 3
- [ ] Los peers aparecen en la lista

### 2. Sincronización
- [ ] Bloque creado en Nodo 1 se propaga a Nodo 2
- [ ] Bloque creado en Nodo 1 se propaga a Nodo 3
- [ ] Todos los nodos tienen el mismo número de bloques

### 3. Broadcast de Transacciones
- [ ] Transacción creada en un nodo se propaga a otros
- [ ] La transacción aparece en todos los nodos

### 4. Validación Distribuida
- [ ] Bloque inválido es rechazado por todos los nodos
- [ ] Solo bloques válidos se agregan a la cadena

## 🐛 Solución de Problemas

### Los nodos no se conectan
- Verifica que los puertos no estén en uso: `lsof -i :8081`
- Asegúrate de usar la dirección correcta: `127.0.0.1:8083` (no `localhost`)

### Los nodos no se sincronizan
- Espera unos segundos, la sincronización es asíncrona
- Verifica los logs de cada nodo para errores
- Intenta forzar sincronización: `curl -X POST http://127.0.0.1:8080/api/v1/sync`

### Error "Address already in use"
- Detén los procesos anteriores: `pkill -f "target/release/rust-bc"`
- Espera unos segundos y vuelve a intentar

## 📊 Ver Logs

Cada nodo imprime logs en su terminal. Busca:
- `📡 Nueva conexión desde:` - Conexión P2P establecida
- `✅ Bloque agregado` - Bloque recibido y validado
- `🔄 Sincronizando blockchain...` - Sincronización en progreso

## 🎯 Resultado Esperado

Al final de las pruebas, deberías ver:
- ✅ 3 nodos corriendo simultáneamente
- ✅ Nodos conectados entre sí
- ✅ Blockchain sincronizada en todos los nodos
- ✅ Bloques y transacciones propagándose automáticamente

## 🛑 Detener los Nodos

Presiona `Ctrl+C` en cada terminal, o ejecuta:
```bash
pkill -f "target/release/rust-bc"
```

