# Instrucciones para Ejecutar Pruebas de Seguridad

## Problema Identificado

El script de pruebas de seguridad se queda sin respuesta porque:
1. **El servidor no está corriendo** - El script verifica esto y se detiene correctamente
2. **El mining toma tiempo** - Incluso con dificultad 1, puede tomar varios segundos

## Solución: Ejecutar en Dos Terminales

### Terminal 1: Iniciar el Servidor
```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
source ~/.cargo/env
DIFFICULTY=1 cargo run --release 8080 8081 blockchain
```

Espera a ver el mensaje:
```
🌐 Servidor API iniciado en http://127.0.0.1:8080
```

### Terminal 2: Ejecutar Pruebas de Seguridad
```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
./scripts/run_security_tests.sh
```

O directamente:
```bash
./scripts/test_security_attacks.sh
```

## Qué Hace el Script de Pruebas

El script ejecuta 7 tipos de pruebas de seguridad:

1. **Ataque de Doble Gasto** - Intenta gastar el mismo saldo dos veces
2. **Ataque de Saldo Insuficiente** - Intenta enviar más de lo disponible
3. **Ataque de Spam** - Envía 100+ transacciones rápidamente
4. **Ataque de Rate Limiting** - Envía 200+ requests para probar límites
5. **Ataque de Firma Inválida** - Intenta usar firmas falsas
6. **Ataque de Carga Extrema** - 500+ requests simultáneos
7. **Validación de Cadena** - Verifica integridad de la blockchain

## Tiempo Estimado

- **Con servidor corriendo**: ~2-5 minutos
- **Depende de**: Velocidad del mining (dificultad 1 es rápido)

## Si el Script se Queda Colgado

1. Verifica que el servidor esté corriendo: `curl http://localhost:8080/api/v1/health`
2. Verifica que no haya procesos bloqueados: `ps aux | grep rust-bc`
3. Si es necesario, reinicia el servidor

## Resultado Esperado

Todas las pruebas deben pasar (✅) para considerar el sistema seguro:
- ✅ Doble gasto: Sistema rechazó correctamente
- ✅ Saldo insuficiente: Sistema rechazó correctamente
- ✅ Spam: Sistema limitó correctamente
- ✅ Rate limiting: Sistema aplicó límites
- ✅ Firma inválida: Sistema rechazó correctamente
- ✅ Carga extrema: Sistema manejó correctamente
- ✅ Validación de cadena: Cadena es válida

