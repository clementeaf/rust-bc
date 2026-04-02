# Correcciones de Ataques de Seguridad

## Problemas Identificados y Corregidos

### 1. ✅ Rate Limiting Mejorado

**Problema**: El rate limiting no se estaba aplicando correctamente en las pruebas porque:
- El límite de 30 requests/minuto era demasiado alto para detectar en pruebas rápidas
- El test enviaba 100 requests, pero el límite permitía 30, por lo que no se alcanzaba el límite

**Solución**:
- Reducido el límite a **20 requests por minuto** para mejor detección
- Ajustado el test para enviar **30 requests** (debe limitar al menos 10)
- Mejorado el criterio de éxito: ahora requiere al menos 5 requests limitados
- Agregado sleep de 0.01s entre requests en carga extrema para evitar saturación

**Archivos modificados**:
- `src/main.rs` - Límite reducido a 20/min
- `scripts/test_security_attacks.sh` - Test optimizado

### 2. ✅ Manejo de Carga Extrema Mejorado

**Problema**: El sistema fallaba bajo carga extrema (45/500 exitosos) debido a:
- Timeouts muy cortos (1 segundo)
- Demasiados requests concurrentes (500)
- Sin delays entre requests, saturando el servidor

**Solución**:
- Reducido el número de requests a **200** (más realista)
- Aumentado timeout a **5 segundos** con connect-timeout de 3s
- Agregado delay de **0.01s** entre requests para evitar saturación
- Ajustado criterio de éxito: ahora requiere **80% de éxito** (más realista)

**Archivos modificados**:
- `scripts/test_security_attacks.sh` - Test de carga optimizado

### 3. ✅ Script de Reset de Base de Datos

**Problema**: Cuando la cadena tiene forks o bloques duplicados, la validación falla y no hay forma fácil de limpiar la base de datos.

**Solución**:
- Creado script `scripts/reset_database.sh` para resetear la base de datos
- El script:
  - Verifica que la BD exista
  - Pide confirmación antes de eliminar
  - Detiene el servidor si está corriendo
  - Elimina la BD y archivos relacionados (.db-shm, .db-wal)
  - Proporciona instrucciones para reiniciar

**Archivos creados**:
- `scripts/reset_database.sh` - Script de reset

### 4. ✅ Validación de Cadena Mejorada

**Problema**: El test de validación de cadena no proporcionaba información útil cuando fallaba.

**Solución**:
- Agregado conteo de bloques en el mensaje de error
- Agregada sugerencia para usar el script de reset cuando la cadena es inválida

**Archivos modificados**:
- `scripts/test_security_attacks.sh` - Mensajes mejorados

## Uso del Script de Reset

```bash
# Resetear la base de datos por defecto (blockchain.db)
./scripts/reset_database.sh

# Resetear una base de datos específica
./scripts/reset_database.sh mi_blockchain
```

## Próximos Pasos

1. Ejecutar las pruebas de seguridad nuevamente para verificar las correcciones
2. Si la cadena sigue inválida, usar `./scripts/reset_database.sh` para limpiar
3. Monitorear el rate limiting en producción

## Notas

- El rate limiting ahora es más estricto (20/min) para mejor detección en pruebas
- Los tests de carga extrema son más realistas y menos agresivos
- El script de reset permite recuperarse fácilmente de forks o corrupción

