# Problemas Pendientes - Sincronización P2P de Contratos

## Resumen

Aunque la sincronización P2P de contratos está implementada y funcional, existen varios problemas y mejoras pendientes que deberían abordarse para mejorar la robustez, seguridad y eficiencia del sistema.

## Problemas Identificados

### 1. ✅ CORREGIDO: Script de Prueba con Formato Incorrecto

**Problema:** El script `test_p2p_contracts_sync.sh` tenía varios problemas:
- Formato incorrecto para ejecutar funciones de contrato
- Endpoints incorrectos (`/wallets` en lugar de `/wallets/create`)
- Endpoint de minado incorrecto (`/mining/mine` en lugar de `/mine`)
- Endpoint de conexión incorrecto (`/network/connect` en lugar de `/peers/{address}/connect`)

**Estado:** ✅ Corregido

### 2. ⚠️ Sincronización Bidireccional Incompleta

**Problema:** Similar al problema de bloques, la sincronización solo funciona cuando **nosotros** nos conectamos a un peer. Si un peer se conecta a nosotros con contratos nuevos, no se sincronizan automáticamente.

**Impacto:**
- Si Nodo A tiene contratos y Nodo B se conecta a Nodo A, Nodo B no recibe los contratos automáticamente
- Solo funciona cuando Nodo B se conecta activamente a Nodo A

**Solución Sugerida:**
- Cuando un peer se conecta a nosotros, también deberíamos sincronizar contratos
- Agregar lógica en `handle_connection` para solicitar contratos cuando recibimos una conexión entrante

**Prioridad:** Media

### 3. ⚠️ Sin Validación de Integridad

**Problema:** Los contratos recibidos no se validan con hash o firma. Un nodo malicioso podría enviar contratos corruptos o modificados.

**Impacto:**
- Posible corrupción de datos
- Vulnerabilidad a ataques de nodos maliciosos
- No hay garantía de que el contrato recibido sea el mismo que se envió

**Solución Sugerida:**
- Agregar hash de validación a los contratos
- Verificar hash al recibir contratos
- Rechazar contratos con hash inválido

**Prioridad:** Alta (Seguridad)

### 4. ⚠️ Sin Manejo de Errores/Reintentos en Broadcast

**Problema:** Si un peer falla al recibir un contrato (por ejemplo, está desconectado), el error solo se registra pero no hay reintentos.

**Impacto:**
- Contratos pueden no llegar a todos los peers
- No hay garantía de entrega
- Pérdida de sincronización si un peer se desconecta temporalmente

**Solución Sugerida:**
- Implementar sistema de reintentos con backoff exponencial
- Mantener cola de contratos pendientes para peers desconectados
- Reintentar cuando el peer se reconecte

**Prioridad:** Media

### 5. ⚠️ Posibles Race Conditions

**Problema:** Si dos nodos actualizan el mismo contrato simultáneamente, puede haber conflictos. El sistema usa `updated_at` para resolver conflictos, pero no hay lock distribuido.

**Escenario:**
1. Nodo A y Nodo B ejecutan funciones en el mismo contrato al mismo tiempo
2. Ambos actualizan `updated_at` con timestamps muy cercanos
3. Puede haber inconsistencias en qué actualización se acepta

**Impacto:**
- Posibles inconsistencias temporales
- Pérdida de actualizaciones si los timestamps son idénticos

**Solución Sugerida:**
- Usar timestamps con mayor precisión (nanosegundos)
- Agregar número de secuencia a las actualizaciones
- Implementar consenso para actualizaciones conflictivas

**Prioridad:** Media-Alta

### 6. ⚠️ Sin Validación de Permisos

**Problema:** No se verifica que el `owner` del contrato sea el correcto cuando se recibe una actualización. Un nodo podría enviar una actualización de un contrato que no le pertenece.

**Impacto:**
- Vulnerabilidad de seguridad
- Posible manipulación de contratos por nodos no autorizados

**Solución Sugerida:**
- Validar que el `owner` del contrato no cambió (a menos que sea una transferencia autorizada)
- Verificar permisos antes de aceptar actualizaciones
- Rechazar actualizaciones de contratos con `owner` diferente

**Prioridad:** Alta (Seguridad)

### 7. ⚠️ Conexiones No Persistentes

**Problema:** Cada broadcast crea una nueva conexión TCP, lo cual es ineficiente. Además, no hay delay como en el broadcast de bloques, lo que puede causar que el peer no procese el mensaje antes de que se cierre la conexión.

**Impacto:**
- Ineficiencia en el uso de recursos
- Posible pérdida de mensajes si el peer no procesa rápido

**Solución Sugerida:**
- Agregar delay similar al broadcast de bloques (100ms)
- Considerar conexiones persistentes para peers activos
- Implementar pool de conexiones

**Prioridad:** Baja

### 8. ⚠️ Sin Sincronización Incremental

**Problema:** Cuando se solicitan contratos, se envían **todos** los contratos, incluso si ya están sincronizados. No hay sincronización incremental basada en timestamps.

**Impacto:**
- Ineficiencia en el uso de ancho de banda
- Tiempo de sincronización más largo de lo necesario

**Solución Sugerida:**
- Implementar sincronización incremental
- Solicitar solo contratos nuevos/modificados desde última sincronización
- Usar timestamp de última sincronización

**Prioridad:** Baja (Optimización)

### 9. ⚠️ Sin Métricas de Sincronización

**Problema:** No hay métricas sobre el proceso de sincronización (tiempo, cantidad de contratos sincronizados, errores, etc.).

**Impacto:**
- Difícil diagnosticar problemas
- No hay visibilidad del estado de sincronización

**Solución Sugerida:**
- Agregar métricas de sincronización
- Logs más detallados
- Endpoint de API para consultar estado de sincronización

**Prioridad:** Baja

## Resumen de Prioridades

| Problema | Prioridad | Estado |
|----------|-----------|--------|
| Script de prueba incorrecto | Alta | ✅ Corregido |
| Validación de integridad | Alta | ⚠️ Pendiente |
| Validación de permisos | Alta | ⚠️ Pendiente |
| Race conditions | Media-Alta | ⚠️ Pendiente |
| Sincronización bidireccional | Media | ⚠️ Pendiente |
| Manejo de errores/reintentos | Media | ⚠️ Pendiente |
| Conexiones no persistentes | Baja | ⚠️ Pendiente |
| Sincronización incremental | Baja | ⚠️ Pendiente |
| Métricas de sincronización | Baja | ⚠️ Pendiente |

## Conclusión

La sincronización P2P de contratos está **funcional** pero tiene varias áreas de mejora, especialmente en:
- **Seguridad**: Validación de integridad y permisos
- **Robustez**: Manejo de errores y race conditions
- **Eficiencia**: Sincronización incremental y conexiones persistentes

Se recomienda abordar primero los problemas de **alta prioridad** relacionados con seguridad antes de pasar a optimizaciones.

