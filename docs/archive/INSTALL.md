# Instalación y Pruebas

## Instalación de Rust

Si no tienes Rust instalado, puedes instalarlo fácilmente:

### macOS / Linux
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Windows
Descarga e instala desde: https://rustup.rs/

### Verificar instalación
```bash
rustc --version
cargo --version
```

## Compilación y Ejecución

### Compilar el proyecto
```bash
cargo build --release
```

### Ejecutar el programa
```bash
cargo run
```

### Ejecutar tests unitarios
```bash
cargo test
```

### Ejecutar script de pruebas completo
```bash
./test.sh
```

## Pruebas Manuales

Una vez que ejecutes `cargo run`, prueba las siguientes funcionalidades:

1. **Ver cadena completa** (opción 2)
   - Debe mostrar el bloque génesis ya minado
   - Verifica que el hash comience con 4 ceros (dificultad 4)

2. **Minar nuevo bloque** (opción 1)
   - Ingresa cualquier texto como datos
   - Observa el tiempo de minado
   - Verifica que el hash comience con 4 ceros

3. **Verificar cadena** (opción 3)
   - Debe mostrar que la cadena es válida
   - Minar más bloques y verificar nuevamente

4. **Ver cadena después de minar**
   - Debe mostrar todos los bloques encadenados
   - Cada bloque debe tener el hash del anterior como `previous_hash`

## Verificación de Requisitos

- ✅ Bloque génesis: Se crea automáticamente al iniciar
- ✅ Creación de bloques: Opción 1 del menú
- ✅ Proof of Work: Dificultad 4 (4 ceros al inicio del hash)
- ✅ Verificación: Opción 3 del menú
- ✅ CLI interactivo: Menú completo funcional
- ✅ Menos de 300 líneas: Verificado

