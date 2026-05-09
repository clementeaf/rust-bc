# Preguntas frecuentes — Evaluacion empresarial

Documento para equipos de TI, compliance y gerencia que evaluan Cerulean Ledger como plataforma de registros verificables.

---

## 1. ¿Quedamos atados al proveedor?

No. Cerulean esta disenado para que usted mantenga el control en todo momento:

- **Codigo abierto** — El software completo esta disponible publicamente. Si algun dia dejamos de operar, su equipo (o cualquier tercero) puede mantener la red sin nuestra participacion.
- **Sus datos, su infraestructura** — Los nodos corren en sus propios servidores. No alojamos su informacion ni controlamos su red.
- **Exportacion completa** — En cualquier momento puede extraer todos los registros en formato estandar. No hay barreras de salida.

**Detalle tecnico:**

```bash
# Exportar ledger completo en cualquier momento
POST /api/v1/forensic/export

# Formato: JSON + firmas criptograficas verificables sin la plataforma
# Fork del repositorio y sigue operando — sin dependencia de Cerulean
# Contenedores OCI estandar — migra a cualquier Docker/Kubernetes
```

---

## 2. ¿Usan estandares abiertos o tecnologia propietaria?

Estandares abiertos en cada capa. Esto significa que su inversion no depende de una sola empresa:

| Area | Estandar utilizado |
|---|---|
| Seguridad | Criptografia post-cuantica certificable (FIPS 204, 202, 203) |
| Identidad | Identificadores descentralizados (W3C DID) y credenciales verificables |
| Financiero | ISO 20022, ISO 4217 (monedas), ISO 8601 (fechas) |
| Tokens regulados | ERC-3643 (security tokens con compliance integrado) |
| Comunicacion | API REST + JSON — sin herramientas especiales para conectarse |
| Despliegue | Contenedores estandar — corre en cualquier infraestructura moderna |

No necesita aprender un lenguaje nuevo, comprar hardware especial, ni instalar software propietario para usar la plataforma.

**Detalle tecnico:**

```
Criptografia:
  ML-DSA-65 (FIPS 204) — firmas post-cuanticas
  SHA3-256 (FIPS 202) — hash configurable
  ML-KEM-768 (FIPS 203) — key encapsulation para TLS hibrido

Identidad:
  did:cerulean: con rotacion de llaves
  Verifiable Credentials W3C con firma del emisor

Financiero:
  7 tipos de mensaje ISO 20022 validados (pacs.008, pacs.002, pacs.004,
  pain.001, pain.002, camt.053, camt.052)
  64 monedas ISO 4217 — soporte 3 decimales
  193 paises ISO 3166
```

---

## 3. ¿Cual es el costo real en el ano tres?

Transparente y predecible. Sin sorpresas a medida que crece el uso:

**Lo que NO cobramos:**
- No hay cobro por transaccion
- No hay token ni criptomoneda que comprar
- No hay costos de "gas" que fluctuan con el mercado
- No alojamos su red — no paga hosting a nosotros

**Lo que SI incluye:**
- **Ano 1** — Implementacion, capacitacion y licencia
- **Ano 2 en adelante** — Soporte, actualizaciones de seguridad y nuevas funcionalidades

**Resultado:** A mayor uso, menor costo por operacion. El hardware necesario es modesto — un nodo completo opera con los mismos recursos que un servidor de correo pequeno.

**Detalle tecnico:**

```
Requisitos de hardware por nodo:
  CPU:     2 vCPU
  RAM:     4 GB
  Disco:   SSD, ~1 KB por bloque (RocksDB)
  Red:     Cualquier conexion con <100ms latencia entre nodos

Rendimiento demostrado:
  18,700 TX/s (motor de ejecucion paralela)
  14 ms latencia p50
  Storage backend: RocksDB local — sin base de datos externa
```

---

## 4. ¿Como se integra con nuestros sistemas actuales?

De la misma forma que cualquier servicio moderno. Si su ERP, CRM o sistema de trazabilidad puede hacer una llamada HTTP (y practicamente todos pueden), se integra con Cerulean.

**Ejemplos concretos:**

- Su ERP emite una orden de compra → Cerulean registra el hash como evidencia inmutable
- Su sistema de RRHH verifica un titulo profesional → consulta directa, respuesta en milisegundos
- Su plataforma de compliance valida un mensaje financiero → respuesta inmediata con detalle de errores
- Ocurre un evento de seguridad → Cerulean notifica automaticamente a su sistema de monitoreo (SIEM)

**Opciones de conexion:**
- API REST directa (la mas simple — funciona con cualquier lenguaje)
- SDKs listos para TypeScript y Python
- Webhooks para notificaciones automaticas hacia sus sistemas

No se requiere reemplazar ni modificar sus sistemas actuales. Cerulean se acopla como una capa adicional de verificacion.

**Detalle tecnico:**

```bash
# Emitir credencial desde ERP
curl POST /api/v1/store/credentials \
  -d '{"issuer_did":"did:cerulean:su-org", "subject_did":"...", ...}'

# Consultar precios de oracle (datos de mercado en tiempo real)
curl GET /api/v1/oracle/feeds

# Validar mensaje ISO 20022 antes de enviarlo al banco
curl POST /api/v1/compliance/validate/pacs008 -d '{...}'

# Sellar un bloque desde un webhook de su sistema
curl POST /api/v1/mine -d '{"miner_address":"..."}'

# Webhooks inversos: nodo notifica a su SIEM
# Configurar con variable de entorno:
CSIRT_WEBHOOK_URL=https://su-siem.empresa.cl/events

# SDKs disponibles:
#   TypeScript — sdks/js/ (axios, tipos completos, ejemplos)
#   Python — sdks/python/ (cliente, tipos, excepciones)
```

---

## 5. ¿Que pasa si la regulacion cambia?

La plataforma fue construida anticipando cambios regulatorios:

- **Criptografia post-cuantica** — Preparada para los requisitos de seguridad de la proxima decada, no solo los de hoy
- **Ley 21.663 (Ciberseguridad Chile)** — Cumplimiento verificado en las 21 categorias aplicables
- **Gobernanza on-chain** — Los parametros del sistema se modifican mediante votacion, no mediante actualizaciones que rompen compatibilidad

Cuando la regulacion cambia, la plataforma se adapta mediante configuracion — no requiere migraciones ni re-implementaciones.

**Detalle tecnico:**

```bash
# Migrar algoritmo de firma sin redeployar
SIGNING_ALGORITHM=ml-dsa-65   # Ed25519 → post-cuantico

# Modo dual: transicion gradual entre algoritmos
DUAL_SIGN_VERIFY_MODE=either  # Acepta ambos durante migracion
DUAL_SIGN_VERIFY_MODE=both    # Exige ambos post-migracion

# Cambios de protocolo via gobernanza (votacion de validadores)
POST /api/v1/governance/proposals
POST /api/v1/governance/proposals/{id}/vote

# 21 verificaciones regulatorias automaticas
GET /api/v1/regulatory/report
# Resultado: informe con hash SHA-256 para auditoria
```

---

## 6. ¿Como sabemos que es seguro?

Tres niveles de verificacion independientes:

1. **Auditoria externa** — Evaluada por la Camara Blockchain de Chile en cuatro dimensiones: motor, transacciones, contratos inteligentes y seguridad.
2. **Pentest adversarial** — 20 escenarios de ataque simulados (falsificacion, doble gasto, manipulacion de datos, fuerza bruta). Cero vulnerabilidades criticas.
3. **Pruebas automatizadas** — Mas de 1,600 verificaciones que se ejecutan antes de cada cambio. Ningun codigo llega a produccion sin pasar todas.

**Detalle tecnico:**

```bash
# Informe de pentest en tiempo real (ejecuta los 20 ataques)
GET /api/v1/pentest/report

# Ataques simulados incluyen:
#   - Falsificacion de firma (Ed25519 y ML-DSA-65)
#   - Doble gasto via transacciones concurrentes
#   - Replay de bloques
#   - Equivocacion bizantina (doble propuesta)
#   - Bypass de ACL sin identidad
#   - Evasion de rate limit
#   - Cruce entre canales aislados
#   - Suplantacion de identidad (tag forgery)
#   - Manipulacion de oracle (outlier 20x)
#   - Overflow en tokens
#   - Inyeccion de null bytes
#   - Payload oversized (100KB)
#   - Path traversal en canales
#   - Voto sin stake en gobernanza
#   - Forgery de credenciales

# Protecciones activas:
#   Rate limiting por IP (sliding window)
#   mTLS + ACL en cada endpoint de mutacion
#   ZeroizeOnDrop + mlock en claves privadas
#   Deteccion de equivocacion con cuarentena automatica
#   MVCC para prevencion de doble gasto

# Suite de tests (ejecutar localmente)
cargo test --lib   # 1,604 tests
```

---

## 7. ¿Cuanto tarda una implementacion tipica?

Depende del alcance, pero la arquitectura esta pensada para ser rapida:

- **Piloto funcional** — Semanas, no meses. Un nodo se levanta en minutos con datos de prueba incluidos.
- **Integracion con sistemas existentes** — Depende de la complejidad del lado del cliente, pero la API es estandar y documentada.
- **Produccion multi-organizacion** — Requiere definir politicas de gobernanza entre las partes. La tecnologia no es el cuello de botella.

**Detalle tecnico:**

```bash
# Levantar piloto completo (nodo + explorer + datos demo)
./scripts/sandbox.sh

# O manualmente en 3 comandos:
docker compose -f docker-compose.sandbox.yml up -d
./scripts/seed-sandbox.sh
# Listo: explorador en :5173, API en :9600

# Produccion multi-nodo (3 peers + 3 orderers + monitoring):
docker compose up -d
# Incluye Prometheus + Grafana preconfigurado
```

---

## Resumen en una frase

> Cerulean no es una plataforma donde usted entra — es infraestructura que usted opera. Sus datos, sus servidores, estandares abiertos. Si manana no existimos, su red sigue funcionando.

---

*Documento actualizado: 2026-05-09*
