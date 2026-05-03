# Cotizacion — Infraestructura Blockchain para Monitoreo Ambiental Minero en Argentina

**Fecha:** 28 de abril de 2026
**Vigencia:** 30 dias corridos desde la fecha de emision
**Moneda:** Dolares estadounidenses (USD), valores netos
**Cliente:** Ambioteck
**Proveedor:** Cerulean Ledger

---

## 1. Resumen ejecutivo

Propuesta de infraestructura blockchain como capa de trazabilidad, auditoria e inmutabilidad para la plataforma de inteligencia satelital y monitoreo ambiental de Ambioteck, orientada a proyectos de mineria en Argentina.

Cerulean Ledger provee el ledger permisionado donde se registran de forma inmutable las mediciones IoT hidricas (AquaTrace), los resultados de modelos de IA satelital, y los pasaportes ESG (AquaPassport) que las mineras presentan ante reguladores provinciales (DPA), la Secretaria de Mineria y rondas de financiamiento internacional (IFC/BID/PE).

**Contexto regulatorio:** La Ley de Glaciares, el regimen RIGI y las exigencias ESG del financiamiento internacional requieren evidencia objetiva, continua y auditable. El blockchain transforma datos de sensores y modelos de IA en registros legalmente verificables que no pueden ser alterados retroactivamente.

---

## 2. Rol de Cerulean Ledger en la solucion

| Servicio Ambioteck | Rol blockchain | Valor para la minera |
|---|---|---|
| **S-6 AquaTrace** — Trazabilidad hidrica IoT | Ledger permisionado: cada lectura de sensor (caudal, calidad, nivel) queda registrada con timestamp, firma criptografica y hash del dato crudo. Acceso de solo lectura para reguladores. | El DPA puede auditar el historial hidrico completo sin depender de reportes manuales de la minera. Evidencia inimpugnable. |
| **S-8 AquaPassport** — Pasaporte ESG | Certificados de cumplimiento on-chain: huella hidrica operacional, compromisos del EIA, indicadores de impacto. Credencial verificable vinculada al DID de la operacion minera. | Instrumento concreto para roadshows IFC/BID. El inversor verifica el cumplimiento ESG en tiempo real, no en un PDF estatico. |
| **S-1 GeoClassify / S-2 GlacierWatch / S-3 EIA-AI** | Registro inmutable de resultados de clasificacion IA: hash del modelo, version, datos de entrada, resultado. Cadena de custodia digital del dato satelital. | El EIA se respalda con evidencia tecnica cuyo historial es publicamente verificable. Dificil de impugnar judicialmente. |
| **S-4 HydroTwin** | Anclaje periodico del estado del gemelo digital al ledger (snapshot hash). Auditoria de cambios en el modelo. | Demuestra que el modelo predictivo no fue ajustado retroactivamente para favorecer resultados. |

---

## 3. Arquitectura tecnica

### Red permisionada por proyecto minero

Cada proyecto minero opera en un **canal aislado** (channel) dentro de la red Cerulean Ledger:

- **Aislamiento de datos:** Los datos hidricos de Los Azules no son visibles desde el canal de Rincon, y viceversa.
- **Acceso regulatorio:** Cada canal otorga acceso de lectura al DPA provincial correspondiente (San Juan, Salta, Jujuy, Catamarca, Santa Cruz) via identidad verificada.
- **Consenso BFT:** Tolerancia a fallas bizantinas — ningun nodo individual puede alterar registros.
- **Firmas post-cuanticas:** ML-DSA-65 (FIPS 204) disponible para operaciones que requieran proteccion a largo plazo.

### Componentes desplegados

| Componente | Descripcion |
|---|---|
| Nodos validadores | Red de 3-5 nodos BFT (Ambioteck, Cerulean, opcionalmente la minera y el regulador) |
| API de ingestion | Endpoints REST para recibir datos IoT (sensores Aguartec), resultados de IA, y documentos EIA |
| API de consulta | Endpoints de lectura para dashboards, auditoria regulatoria, y verificacion ESG |
| Almacenamiento persistente | RocksDB con column families por tipo de dato (mediciones, certificados, hashes de modelo) |
| Identidad descentralizada | DIDs para cada actor: operacion minera, sensor IoT, modelo de IA, regulador, auditor ESG |
| Credenciales verificables | AquaPassport como credencial W3C verificable, emitida on-chain, verificable off-chain |
| Explorer / Dashboard | Panel web de auditoria publica (Cerulean Ledger Explorer) adaptado al dominio minero-ambiental |

### Integracion con stack Ambioteck

```
Sensores IoT (Aguartec)  ──┐
Modelos IA satelital     ──┤──> API Ingestion ──> Cerulean Ledger ──> API Consulta ──┐
Gemelo digital (HydroTwin)─┘                         |                               |
                                                     v                               v
                                              Registro inmutable              Dashboard regulador
                                              (canal por proyecto)            Dashboard ESG (IFC/BID)
```

---

## 4. Alcance por fase

### Fase 1 — Infraestructura base + AquaTrace (Semanas 1-6)

Despliegue de la red y el flujo completo de trazabilidad hidrica para el primer proyecto piloto.

| # | Entregable | Descripcion |
|---|---|---|
| 1 | Red permisionada | 3 nodos BFT desplegados (cloud o on-premise), configuracion de canales, genesis block |
| 2 | API de ingestion IoT | Endpoints para registrar lecturas de sensores hidricos (caudal, pH, conductividad, nivel, temperatura) con firma y timestamp |
| 3 | Modelo de datos on-chain | Esquema de transacciones para mediciones hidricas, metadatos de sensor, alertas de anomalia |
| 4 | Canal piloto | Un canal configurado para el primer proyecto minero (recomendado: Los Azules o Rincon) |
| 5 | Acceso regulatorio | DID + credencial para el DPA provincial, con permisos de solo lectura sobre el canal |
| 6 | Dashboard de auditoria | Explorer web adaptado: timeline de mediciones, alertas, busqueda por sensor/fecha/parametro |
| 7 | Documentacion tecnica | Guia de integracion API, manual de operacion de nodos, procedimiento de onboarding de proyecto |

### Fase 2 — AquaPassport + Certificacion ESG (Semanas 7-10)

Capa de certificacion para roadshows de financiamiento.

| # | Entregable | Descripcion |
|---|---|---|
| 1 | Motor de credenciales ESG | Emision automatica de AquaPassport como credencial verificable (W3C VC) vinculada al historial on-chain |
| 2 | Indicadores configurables | Huella hidrica operacional, cumplimiento de compromisos EIA, variacion glaciar, balance de cuenca |
| 3 | Verificacion publica | Endpoint y QR para que un inversor o auditor verifique la credencial sin acceder al ledger completo |
| 4 | Reportes periodicos | Generacion automatica de reportes de cumplimiento (mensual/trimestral) con hash anclado al ledger |
| 5 | Integracion IA | Registro de hashes de resultados de modelos (GeoClassify, GlacierWatch, EIA-AI, HydroTwin) con cadena de custodia |

### Fase 3 — Escalamiento multi-proyecto (Semanas 11-14)

Onboarding de proyectos adicionales y hardening para produccion.

| # | Entregable | Descripcion |
|---|---|---|
| 1 | Canales adicionales | Hasta 5 proyectos mineros adicionales, cada uno en canal aislado |
| 2 | Multi-DPA | Configuracion de acceso para multiples reguladores provinciales (San Juan, Salta, Jujuy, Catamarca, Santa Cruz) |
| 3 | Nodo regulador (opcional) | Un nodo de la red operado por el regulador para maxima transparencia |
| 4 | Alta disponibilidad | Expansion a 5 nodos, failover automatico, backups cifrados |
| 5 | Monitoreo y alertas | Prometheus + Grafana: salud de nodos, latencia de consenso, volumen de transacciones |
| 6 | Capacitacion | Sesion para equipo Ambioteck (operacion de nodos, onboarding de clientes, troubleshooting) |

---

## 5. Cronograma y costos

### Setup e integracion (one-time)

| Fase | Duracion | Costo (USD) |
|---|---|---|
| Fase 1 — Infraestructura + AquaTrace | 6 semanas | $5.000 |
| Fase 2 — AquaPassport + ESG | 4 semanas | $3.500 |
| Fase 3 — Escalamiento multi-proyecto | 4 semanas | $3.000 |
| **Total setup** | **14 semanas** | **$11.500** |

**Descuento por contrato completo anticipado: $10.000** (ahorro de $1.500).

Nota: La infraestructura base (consenso BFT, identidad, storage, API, explorer) ya esta construida y en produccion. El costo de setup cubre exclusivamente integracion con los sistemas Ambioteck (IoT, IA), configuracion de canales por proyecto, y personalizacion del dashboard.

### Licencia mensual de operacion (post-despliegue)

| Concepto | Costo mensual (USD) |
|---|---|
| Operacion de red base (3 nodos, hasta 3 canales) | $500 |
| Canal adicional (por proyecto minero) | $150 /canal |
| Emision de AquaPassport (por credencial) | $25 /credencial |
| Almacenamiento extendido (>50 GB on-chain) | $100 /50 GB adicionales |

### Soporte (post-despliegue)

| Plan | Incluye | Costo mensual (USD) |
|---|---|---|
| Basico | Bugs criticos, actualizaciones de seguridad, soporte email (48h) | $300 |
| Estandar | Basico + mejoras menores, soporte prioritario (24h), monitoreo 24/7 | $600 |
| Premium | Estandar + ingeniero dedicado parcial, SLA 4h, soporte telefono | $1.200 |

---

## 6. Estimacion de costos por escenario

### Escenario A: Piloto con 1 proyecto (6 meses)

| Item | Costo |
|---|---|
| Fase 1 (setup) | $5.000 |
| Operacion 6 meses (1 canal) | $3.000 |
| Soporte basico 6 meses | $1.800 |
| **Total** | **$9.800** |

### Escenario B: 3 proyectos prioritarios (12 meses)

Proyectos: Los Azules (McEwen), Rincon (Rio Tinto), Vicuna Corp (Lundin/BHP)

| Item | Costo |
|---|---|
| Setup completo (Fases 1-3) | $10.000 |
| Operacion 12 meses (3 canales) | $6.000 |
| 12 AquaPassports (trimestrales x 3 proyectos x 4 trimestres) | $300 |
| Soporte estandar 12 meses | $7.200 |
| **Total** | **$23.500** |

### Escenario C: Plataforma completa — 10 proyectos (12 meses)

Cobertura de los proyectos de urgencia ALTA y MEDIA-ALTA del portafolio Ambioteck.

| Item | Costo |
|---|---|
| Setup completo (Fases 1-3) | $10.000 |
| Operacion 12 meses (3 base + 7 adicionales) | $18.600 |
| 40 AquaPassports | $1.000 |
| Soporte premium 12 meses | $14.400 |
| **Total** | **$44.000** |

---

## 7. Modelo de revenue-share (alternativa)

Si Ambioteck prefiere minimizar el desembolso inicial, ofrecemos un modelo de revenue-share:

| Parametro | Valor |
|---|---|
| Setup con descuento | $5.000 (50% del costo base) |
| Revenue share por proyecto minero onboarded | 8% del valor facturado por Ambioteck al cliente final por los servicios S-6 y S-8 |
| Minimo mensual garantizado | $500 |
| Duracion del acuerdo | 24 meses, renovable |

Este modelo alinea incentivos: Cerulean Ledger invierte en el exito comercial de Ambioteck.

---

## 8. Diferenciadores tecnicos

| Caracteristica | Cerulean Ledger | Alternativas (Hyperledger, cloud generico) |
|---|---|---|
| Consenso BFT nativo | Si, HotStuff-inspired | Requiere configuracion compleja |
| Firmas post-cuanticas (ML-DSA-65) | Si, FIPS 204 | No disponible |
| Identidad descentralizada (DIDs) | Nativo | Requiere integracion externa |
| Credenciales verificables (W3C VC) | Nativo | Requiere desarrollo custom |
| Canales aislados por proyecto | Si, Fabric-compatible | Parcial |
| Ejecucion paralela de transacciones | Si, wave-parallel MVCC (56K TPS) | Limitado |
| EVM compatible | Si, contratos Solidity si se requieren | Depende |
| Modulo criptografico FIPS 140-3 | En proceso de certificacion | No |
| Codigo fuente auditable | Si | Depende del vendor |

---

## 9. Requisitos de Ambioteck

- Designar un punto de contacto tecnico para la integracion API.
- Proveer especificaciones de los sensores IoT (Aguartec): formato de datos, frecuencia de envio, protocolo.
- Definir los indicadores ESG que componen cada AquaPassport por tipo de proyecto (cobre, litio, oro).
- Infraestructura cloud para nodos (o contratar hosting como servicio adicional, estimado $300-500/mes por nodo).

---

## 10. Exclusiones

- Desarrollo de los modelos de IA satelital (responsabilidad de Ambioteck).
- Sensores IoT y su instalacion en campo.
- Tramites regulatorios ante DPA, Secretaria de Mineria, o COFEMA.
- Certificaciones formales ante organismos ambientales argentinos.
- Hosting e infraestructura cloud (cotizable por separado).
- Integracion con sistemas gubernamentales argentinos (TAD, COFEMA digital) — cotizable como adenda.

---

## 11. Proyectos mineros de referencia (del portafolio Ambioteck)

La siguiente tabla resume los 15 proyectos identificados y los servicios blockchain aplicables a cada uno:

| Proyecto | Provincia | Urgencia | Servicios blockchain |
|---|---|---|---|
| Los Azules (McEwen) | San Juan | MUY ALTA | AquaTrace + AquaPassport + cadena de custodia EIA/IA |
| Vicuna / Josemaria (Lundin/BHP) | San Juan | MUY ALTA | AquaTrace + AquaPassport (roadshow USD 18.000M) |
| Rincon (Rio Tinto) | Salta | ALTA | AquaTrace + AquaPassport (ESG Tier 1) |
| Hombre Muerto O. (Galan) | Catamarca | ALTA | AquaTrace + HydroTwin anchor |
| Cauchari-Olaroz (Ganfeng) | Jujuy | ALTA | AquaTrace + AquaPassport (ampliacion RIGI) |
| Centenario-Ratones (Eramet) | Salta | ALTA | AquaTrace (EU ESG reporting) |
| El Pachon (Glencore) | San Juan | MEDIA-ALTA | AquaTrace + cadena de custodia EIA |
| Lindero (Fortuna Silver) | Salta | MEDIA | AquaTrace |
| Joaquin + Cerro Leon (Unico) | Santa Cruz | MEDIA-ALTA | Cadena de custodia EIA (cotiza ASX) |
| Pastos Grandes (Ganfeng) | Salta | MEDIA | AquaTrace + AquaPassport |
| Cerro Moro (AEM/PAAS) | Santa Cruz | MEDIA | AquaTrace |
| Pirquitas (SSR Mining) | Jujuy | MEDIA | AquaTrace |
| Esperanza (Latin Metals) | San Juan | BAJA | Cadena de custodia EIA |
| Solaroz (Lithium Energy) | Jujuy | BAJA | HydroTwin anchor |
| La Manchuria (Astra) | Santa Cruz | BAJA | Cadena de custodia EIA |

---

## 12. Garantias

- Todo el codigo fuente de la capa de integracion se entrega a Ambioteck.
- La plataforma base (Cerulean Ledger) es de codigo abierto y auditable.
- Suite de tests automatizados con cobertura superior al 80%.
- Documentacion tecnica y de integracion incluida.
- 2 semanas de soporte post-lanzamiento incluidas en Fase 3.

---

## 13. Contacto

**Cerulean Ledger**
Contacto: [nombre]
Email: [email]
Telefono: [telefono]

---

*Este documento es una cotizacion referencial. Los plazos y costos pueden ajustarse segun los requisitos especificos del proyecto una vez definido el alcance detallado con Ambioteck.*
