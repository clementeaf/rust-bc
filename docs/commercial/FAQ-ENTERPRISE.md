# Preguntas frecuentes — Evaluacion empresarial

Documento para equipos de TI, compliance y gerencia que evaluan Cerulean Ledger como plataforma de registros verificables.

---

## 1. ¿Quedamos atados al proveedor?

No. Cerulean esta disenado para que usted mantenga el control en todo momento:

- **Codigo abierto** — El software completo esta disponible publicamente. Si algun dia dejamos de operar, su equipo (o cualquier tercero) puede mantener la red sin nuestra participacion.
- **Sus datos, su infraestructura** — Los nodos corren en sus propios servidores. No alojamos su informacion ni controlamos su red.
- **Exportacion completa** — En cualquier momento puede extraer todos los registros en formato estandar. No hay barreras de salida.

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

---

## 5. ¿Que pasa si la regulacion cambia?

La plataforma fue construida anticipando cambios regulatorios:

- **Criptografia post-cuantica** — Preparada para los requisitos de seguridad de la proxima decada, no solo los de hoy
- **Ley 21.663 (Ciberseguridad Chile)** — Cumplimiento verificado en las 21 categorias aplicables
- **Gobernanza on-chain** — Los parametros del sistema se modifican mediante votacion, no mediante actualizaciones que rompen compatibilidad

Cuando la regulacion cambia, la plataforma se adapta mediante configuracion — no requiere migraciones ni re-implementaciones.

---

## 6. ¿Como sabemos que es seguro?

Tres niveles de verificacion independientes:

1. **Auditoria externa** — Evaluada por la Camara Blockchain de Chile en cuatro dimensiones: motor, transacciones, contratos inteligentes y seguridad.
2. **Pentest adversarial** — 20 escenarios de ataque simulados (falsificacion, doble gasto, manipulacion de datos, fuerza bruta). Cero vulnerabilidades criticas.
3. **Pruebas automatizadas** — Mas de 1,600 verificaciones que se ejecutan antes de cada cambio. Ningun codigo llega a produccion sin pasar todas.

---

## 7. ¿Cuanto tarda una implementacion tipica?

Depende del alcance, pero la arquitectura esta pensada para ser rapida:

- **Piloto funcional** — Semanas, no meses. Un nodo se levanta en minutos con datos de prueba incluidos.
- **Integracion con sistemas existentes** — Depende de la complejidad del lado del cliente, pero la API es estandar y documentada.
- **Produccion multi-organizacion** — Requiere definir politicas de gobernanza entre las partes. La tecnologia no es el cuello de botella.

---

## Resumen en una frase

> Cerulean no es una plataforma donde usted entra — es infraestructura que usted opera. Sus datos, sus servidores, estandares abiertos. Si manana no existimos, su red sigue funcionando.

---

*Documento actualizado: 2026-05-09*
