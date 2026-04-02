# 🧹 Plan de Limpieza del Proyecto

## Problemas Identificados

### 1. Documentación Excesiva (114 archivos .md en Documents/)
- **Problema**: Documentación duplicada, obsoleta y desorganizada
- **Impacto**: Dificulta encontrar información relevante
- **Solución**: Consolidar en archivos principales

### 2. Archivos de Test Temporales
- **Problema**: Múltiples directorios `test_*` y archivos `.db` en raíz
- **Impacto**: Desorden y confusión
- **Solución**: Mover a directorio `tests/data/` o eliminar

### 3. Código Muerto
- **Problema**: `src/database.rs` (809 líneas) - ya no se usa
- **Problema**: `src/blockchain.rs.broken` - archivo roto
- **Impacto**: Confusión y mantenimiento innecesario
- **Solución**: Eliminar

### 4. Archivos de Resultados
- **Problema**: `load_test_results_*.txt` en raíz
- **Impacto**: Desorden
- **Solución**: Mover a `tests/results/` o eliminar

### 5. node_modules no ignorados
- **Problema**: `block-explorer/node_modules/` y `sdk-js/node_modules/`
- **Impacto**: Tamaño innecesario del repo
- **Solución**: Agregar a .gitignore

## Acciones Recomendadas

### Fase 1: Eliminar Código Muerto (SEGURO)
- [ ] Eliminar `src/database.rs` (no se usa)
- [ ] Eliminar `src/blockchain.rs.broken` (archivo roto)
- [ ] Verificar que no haya referencias a estos archivos

### Fase 2: Limpiar Archivos Temporales (SEGURO)
- [ ] Eliminar archivos `test_*.db*` en raíz
- [ ] Eliminar directorios `test_*_blocks/`, `test_*_snapshots/`, etc.
- [ ] Eliminar `load_test_results_*.txt`
- [ ] Mover archivos de test necesarios a `tests/data/`

### Fase 3: Consolidar Documentación (REQUIERE REVISIÓN)
- [ ] Identificar documentos esenciales
- [ ] Consolidar información duplicada
- [ ] Mover documentos obsoletos a `Documents/archive/`
- [ ] Crear índice de documentación actualizado

### Fase 4: Mejorar .gitignore
- [ ] Agregar `node_modules/` para subproyectos
- [ ] Agregar `*.txt` para resultados de tests
- [ ] Verificar que `target/` esté ignorado

## Archivos a Mantener

### Documentación Esencial
- `README.md` - Documentación principal
- `ESTADO_ACTUAL_ROADMAP.md` - Roadmap actual
- `Documents/API_DOCUMENTATION.md` - Documentación de API
- `Documents/GUIA_USUARIO.md` - Guía de usuario

### Código Fuente
- Todos los archivos en `src/*.rs` (excepto los marcados para eliminar)
- `Cargo.toml` y `Cargo.lock`
- Tests en `tests/`

## Estadísticas Actuales

- **Archivos .md**: 683+ (114 en Documents/)
- **Líneas de código Rust**: ~12,500
- **Archivos de test temporales**: ~30+
- **Código muerto**: ~809 líneas (database.rs)

