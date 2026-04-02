# Tests de Seguridad Agresivos - Sistema de Billing

## 🛡️ Tests Implementados

Se han creado **12 tests de seguridad agresivos** que simulan ataques realistas y violentos contra el sistema de billing:

### 1. ✅ Ataque de Fuerza Bruta en API Keys
- **Ataque**: 1,000 intentos de adivinar API keys aleatorias
- **Objetivo**: Verificar que el sistema rechaza keys inválidas
- **Criterio de éxito**: 0 keys válidas encontradas de 1,000 intentos

### 2. ✅ Ataque de Bypass de Límites de Transacciones
- **Ataque**: Intentar realizar 150 transacciones con tier Free (límite: 100)
- **Objetivo**: Verificar que el sistema aplica límites correctamente
- **Criterio de éxito**: Máximo 100 transacciones registradas

### 3. ✅ Ataque de Rate Limiting Masivo
- **Ataque**: 200 requests rápidos en segundos
- **Objetivo**: Verificar que el rate limiting funciona
- **Criterio de éxito**: Al menos 50 requests limitados (HTTP 429)

### 4. ✅ Ataque de Manipulación de Contadores
- **Ataque**: Realizar 50 transacciones y verificar que se registran
- **Objetivo**: Verificar que los contadores no pueden manipularse
- **Criterio de éxito**: Contadores incrementan correctamente

### 5. ✅ Ataque de DoS con Requests Masivos
- **Ataque**: 1,000 requests simultáneos
- **Objetivo**: Verificar que el sistema no colapsa
- **Criterio de éxito**: Al menos 80% de requests exitosos (200 o 429)

### 6. ✅ Ataque de Keys Inválidas y Malformadas
- **Ataque**: 15 tipos diferentes de keys malformadas
- **Objetivo**: Verificar validación estricta de formato
- **Criterio de éxito**: Todas las keys inválidas rechazadas

### 7. ✅ Ataque de Keys Desactivadas
- **Ataque**: Desactivar key y intentar usarla
- **Objetivo**: Verificar que keys desactivadas no funcionan
- **Criterio de éxito**: Key desactivada rechazada (HTTP != 200)

### 8. ✅ Ataque Concurrente Masivo
- **Ataque**: 100 requests concurrentes simultáneos
- **Objetivo**: Verificar manejo de race conditions
- **Criterio de éxito**: Contadores correctos después de concurrencia

### 9. ✅ Ataque de Inyección en Headers
- **Ataque**: Intentos de inyección SQL, XSS, null bytes en headers
- **Objetivo**: Verificar sanitización de headers
- **Criterio de éxito**: Todos los intentos de inyección rechazados

### 10. ✅ Ataque de Timing Attack
- **Ataque**: Medir tiempos de respuesta para keys válidas vs inválidas
- **Objetivo**: Verificar que no se expone información por timing
- **Criterio de éxito**: Tiempos similares para keys válidas e inválidas

### 11. ✅ Ataque de Exhaustión de Límites
- **Ataque**: Intentar exceder límites sistemáticamente
- **Objetivo**: Verificar que límites se aplican correctamente
- **Criterio de éxito**: Al menos 50/150 requests rechazados después de límite

### 12. ✅ Ataque de Keys Duplicadas
- **Ataque**: Crear múltiples keys y verificar unicidad
- **Objetivo**: Verificar que no se generan keys duplicadas
- **Criterio de éxito**: Todas las keys generadas son únicas

## 🔒 Medidas de Seguridad Verificadas

### Validación de API Keys
- ✅ Hash SHA-256 (no se almacena la key original)
- ✅ Validación de formato estricta (`bc_` + 32 caracteres)
- ✅ Rechazo de keys vacías, null, undefined
- ✅ Protección contra inyección en headers

### Rate Limiting
- ✅ Límites por tier (Free: 10/min, Basic: 100/min, Pro: 1000/min)
- ✅ Ventana deslizante estricta (máx 5 req/seg)
- ✅ Protección contra DoS masivo

### Límites de Uso
- ✅ Límites de transacciones por mes
- ✅ Límites de wallets por tier
- ✅ Validación antes de procesar
- ✅ Contadores thread-safe

### Protección Contra Ataques
- ✅ Fuerza bruta: Keys de 32 caracteres aleatorios (UUID)
- ✅ Timing attacks: Validación constante
- ✅ Race conditions: Mutexes thread-safe
- ✅ Inyección: Sanitización de entrada

## 📊 Ejecución de Tests

```bash
# Iniciar servidor
DIFFICULTY=1 cargo run --release 8080 8081 blockchain

# Ejecutar tests (en otra terminal)
./scripts/test_billing_security.sh
```

## 🎯 Resultados Esperados

**Todos los tests deben pasar** para considerar el sistema 100% seguro:

- ✅ Fuerza bruta: 0/1000 keys válidas encontradas
- ✅ Bypass de límites: Límites aplicados correctamente
- ✅ Rate limiting: >50 requests limitados
- ✅ Manipulación: Contadores correctos
- ✅ DoS: >80% requests manejados
- ✅ Keys inválidas: 100% rechazadas
- ✅ Keys desactivadas: Rechazadas correctamente
- ✅ Concurrencia: Sin race conditions
- ✅ Inyección: 100% rechazados
- ✅ Timing: Sin exposición de información
- ✅ Exhaustión: Límites aplicados
- ✅ Duplicados: Keys únicas

## ⚠️ Notas Importantes

1. **Tests Agresivos**: Estos tests son extremadamente agresivos y pueden tomar varios minutos
2. **Recursos**: Los tests de DoS y concurrencia consumen recursos significativos
3. **Rate Limiting**: El rate limiting puede afectar tests consecutivos (esperar entre tests)
4. **Servidor**: El servidor debe estar corriendo antes de ejecutar tests

## 🔧 Mejoras Continuas

Si algún test falla, indica una vulnerabilidad que debe corregirse:

1. **Fuerza Bruta**: Aumentar longitud de keys o agregar rate limiting más estricto
2. **Bypass de Límites**: Revisar lógica de validación de límites
3. **Rate Limiting**: Ajustar límites o implementar ventana deslizante más estricta
4. **Manipulación**: Revisar thread-safety de contadores
5. **DoS**: Optimizar manejo de requests masivos
6. **Inyección**: Mejorar sanitización de headers
7. **Timing**: Implementar validación con tiempo constante
8. **Concurrencia**: Revisar locks y mutexes

## ✅ Conclusión

Estos tests proporcionan una **verificación exhaustiva** de la seguridad del sistema de billing contra ataques realistas y violentos. Un sistema que pasa todos estos tests puede considerarse **altamente seguro** para uso en producción.

