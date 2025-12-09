# Fix CORS - Block Explorer

## 🐛 Problema

El Block Explorer mostraba errores `Error CORS` al intentar conectarse al backend:

```
Failed to load resource: Error CORS
```

**Causa**: El servidor backend (Rust/Actix-Web) no tenía configurado CORS (Cross-Origin Resource Sharing), lo que bloqueaba las peticiones desde el frontend que corre en un puerto diferente (localhost:3000).

---

## ✅ Solución Implementada

### 1. Agregada Dependencia CORS

**Archivo**: `Cargo.toml`

```toml
actix-cors = "0.7"
```

### 2. Configurado Middleware CORS

**Archivo**: `src/main.rs`

```rust
use actix_cors::Cors;

// En HttpServer::new()
let cors = Cors::default()
    .allow_any_origin()      // Permite cualquier origen
    .allow_any_method()      // Permite cualquier método HTTP (GET, POST, etc.)
    .allow_any_header()      // Permite cualquier header
    .supports_credentials()  // Soporta credenciales
    .max_age(3600);          // Cache de preflight por 1 hora

App::new()
    .wrap(cors)  // Agregar CORS antes de otros middlewares
    .wrap(Compress::default())
    .wrap(RateLimitMiddleware::new(rate_limit_config.clone()))
    // ...
```

---

## 🔄 Aplicar el Fix

### Paso 1: Recompilar el Backend

```bash
cd /Users/clementefalcone/Desktop/personal/rust-bc
cargo build
```

### Paso 2: Reiniciar el Servidor

Si el servidor está corriendo, detenerlo (Ctrl+C) y reiniciarlo:

```bash
cargo run
```

O usar el script:

```bash
./scripts/start_node.sh
```

### Paso 3: Verificar

1. El servidor debe iniciar sin errores
2. El Block Explorer debe poder conectarse sin errores CORS
3. Las peticiones a `/api/v1/stats` y `/api/v1/blocks` deben funcionar

---

## 🔒 Seguridad

**Nota**: La configuración actual usa `.allow_any_origin()` que permite peticiones desde cualquier origen. Esto es adecuado para desarrollo, pero para producción deberías restringir los orígenes permitidos:

```rust
let cors = Cors::default()
    .allowed_origin("http://localhost:3000")  // Solo permitir frontend específico
    .allowed_origin("https://tu-dominio.com") // Y tu dominio de producción
    .allow_any_method()
    .allow_any_header()
    .supports_credentials()
    .max_age(3600);
```

O usar una lista de orígenes permitidos:

```rust
let allowed_origins = vec![
    "http://localhost:3000",
    "http://localhost:3001",
    "https://blockexplorer.tudominio.com",
];

let cors = Cors::default()
    .allowed_origins(&allowed_origins)
    .allow_any_method()
    .allow_any_header()
    .supports_credentials()
    .max_age(3600);
```

---

## ✅ Estado

- [x] Dependencia agregada
- [x] Middleware configurado
- [x] Código compilado correctamente
- [ ] Servidor reiniciado (requiere acción manual)
- [ ] Verificado funcionamiento

---

**Fecha**: 2024-12-06  
**Estado**: ✅ Implementado - Requiere reinicio del servidor

