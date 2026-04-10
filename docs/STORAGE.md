# Almacenamiento de Datos

## Dos capas de almacenamiento

La blockchain maneja los datos en dos capas complementarias:

### 1. On-chain (en la blockchain)

Datos que quedan registrados directamente en la cadena de bloques:
- Transacciones firmadas
- Hashes de bloques
- Firmas digitales (DID)
- Credenciales verificables
- Metadatos y marcas temporales

Estos datos son **inmutables** y **replicados** en cada nodo de la red.

### 2. Off-chain (fuera de la cadena)

Datos pesados o sensibles que no deben replicarse a todos los nodos:
- Documentos completos (PDFs, imagenes)
- Historiales medicos detallados
- Informacion privada entre organizaciones

Se almacenan en **private data collections**, accesibles solo para las organizaciones autorizadas por la politica de endorsement.

---

## Motor de persistencia: RocksDB

### Que es RocksDB

Una base de datos embebida de clave-valor creada por Facebook/Meta. A diferencia de PostgreSQL o MySQL, **no es un servidor separado** — se ejecuta dentro del mismo proceso de la blockchain. Usa LSM-trees (Log-Structured Merge) para optimizar escrituras rapidas en disco.

### Por que RocksDB

| Criterio | RocksDB | DB relacional (PostgreSQL) | En memoria |
|---|---|---|---|
| Latencia de escritura | Muy baja | Media | Instantanea |
| Persistencia | Si (disco) | Si (disco) | No (se pierde al reiniciar) |
| Complejidad operativa | Ninguna (embebida) | Alta (servidor separado) | Ninguna |
| Escalabilidad en disco | Excelente (TB de datos) | Buena | Limitada por RAM |
| Dependencias externas | Ninguna | Requiere servicio corriendo | Ninguna |

### Como se organiza la data

RocksDB usa **Column Families** para separar tipos de datos, similar a tablas en una DB relacional:

```
RocksDB (./data/rocksdb)
|
|-- blocks          -> Bloques de la cadena (clave: altura zero-padded a 12 digitos)
|-- transactions    -> Transacciones individuales (clave: tx_id)
|-- identities      -> Registros DID de identidad (clave: did)
|-- credentials     -> Credenciales verificables (clave: credential_id)
|-- meta            -> Metadatos del nodo (altura actual, config)
|-- tx_by_block     -> Indice secundario: transacciones por bloque
```

### Claves y orden lexicografico

Las claves de bloques se formatean con **zero-padding a 12 digitos** para que el orden lexicografico coincida con el orden numerico:

```
Bloque 1    -> "000000000001"
Bloque 42   -> "000000000042"
Bloque 1000 -> "000000001000"
```

Esto permite hacer range scans eficientes (ej: "dame los bloques del 100 al 200") sin conversion numerica.

### Indice secundario: tx_by_block

Para buscar transacciones por bloque sin recorrer toda la tabla, se usa un indice secundario con clave compuesta:

```
{altura_zero_padded}:{tx_id}

Ejemplo:
"000000000042:tx_abc123"
"000000000042:tx_def456"
"000000000043:tx_ghi789"
```

Un prefix scan con `"000000000042:"` devuelve todas las transacciones del bloque 42 sin escanear el resto.

---

## Configuracion

| Variable | Default | Descripcion |
|---|---|---|
| `STORAGE_BACKEND` | `memory` | Backend de almacenamiento. Usar `rocksdb` para persistencia |
| `STORAGE_PATH` | `./data/rocksdb` | Directorio donde RocksDB almacena los datos |

### Modo memoria (default)

```bash
cargo run
```

Los datos viven en HashMaps en RAM. Util para desarrollo y testing. **Se pierden al reiniciar.**

### Modo RocksDB (produccion)

```bash
STORAGE_BACKEND=rocksdb STORAGE_PATH=./data/rocksdb cargo run
```

Los datos persisten en disco. Cada nodo mantiene su propia copia independiente.

---

## Arquitectura de almacenamiento

```
+-------------------------------------------+
|            Aplicacion / API               |
+-------------------------------------------+
                    |
                    v
+-------------------------------------------+
|         Trait BlockStore                   |
|  (interfaz abstracta de almacenamiento)   |
+-------------------------------------------+
          |                    |
          v                    v
+------------------+  +------------------+
|   MemoryStore    |  | RocksDbBlockStore|
|   (HashMap)      |  | (Column Families)|
|   Para testing   |  | Para produccion  |
+------------------+  +------------------+
                              |
                              v
                    +------------------+
                    |     Disco        |
                    |  ./data/rocksdb  |
                    +------------------+
```

El trait `BlockStore` define la interfaz comun. La implementacion concreta se selecciona en runtime segun `STORAGE_BACKEND`. Esto permite cambiar de backend sin modificar la logica de negocio.
