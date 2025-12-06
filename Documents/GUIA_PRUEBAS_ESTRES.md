# 🔥 Guía de Pruebas de Estrés y Carga Crítica

## 📋 Objetivo

Identificar puntos de falla, colapso y límites del sistema mediante pruebas exhaustivas de:
- **Estrés**: Carga puntual intensa
- **Carga**: Carga prolongada
- **Casos límite**: Valores extremos y edge cases
- **Concurrencia**: Múltiples requests simultáneos
- **Recuperación**: Comportamiento después de errores

---

## 🚀 Ejecución de Pruebas

### Pruebas Individuales

#### 1. Pruebas Críticas (Casos Límite)
```bash
./scripts/test_critical.sh
```
**Duración**: ~2-3 minutos  
**Qué prueba**:
- Valores extremos (amounts muy grandes)
- Strings muy largos
- JSON malformado
- Endpoints inexistentes
- Métodos HTTP incorrectos
- Consistencia de caché
- Recuperación después de errores
- Límites de rate limiting

#### 2. Pruebas de Estrés (Carga Puntual)
```bash
./scripts/test_stress.sh
```
**Duración**: ~3-5 minutos  
**Qué prueba**:
- Rate limiting (100+ requests)
- Concurrencia (50 requests simultáneos)
- Carga alta (200 requests rápidos)
- Creación concurrente de wallets
- Transacciones concurrentes
- Consultas de balance concurrentes
- Memory leak detection
- Timeout handling
- Stress test final (todo junto)

#### 3. Pruebas de Carga Prolongada
```bash
./scripts/test_load.sh
```
**Duración**: 60 segundos (configurable)  
**Qué prueba**:
- Carga sostenida por 60 segundos
- 10 workers concurrentes
- Múltiples endpoints simultáneos
- Métricas de rendimiento (RPS, tasa de éxito)
- Degradación de rendimiento

### Suite Completa

Ejecutar todas las pruebas en secuencia:
```bash
./scripts/run_all_stress_tests.sh
```

**Duración total**: ~10-15 minutos

---

## 📊 Métricas Monitoreadas

### Durante las Pruebas

1. **HTTP Status Codes**
   - `200/201`: Éxito
   - `400/422`: Validación (esperado para datos inválidos)
   - `404`: No encontrado (esperado para endpoints inexistentes)
   - `429`: Rate limited (esperado después de 100 requests/min)
   - `500`: Error del servidor (CRÍTICO - no debería ocurrir)
   - `000`: Timeout/Error de conexión (CRÍTICO)

2. **Rendimiento**
   - Requests por segundo (RPS)
   - Tiempo de respuesta
   - Tasa de éxito
   - Tasa de errores

3. **Recursos**
   - Uso de memoria (monitorear manualmente)
   - Uso de CPU (monitorear manualmente)
   - Conexiones de base de datos

---

## 🎯 Puntos de Falla a Identificar

### 1. Rate Limiting
- ✅ ¿Se aplica correctamente después de 100 requests/min?
- ✅ ¿Se resetea correctamente?
- ✅ ¿Afecta a todos los endpoints?

### 2. Concurrencia
- ✅ ¿Múltiples requests simultáneos funcionan?
- ✅ ¿Hay race conditions?
- ✅ ¿Los locks se liberan correctamente?

### 3. Caché
- ✅ ¿El caché es consistente bajo carga?
- ✅ ¿Se invalida correctamente?
- ✅ ¿Hay memory leaks?

### 4. Base de Datos
- ✅ ¿Múltiples escrituras simultáneas funcionan?
- ✅ ¿WAL mode funciona correctamente?
- ✅ ¿Los índices mejoran el rendimiento?

### 5. Validación
- ✅ ¿Datos inválidos se rechazan?
- ✅ ¿Valores extremos se manejan?
- ✅ ¿JSON malformado se rechaza?

### 6. Recuperación
- ✅ ¿El sistema se recupera después de errores?
- ✅ ¿No hay degradación de rendimiento?
- ✅ ¿No hay memory leaks?

---

## 📈 Interpretación de Resultados

### ✅ Éxito
- Todas las pruebas pasan
- Tasa de éxito > 95%
- Sin errores 500
- Sin timeouts
- Rendimiento estable

### ⚠️ Advertencias
- Algunas pruebas fallan ocasionalmente
- Tasa de éxito 90-95%
- Algunos timeouts bajo carga extrema
- Degradación leve de rendimiento

### ❌ Fallos Críticos
- Múltiples pruebas fallan consistentemente
- Tasa de éxito < 90%
- Errores 500 frecuentes
- Timeouts frecuentes
- Degradación severa de rendimiento
- Memory leaks detectados

---

## 🔧 Solución de Problemas

### Si las Pruebas Fallan

1. **Revisar Logs del Servidor**
   ```bash
   # Ver logs en tiempo real
   cargo run --release 2>&1 | tee server.log
   ```

2. **Verificar Recursos**
   ```bash
   # Monitorear memoria y CPU
   top -p $(pgrep -f rust-bc)
   ```

3. **Revisar Base de Datos**
   ```bash
   # Verificar tamaño y estado
   ls -lh blockchain.db*
   sqlite3 blockchain.db "PRAGMA integrity_check;"
   ```

4. **Analizar Resultados**
   - Revisar archivos en `test_results_*/`
   - Buscar patrones de errores
   - Identificar endpoints problemáticos

### Problemas Comunes

#### Rate Limiting No Funciona
- Verificar que el middleware esté configurado
- Revisar logs para ver si se aplica
- Verificar configuración de límites

#### Errores 500 Frecuentes
- Revisar logs del servidor
- Verificar manejo de errores
- Buscar panics o unwraps sin manejo

#### Timeouts
- Verificar que el servidor esté respondiendo
- Revisar carga del sistema
- Verificar conexiones de base de datos

#### Memory Leaks
- Monitorear uso de memoria durante pruebas
- Buscar crecimiento constante
- Revisar cachés y estructuras de datos

---

## 📝 Resultados Esperados

### Pruebas Críticas
- ✅ 10/10 pruebas pasan
- ✅ Validación correcta de datos inválidos
- ✅ Caché consistente
- ✅ Recuperación después de errores

### Pruebas de Estrés
- ✅ 10/10 pruebas pasan
- ✅ Rate limiting funciona
- ✅ Concurrencia manejada correctamente
- ✅ Sin memory leaks

### Pruebas de Carga
- ✅ > 95% tasa de éxito
- ✅ RPS estable durante toda la prueba
- ✅ Sin degradación de rendimiento
- ✅ Sin errores 500

---

## 🎯 Próximos Pasos Después de las Pruebas

1. **Si hay fallos**:
   - Documentar los fallos encontrados
   - Priorizar por criticidad
   - Implementar correcciones
   - Re-ejecutar pruebas

2. **Si todo pasa**:
   - Aumentar límites de carga
   - Probar con más concurrencia
   - Probar con más duración
   - Documentar límites conocidos

3. **Optimizaciones**:
   - Identificar cuellos de botella
   - Optimizar endpoints lentos
   - Mejorar manejo de errores
   - Ajustar configuración

---

## 📚 Archivos de Resultados

Los resultados se guardan en:
- `test_results_critical/` - Resultados de pruebas críticas
- `test_results_stress/` - Resultados de pruebas de estrés
- `load_test_results_*.txt` - Resultados de pruebas de carga

Cada archivo contiene:
- Timestamp de ejecución
- Resultados de cada prueba
- Estadísticas finales
- Errores encontrados

---

**Última actualización**: Después de implementar mejoras v2.0

