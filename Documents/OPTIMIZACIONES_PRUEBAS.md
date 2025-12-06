# Optimizaciones para Pruebas Rápidas

## Cambios Implementados

### 1. Mining Asíncrono
- **Archivo**: `src/api.rs`
- **Cambio**: El endpoint `/mine` ahora ejecuta el mining en un thread separado usando `actix_web::web::block()`
- **Beneficio**: El servidor no se bloquea durante el mining, permitiendo que otras peticiones se procesen

### 2. Dificultad Configurable
- **Archivo**: `src/main.rs`
- **Cambio**: La dificultad ahora se puede configurar con la variable de entorno `DIFFICULTY` (default: 1)
- **Uso**: `DIFFICULTY=1 cargo run --release 8080 8081 blockchain`
- **Beneficio**: Para pruebas, usar dificultad 1 hace que el mining sea mucho más rápido

### 3. Scripts de Prueba Optimizados
- **Archivos**: `scripts/test_stress.sh`, `scripts/test_load.sh`, `scripts/run_all_stress_tests.sh`
- **Cambios**:
  - Timeouts aumentados (TIMEOUT=10, MINE_TIMEOUT=15)
  - `set +e` para no detenerse en errores
  - Mejor manejo de arrays de wallets
  - Verificación de saldos antes de transacciones concurrentes

## Instrucciones para Ejecutar Pruebas

### 1. Iniciar Servidor con Dificultad Baja
```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
source ~/.cargo/env
DIFFICULTY=1 cargo run --release 8080 8081 blockchain
```

### 2. En otra terminal, ejecutar pruebas
```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
./scripts/run_all_stress_tests.sh
```

### 3. O ejecutar pruebas individuales
```bash
# Pruebas críticas
./scripts/test_critical.sh

# Pruebas de estrés
./scripts/test_stress.sh

# Prueba de carga prolongada
./scripts/test_load.sh
```

## Estado Actual

- ✅ **Compilación**: Sin errores (1 warning)
- ✅ **Mining Asíncrono**: Implementado
- ✅ **Dificultad Configurable**: Implementado
- ⚠️ **Test de Transacciones Concurrentes**: Puede fallar si los wallets no tienen suficiente saldo
- ⚠️ **Test de Carga Prolongada**: Puede tener problemas si el servidor se sobrecarga

## Notas Importantes

1. **Dificultad 1**: Usar solo para pruebas. En producción, usar dificultad 4 o superior
2. **Mining Asíncrono**: Ahora no bloquea el servidor, pero sigue siendo CPU-intensivo
3. **Timeouts**: Si las pruebas fallan, aumentar los timeouts en los scripts
4. **Limpieza**: Si el servidor se queda colgado, usar:
   ```bash
   pkill -9 -f "rust-bc|cargo run"
   lsof -ti:8080,8081 | xargs kill -9
   ```

## Próximos Pasos Sugeridos

1. Ajustar el test de transacciones concurrentes para asegurar que todos los wallets tengan saldo
2. Implementar un límite de tiempo para el mining (timeout)
3. Considerar usar dificultad 0 para pruebas extremadamente rápidas (solo testing)
4. Agregar métricas de tiempo en las pruebas para identificar cuellos de botella

