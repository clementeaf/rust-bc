# 🧪 Instrucciones para Ejecutar las Pruebas del Sistema

## 📋 Requisitos Previos

1. **Rust y Cargo instalados** y en el PATH
2. **Terminal** con acceso a comandos
3. **Dos terminales** (una para el servidor, otra para las pruebas)

---

## 🚀 Pasos para Ejecutar las Pruebas

### Paso 1: Iniciar el Servidor

**En la Terminal 1:**

```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
cargo run 8080 8081 blockchain
```

Deberías ver algo como:
```
[INFO] Iniciando servidor API en 127.0.0.1:8080
[INFO] Iniciando servidor P2P en 127.0.0.1:8081
[INFO] Blockchain cargada: X bloques
```

**Mantén esta terminal abierta** - el servidor debe seguir corriendo.

---

### Paso 2: Ejecutar las Pruebas

**En la Terminal 2 (nueva terminal):**

```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
./scripts/test_sistema_completo.sh
```

El script ejecutará automáticamente 12 pruebas:
1. ✅ Crear wallet
2. ✅ Obtener información de blockchain
3. ✅ Verificar cadena
4. ✅ Obtener estadísticas
5. ✅ Consultar mempool
6. ✅ Minar bloque con recompensa
7. ✅ Verificar balance después de minar
8. ✅ Crear segundo wallet
9. ✅ Crear transacción
10. ✅ Minar bloque con transacción
11. ✅ Verificar balances finales
12. ✅ Obtener todos los bloques

---

## 📊 Resultado Esperado

Si todo funciona correctamente, deberías ver:

```
🚀 Iniciando Prueba Completa del Sistema
==========================================

Verificando servidor... ✓ Servidor activo

📋 Ejecutando Pruebas
=====================

1. Creando wallet...
Probando Crear wallet... ✓ OK (HTTP 201)
   Wallet creado: abc123...

2. Obteniendo información de blockchain...
Probando Información de blockchain... ✓ OK (HTTP 200)

[... más pruebas ...]

==========================================
📊 Resumen de Pruebas
==========================================
Pruebas exitosas: 12
Pruebas fallidas: 0

✅ Todas las pruebas pasaron exitosamente
```

---

## 🔧 Solución de Problemas

### Problema: "Servidor no responde"

**Solución:**
1. Verifica que el servidor esté corriendo en la Terminal 1
2. Verifica que esté escuchando en el puerto 8080:
   ```bash
   curl http://127.0.0.1:8080/api/v1/chain/info
   ```
3. Si no responde, reinicia el servidor

### Problema: "cargo: command not found"

**Solución:**
1. Instala Rust: https://www.rust-lang.org/tools/install
2. Asegúrate de que cargo esté en tu PATH:
   ```bash
   source $HOME/.cargo/env
   ```

### Problema: "Permission denied" al ejecutar el script

**Solución:**
```bash
chmod +x scripts/test_sistema_completo.sh
```

### Problema: Puerto ya en uso

**Solución:**
```bash
# Usa puertos diferentes
cargo run 8082 8083 blockchain
# Y actualiza el script o usa:
API_URL="http://127.0.0.1:8082/api/v1" ./scripts/test_sistema_completo.sh
```

---

## 🧪 Pruebas Manuales Alternativas

Si prefieres probar manualmente, aquí tienes algunos comandos:

### 1. Crear Wallet
```bash
curl -X POST http://127.0.0.1:8080/api/v1/wallets/create
```

### 2. Ver Estadísticas
```bash
curl http://127.0.0.1:8080/api/v1/stats
```

### 3. Minar Bloque
```bash
curl -X POST http://127.0.0.1:8080/api/v1/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_address": "TU_DIRECCION", "max_transactions": 10}'
```

### 4. Ver Información de Blockchain
```bash
curl http://127.0.0.1:8080/api/v1/chain/info
```

### 5. Ver Mempool
```bash
curl http://127.0.0.1:8080/api/v1/mempool
```

---

## 📝 Notas

- El servidor debe estar corriendo antes de ejecutar las pruebas
- Las pruebas pueden tardar 1-2 minutos en completarse
- Si alguna prueba falla, revisa los mensajes de error
- Los resultados se muestran en tiempo real

---

## ✅ Checklist de Ejecución

- [ ] Rust y Cargo instalados
- [ ] Servidor iniciado en Terminal 1
- [ ] Servidor responde en http://127.0.0.1:8080
- [ ] Script de prueba ejecutado en Terminal 2
- [ ] Todas las pruebas pasaron

---

**¡Listo para probar!** 🚀

