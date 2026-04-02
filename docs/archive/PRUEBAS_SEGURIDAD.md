# Pruebas de Seguridad y Ataques Agresivos

## Objetivo

Este documento describe las pruebas de seguridad implementadas para validar la robustez del sistema blockchain ante diversos tipos de ataques.

## Tipos de Ataques Probados

### 1. Ataque de Doble Gasto
- **Objetivo**: Intentar gastar el mismo saldo dos veces
- **Prueba**: Crear dos transacciones simultáneas con el mismo saldo
- **Resultado Esperado**: Solo una transacción debe ser aceptada, la segunda debe ser rechazada

### 2. Ataque de Saldo Insuficiente
- **Objetivo**: Intentar enviar más fondos de los disponibles
- **Prueba**: Crear transacción con amount mayor al balance disponible
- **Resultado Esperado**: Transacción debe ser rechazada con error de saldo insuficiente

### 3. Ataque de Spam de Transacciones
- **Objetivo**: Sobre cargar el sistema con muchas transacciones
- **Prueba**: Enviar 100+ transacciones rápidamente
- **Resultado Esperado**: Sistema debe limitar y procesar solo las válidas

### 4. Ataque de Rate Limiting
- **Objetivo**: Intentar sobrecargar el API con requests masivos
- **Prueba**: Enviar 200+ requests rápidamente
- **Resultado Esperado**: Sistema debe aplicar rate limiting (HTTP 429)

### 5. Ataque de Firma Inválida
- **Objetivo**: Intentar crear transacciones con firmas falsas
- **Prueba**: Enviar transacción con firma inválida o corrupta
- **Resultado Esperado**: Transacción debe ser rechazada

### 6. Ataque de Carga Extrema
- **Objetivo**: Probar la resistencia del sistema bajo carga extrema
- **Prueba**: 500+ requests simultáneos
- **Resultado Esperado**: Sistema debe mantener >80% de éxito

### 7. Ataque de Validación de Cadena
- **Objetivo**: Verificar que la cadena siempre sea válida
- **Prueba**: Verificar integridad de la cadena después de operaciones
- **Resultado Esperado**: Cadena debe ser siempre válida

## Ejecución de Pruebas

```bash
# Asegúrate de que el servidor esté corriendo
DIFFICULTY=1 cargo run --release 8080 8081 blockchain

# En otra terminal, ejecuta las pruebas de seguridad
./scripts/test_security_attacks.sh
```

## Interpretación de Resultados

- **✅ PASS**: El sistema resistió correctamente el ataque
- **❌ FAIL**: El sistema fue vulnerable al ataque (requiere corrección)

## Nivel de Seguridad Requerido

Para un sistema de producción con alto nivel de rentabilidad, se requiere:
- **100% de pruebas pasando**: Todas las pruebas de seguridad deben pasar
- **Resistencia a ataques conocidos**: Sistema debe resistir todos los ataques comunes
- **Validación estricta**: Todas las transacciones deben ser validadas completamente
- **Rate limiting activo**: Protección contra sobrecarga
- **Prevención de doble gasto**: Múltiples capas de validación

## Mejoras Continuas

Las pruebas de seguridad deben ejecutarse:
- Antes de cada release
- Después de cambios en validación
- Como parte de CI/CD
- Antes de despliegues a producción

## Notas Importantes

1. **No comprometer seguridad por velocidad**: El sistema debe ser seguro primero, rápido segundo
2. **Validación completa**: Todas las validaciones deben ejecutarse, sin atajos
3. **Logging de ataques**: Los intentos de ataque deben ser registrados para análisis
4. **Monitoreo continuo**: El sistema debe monitorearse en producción para detectar nuevos tipos de ataques

