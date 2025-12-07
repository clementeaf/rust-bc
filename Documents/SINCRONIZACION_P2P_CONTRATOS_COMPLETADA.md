# Sincronización P2P de Contratos - Completada

## Resumen

Se ha implementado exitosamente la sincronización P2P de smart contracts entre nodos de la red blockchain. Los contratos ahora se distribuyen automáticamente a todos los peers conectados cuando se despliegan o actualizan.

## Funcionalidades Implementadas

### 1. Mensajes P2P para Contratos

Se agregaron nuevos tipos de mensajes al enum `Message` en `src/network.rs`:

- `GetContracts`: Solicita todos los contratos de un peer
- `Contracts(Vec<SmartContract>)`: Respuesta con la lista de contratos
- `NewContract(SmartContract)`: Notificación de un nuevo contrato desplegado
- `UpdateContract(SmartContract)`: Notificación de actualización de un contrato

### 2. Broadcast Automático

Cuando se despliega un contrato o se ejecuta una función que modifica su estado:

- **Al desplegar**: Se envía un mensaje `NewContract` a todos los peers conectados
- **Al actualizar**: Se envía un mensaje `UpdateContract` a todos los peers conectados

Esto asegura que todos los nodos de la red tengan la información más reciente de los contratos.

### 3. Sincronización al Conectar

Cuando un nodo se conecta a otro peer:

1. Se intercambia información de versión (como antes)
2. Se sincronizan los bloques (como antes)
3. **NUEVO**: Se solicitan y sincronizan todos los contratos del peer

El proceso de sincronización:
- Compara contratos por dirección
- Agrega contratos nuevos que no existen localmente
- Actualiza contratos existentes si tienen un `updated_at` más reciente
- Guarda todos los contratos sincronizados en la base de datos

### 4. Manejo de Conflictos

El sistema maneja conflictos de contratos de la siguiente manera:

- **Contrato nuevo**: Se agrega automáticamente
- **Contrato existente**: Se compara `updated_at` y se actualiza solo si el recibido es más reciente
- **Contrato más antiguo**: Se ignora la actualización

Esto asegura que siempre se mantenga la versión más reciente del contrato en toda la red.

## Archivos Modificados

### `src/network.rs`

- Agregado `contract_manager` al struct `Node`
- Agregado método `set_contract_manager()` para configurar el gestor de contratos
- Agregados mensajes de contratos al enum `Message`
- Implementado manejo de mensajes de contratos en `process_message()`
- Agregada función `request_contracts()` para solicitar contratos a un peer
- Agregada función `broadcast_contract()` para enviar nuevos contratos
- Agregada función `broadcast_contract_update()` para enviar actualizaciones
- Modificado `connect_to_peer()` para sincronizar contratos automáticamente

### `src/api.rs`

- Modificado `deploy_contract()` para hacer broadcast del contrato después de desplegarlo
- Modificado `execute_contract_function()` para hacer broadcast de actualizaciones después de ejecutar funciones

### `src/main.rs`

- Configurado `contract_manager` en ambos nodos (para servidor P2P y para el servidor HTTP)

## Flujo de Sincronización

```
1. Nodo A despliega un contrato
   ↓
2. Nodo A guarda el contrato en BD
   ↓
3. Nodo A hace broadcast a todos sus peers (Nodo B, C, D...)
   ↓
4. Nodos B, C, D reciben el contrato
   ↓
5. Nodos B, C, D verifican si el contrato ya existe
   ↓
6. Si no existe, lo agregan y guardan en BD
   ↓
7. Si existe y es más reciente, lo actualizan y guardan en BD
```

## Pruebas

Se creó un script de prueba completo en `scripts/test_p2p_contracts_sync.sh` que verifica:

1. ✅ Despliegue de contrato en Nodo 1
2. ✅ Verificación de que el contrato no existe en Nodo 2 (antes de conectar)
3. ✅ Conexión de Nodo 2 a Nodo 1
4. ✅ Sincronización automática del contrato
5. ✅ Verificación de que el contrato existe en Nodo 2 después de conectar
6. ✅ Ejecución de función (mint) en Nodo 1
7. ✅ Sincronización de actualización en Nodo 2
8. ✅ Verificación de balance sincronizado

## Uso

### Para probar la sincronización:

1. Iniciar Nodo 1:
```bash
cargo run -- --api-port 8080 --p2p-port 5000
```

2. Iniciar Nodo 2 (en otra terminal):
```bash
cargo run -- --api-port 8081 --p2p-port 5001
```

3. Ejecutar el script de prueba:
```bash
./scripts/test_p2p_contracts_sync.sh
```

### Para conectar nodos manualmente:

```bash
# Desde el Nodo 2, conectar al Nodo 1
curl -X POST http://localhost:8081/api/v1/network/connect \
  -H "Content-Type: application/json" \
  -d '{"address": "127.0.0.1:5000"}'
```

Los contratos se sincronizarán automáticamente después de la conexión.

## Beneficios

1. **Consistencia**: Todos los nodos tienen la misma información de contratos
2. **Resistencia a fallos**: Si un nodo se desconecta y reconecta, recupera todos los contratos
3. **Distribución automática**: No se requiere intervención manual para distribuir contratos
4. **Actualizaciones en tiempo real**: Los cambios en contratos se propagan inmediatamente

## Próximos Pasos Sugeridos

- [ ] Sincronización incremental (solo contratos nuevos/modificados desde última sincronización)
- [ ] Validación de integridad de contratos (verificar hash)
- [ ] Compresión de mensajes para contratos grandes
- [ ] Métricas de sincronización (tiempo, cantidad de contratos sincronizados)

## Estado

✅ **COMPLETADO** - La sincronización P2P de contratos está completamente funcional y lista para producción.

