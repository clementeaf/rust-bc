# 🌐 Red P2P - Guía Rápida

## 🚀 Inicio Rápido

### Prueba Automática (2 Nodos)
```bash
./test_p2p_simple.sh
```

### Prueba Completa (3 Nodos)
```bash
./test_multi_node.sh
```

## 📖 Uso Manual

### Iniciar un Nodo

```bash
# Sintaxis: cargo run --release <api_port> <p2p_port> <db_name>
cargo run --release 8080 8081 node1
```

### Conectar Nodos

```bash
# Desde Nodo 1, conectar a Nodo 2
curl -X POST http://127.0.0.1:8080/api/v1/peers/127.0.0.1:8083/connect
```

### Ver Peers Conectados

```bash
curl http://127.0.0.1:8080/api/v1/peers
```

## 🎯 Características

- ✅ **Conexión P2P**: Nodos se conectan entre sí
- ✅ **Sincronización**: Blockchain se sincroniza automáticamente
- ✅ **Broadcast**: Bloques y transacciones se propagan
- ✅ **Validación**: Cada nodo valida independientemente

## 📊 Endpoints P2P

- `GET /api/v1/peers` - Lista de peers conectados
- `POST /api/v1/peers/{address}/connect` - Conectar a un peer

## 🔧 Configuración

Los puertos se pueden configurar de 3 formas:

1. **Argumentos de línea de comandos:**
   ```bash
   cargo run --release 8080 8081 node1
   ```

2. **Variables de entorno:**
   ```bash
   export API_PORT=8080
   export P2P_PORT=8081
   export DB_NAME=node1
   cargo run --release
   ```

3. **Valores por defecto:**
   - API: 8080
   - P2P: 8081
   - DB: blockchain.db

## 🐛 Solución de Problemas

### Puerto en uso
```bash
pkill -f "target/release/rust-bc"
```

### Ver logs
```bash
tail -f /tmp/node1.log
```

### Verificar conexión
```bash
curl http://127.0.0.1:8080/api/v1/chain/info
```

## 📚 Más Información

Ver [GUIA_PRUEBA_P2P.md](GUIA_PRUEBA_P2P.md) para guía detallada.

