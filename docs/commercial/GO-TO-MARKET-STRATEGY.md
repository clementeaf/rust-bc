# Estrategia de Adopción — Go-to-Market

**Fecha:** 2026-05-08

---

## Principio rector

No vendemos blockchain. Vendemos solución a un problema de negocio que usa blockchain por debajo. La tecnología es el motor, no el producto.

---

## Vocabulario

| No decir | Sí decir |
|---|---|
| Blockchain permisionada | Plataforma de registros digitales verificables |
| Consenso BFT | Ninguna parte puede alterar registros unilateralmente |
| Smart contracts en Wasm | Reglas de negocio automatizadas y auditables |
| Criptografía post-cuántica | Protección contra amenazas computacionales de próxima generación |
| Canales y private data | Cada organización ve solo lo que le corresponde |
| Finalidad determinística | Los registros son definitivos al instante, sin esperas |

---

## Playbook en 5 pasos

### Paso 1 — Piloto gratis con una institución ancla

- Elegir UNA vertical: credenciales o voto (son las más maduras)
- Encontrar una institución dispuesta a pilotear sin costo
- Objetivo: caso de éxito publicable, no revenue
- Duración: 4-6 semanas
- Chile: Cámara Blockchain, universidad, municipio, colegio profesional

**Criterios para elegir la institución ancla:**
- Tiene un dolor real (falsificación, disputas, falta de trazabilidad)
- Tiene capacidad técnica mínima (puede levantar Docker o acepta SaaS)
- Está dispuesta a que se publique el caso
- Tiene visibilidad en su sector

### Paso 2 — Caso de éxito con métricas

Documentar resultados concretos, sin jerga:

- "Municipio X redujo disputas de actas en 100%"
- "Universidad Y verifica títulos en 3 segundos vs 5 días"
- "Colegio Z eliminó certificaciones fraudulentas"

Incluir: tiempo de implementación, costo, métricas antes/después.

### Paso 3 — Replicar en el mismo sector

El primer caso de éxito abre el sector. Una universidad exitosa genera interés en otras universidades. Un municipio genera interés en otros municipios.

- Presentar caso de éxito en eventos sectoriales
- Ofrecer piloto con condiciones preferenciales
- Construir comunidad de usuarios en la vertical

### Paso 4 — Modelo de negocio sostenible

| Modelo | Cuándo usarlo | Ventaja |
|---|---|---|
| **SaaS** | Organizaciones pequeñas/medianas, arranque rápido | Revenue recurrente, bajo friction |
| **On-premise** | Gobierno, banca, defensa — exigen soberanía | Ticket alto, compliance |
| **Consorcio** | Múltiples organizaciones comparten red | Cada miembro paga membresía |

Empezar con SaaS (menor barrera). Ofrecer on-premise cuando el cliente lo exija.

### Paso 5 — Ecosistema de integradores

No vender directo a enterprise grande. Vender a través de:

- Consultoras tecnológicas (Accenture, Deloitte, Big 4 locales)
- Integradores de sistemas sectoriales
- Partners regionales

Ellos venden el proyecto, cobran la implementación. Cerulean cobra licencia/soporte.

---

## Canal de entrada por vertical

| Vertical | Cliente ancla ideal | Evento de entrada |
|---|---|---|
| Credenciales | Universidad o colegio profesional | Temporada de graduación / renovación de certificaciones |
| Voto | Municipio o cámara de comercio | Elección de directiva / consulta ciudadana |
| Supply chain | Empresa minera o agroexportadora | Auditoría de certificación (ISO, HACCP) |
| Finanzas | Fintech o cooperativa | Requisito regulatorio nuevo (Ley 21.663) |

---

## Certificaciones como moat competitivo

Las certificaciones crean barrera de entrada que competidores pequeños no pueden cruzar:

| Certificación | Mercado que abre | Estado |
|---|---|---|
| **Compliance Ley 21.663** | Todo servicio esencial en Chile | Mapeo documentado, gaps menores |
| **FIPS 140-3** | Gobierno USA, defensa, banca regulada internacional | Pre-lab hecho, 12-24 meses para formal |
| **ISO 27001** | Enterprise global | No iniciado |
| **SOC 2** | SaaS / fintech | No iniciado |

---

## Métricas de éxito

| Hito | Métrica | Plazo |
|---|---|---|
| Sandbox público en línea | URL accesible, uptime >99% | Semana 2 |
| Primer piloto iniciado | Institución firmó acuerdo | Mes 1-2 |
| Primer caso publicado | Documento con métricas reales | Mes 3 |
| Primer cliente pagando | Revenue recurrente | Mes 4-6 |
| 3 clientes en una vertical | Tracción demostrable | Mes 6-9 |
| Segundo vertical activada | Leverage de plataforma demostrado | Mes 9-12 |

---

## Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| No encontrar institución ancla | Ofrecer piloto completamente gratis + soporte dedicado |
| Piloto falla técnicamente | Stress test previo, rehearsal completo antes de ir al cliente |
| Cliente exige certificación formal | Mostrar pre-lab FIPS + compliance Ley 21.663 documentado |
| Competidor llega primero | Velocidad: el sandbox y el primer piloto son la prioridad |
| "Es solo otra blockchain" | Nunca usar la palabra blockchain en el pitch inicial |

---

## Resumen

La ruta no es convencer a empresas de usar blockchain. Es:

1. Elegir UN problema concreto
2. Resolverlo para UN cliente
3. Publicar el caso de éxito
4. Repetir en el sector

La tecnología ya está. Lo que falta es el primer deploy en producción y la narrativa comercial sin jerga.
