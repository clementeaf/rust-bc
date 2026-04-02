# Consumo de Recursos - Aclaración

## ⚠️ Importante: Este Proyecto NO Usa GPU

Este proyecto de blockchain **NO utiliza GPU** en absoluto. Todo el procesamiento es en **CPU**.

### ¿Por qué puede parecer que usa GPU?

1. **Mining Intensivo de CPU**: El Proof of Work (SHA256) es muy intensivo en CPU
2. **Reporte del Sistema**: Algunos monitores de sistema pueden reportar incorrectamente el uso intensivo de CPU como GPU
3. **Calentamiento**: El uso intensivo de CPU puede hacer que el sistema se caliente, similar a cuando se usa GPU

## Consumo Actual

- **CPU**: ~90% (durante mining activo)
- **GPU**: 0% (no se usa)
- **RAM**: Mínimo (~5-10 MB)

## ¿Qué Está Consumiendo CPU?

El proceso `rust-bc` está ejecutando **mining de bloques** que requiere:
- Calcular millones de hashes SHA256 por segundo
- Buscar un nonce que cumpla con la dificultad
- Esto es **intensivo en CPU**, no GPU

## Soluciones para Reducir Consumo

### 1. Detener el Servidor Cuando No Se Use
```bash
pkill -f "rust-bc|cargo run"
```

### 2. Usar Dificultad Mínima (Solo para Pruebas)
```bash
DIFFICULTY=1 cargo run --release 8080 8081 blockchain
```

### 3. Limitar el Mining
- No ejecutar múltiples requests de `/mine` simultáneamente
- Esperar a que termine un bloque antes de minar el siguiente

### 4. Usar Nice para Reducir Prioridad
```bash
nice -n 19 DIFFICULTY=1 cargo run --release 8080 8081 blockchain
```

### 5. Limitar CPU con cgroups (Linux) o Activity Monitor (macOS)
- En macOS: Activity Monitor > CPU > Limitar proceso
- En Linux: `cpulimit -l 50 -p <PID>`

## Verificar Qué Está Usando GPU

Si realmente hay algo usando GPU, verifica:

### macOS
```bash
# Ver procesos usando GPU
sudo powermetrics --samplers gpu_power -i 1000 | grep -i gpu

# O usar Activity Monitor > Window > GPU History
```

### Linux
```bash
# Ver procesos usando GPU
nvidia-smi  # Para NVIDIA
# O
radeontop   # Para AMD
```

## Recomendaciones

1. **Para Desarrollo/Pruebas**: Usar `DIFFICULTY=1` (mining rápido, menos CPU)
2. **Para Producción**: Usar `DIFFICULTY=4` pero limitar la frecuencia de mining
3. **Cuando No Se Use**: Detener el servidor completamente
4. **Monitoreo**: Usar `htop` o Activity Monitor para ver el consumo real de CPU

## Nota Técnica

El algoritmo SHA256 usado en este proyecto es:
- **CPU-only**: No hay aceleración por GPU
- **Single-threaded**: Cada bloque se mina en un solo thread
- **Intensivo**: Requiere muchos cálculos para encontrar el nonce correcto

Si necesitas reducir el consumo, la mejor opción es **detener el servidor** cuando no lo estés usando para pruebas.

