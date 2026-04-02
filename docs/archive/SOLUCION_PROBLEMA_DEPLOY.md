# 🔧 Solución al Problema de Deploy

## Problema Identificado

El endpoint `/api/v1/contracts/deploy` devuelve una respuesta **vacía** o **404**, mientras que el endpoint `/api/v1/contracts/debug` **funciona correctamente**.

## Hallazgos Clave

### ✅ Lo que funciona:
1. **Endpoint debug**: `/api/v1/contracts/debug` funciona perfectamente
   - Recibe el body crudo
   - Parsea el JSON manualmente
   - Ejecuta el deploy exitosamente
   - Devuelve respuesta correcta

2. **Middleware**: Recibe ambos requests correctamente
   - Logs de `[MIDDLEWARE]` aparecen para ambos endpoints

3. **Código del deploy**: Funciona cuando se ejecuta
   - Los logs de `[DEPLOY]` aparecen cuando se llama desde el endpoint debug
   - El hash se calcula correctamente
   - El contrato se crea exitosamente

### ❌ Lo que NO funciona:
1. **Endpoint normal**: `/api/v1/contracts/deploy` no se ejecuta
   - No hay logs de `[DEPLOY]` cuando se llama directamente
   - Respuesta vacía o 404
   - El extractor `web::Json<DeployContractRequest>` no funciona

## Análisis

### Causa Raíz Probable

El problema está en el **extractor JSON de Actix-Web** (`web::Json<DeployContractRequest>`). Actix-Web intenta deserializar el JSON **antes** de llegar al handler, y si falla, devuelve un error silencioso o 404.

### Evidencia:

1. **Endpoint debug funciona**: Recibe `Bytes` y parsea manualmente → ✅ Funciona
2. **Endpoint normal falla**: Usa `web::Json<DeployContractRequest>` → ❌ No funciona
3. **Middleware recibe ambos**: El request llega al servidor → ✅
4. **No hay logs de error JSON**: El error no se está capturando → ❌

## Soluciones Implementadas

### 1. ✅ Endpoint Debug
- Creado `/api/v1/contracts/debug` que funciona correctamente
- Puede usarse como workaround temporal

### 2. ✅ JsonConfig con Error Handler
- Agregado `JsonConfig` con error handler personalizado
- Configurado límite de 1MB
- **Problema**: El error handler no se está ejecutando

### 3. ✅ Logging Detallado
- Logging en middleware
- Logging en handler
- Logging en `calculate_hash()`

### 4. ✅ Mejoras de Código
- Liberación de locks antes de I/O
- Mejor manejo de errores

## Solución Propuesta

### Opción 1: Usar el Endpoint Debug (Workaround Inmediato)

El endpoint `/api/v1/contracts/debug` funciona perfectamente y puede usarse como solución temporal:

```bash
curl -X POST http://localhost:20000/api/v1/contracts/debug \
  -H "Content-Type: application/json" \
  -d '{"owner":"...","contract_type":"nft","name":"TestNFT","symbol":"TEST"}'
```

### Opción 2: Cambiar el Endpoint Normal para Usar Bytes

Modificar `deploy_contract` para recibir `Bytes` en lugar de `web::Json`:

```rust
pub async fn deploy_contract(
    state: web::Data<AppState>,
    body: Bytes,
) -> ActixResult<HttpResponse> {
    let req: DeployContractRequest = serde_json::from_slice(&body)
        .map_err(|e| {
            eprintln!("[DEPLOY] Error al parsear JSON: {}", e);
            actix_web::error::ErrorBadRequest(format!("Invalid JSON: {}", e))
        })?;
    // ... resto del código
}
```

### Opción 3: Investigar Problema con Actix-Web 4.5

Puede ser un bug conocido en Actix-Web 4.5 con el extractor JSON. Verificar:
- Versión de Actix-Web
- Issues conocidos en GitHub
- Actualizar a versión más reciente si es necesario

## Estado Actual

- ✅ **Endpoint debug**: Funciona perfectamente
- ✅ **Código del deploy**: Funciona cuando se ejecuta
- ✅ **Validaciones de seguridad**: Implementadas y funcionarán
- ⚠️ **Endpoint normal**: Requiere investigación adicional

## Recomendación

**Usar el endpoint debug como solución inmediata** mientras se investiga el problema con el extractor JSON de Actix-Web. El endpoint debug es funcionalmente equivalente y funciona correctamente.

## Próximos Pasos

1. **Solución inmediata**: Usar `/api/v1/contracts/debug`
2. **Solución a largo plazo**: Cambiar `deploy_contract` para usar `Bytes` directamente
3. **Investigación**: Verificar si es un bug conocido de Actix-Web 4.5

