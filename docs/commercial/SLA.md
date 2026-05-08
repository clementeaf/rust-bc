# Acuerdo de Nivel de Servicio (SLA)

**Fecha:** 2026-05-08

---

## Modelos de servicio

### Tier 1 — SaaS (Cerulean hospedado)

| Métrica | Compromiso |
|---|---|
| Disponibilidad mensual | 99.5% |
| Tiempo máximo de caída continua | 4 horas |
| Latencia de API (p95) | < 200 ms |
| Finalidad de transacción | Inmediata (determinística) |
| Backup de datos | Diario, retención 30 días |
| Soporte | Correo electrónico, respuesta < 24 horas hábiles |
| Ventana de mantenimiento | Domingos 02:00-06:00 CLT (notificación 48h antes) |

### Tier 2 — On-premise con soporte

| Métrica | Compromiso |
|---|---|
| Disponibilidad | Depende de infraestructura del cliente |
| Soporte por incidentes críticos | Respuesta < 4 horas, 24/7 |
| Soporte por incidentes no críticos | Respuesta < 8 horas hábiles |
| Actualizaciones de seguridad | Entrega dentro de 72 horas de publicación |
| Asistencia en despliegue | Incluida en onboarding |
| Revisión trimestral | Reunión de estado + recomendaciones |

### Tier 3 — Consorcio (red compartida)

| Métrica | Compromiso |
|---|---|
| Disponibilidad de red | 99.9% (mínimo 3 nodos operativos) |
| Latencia de consenso (Raft) | < 50 ms entre nodos en misma región |
| Incorporación de nuevo miembro | < 5 días hábiles desde aprobación |
| Soporte | Dedicado, respuesta < 2 horas en horario hábil |
| Gobernanza | Votación on-chain para cambios de política |

---

## Exclusiones

El SLA no aplica en caso de:

- Fuerza mayor (desastres naturales, cortes de energía masivos)
- Mantenimiento programado dentro de la ventana notificada
- Acciones del cliente que causen la interrupción (eliminación de certificados, misconfiguration)
- Ataques DDoS contra infraestructura del cliente (en modelo on-premise)

---

## Métricas de medición

| Métrica | Cómo se mide |
|---|---|
| Disponibilidad | Porcentaje de tiempo en que `/api/v1/health` responde 200 en intervalos de 1 minuto |
| Latencia | Percentil 95 de tiempo de respuesta medido desde el load balancer |
| Tiempo de respuesta de soporte | Desde la recepción del ticket hasta la primera respuesta calificada |
| Finalidad | Tiempo desde submit de transacción hasta confirmación de commit |

---

## Compensaciones (modelo SaaS)

| Disponibilidad mensual | Compensación |
|---|---|
| 99.0% - 99.5% | 10% del cargo mensual como crédito |
| 95.0% - 99.0% | 25% del cargo mensual como crédito |
| < 95.0% | 50% del cargo mensual como crédito |

El cliente debe solicitar el crédito dentro de los 30 días siguientes al mes afectado.

---

## Reportes

- **Mensual:** Reporte de disponibilidad, incidentes, y métricas de rendimiento
- **Trimestral:** Revisión de cumplimiento de SLA + recomendaciones de mejora
- **Por incidente:** Post-mortem dentro de 5 días hábiles para incidentes críticos

---

*Este SLA es un modelo base. Los términos específicos se negocian por cliente según el modelo de servicio y las necesidades del proyecto.*
