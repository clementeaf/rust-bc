# Matriz Vertical × Horizontal

Cómo cada vertical de negocio consume las capacidades horizontales de la plataforma.

**Fecha:** 2026-05-08

---

## La plataforma horizontal

Cerulean Ledger es una plataforma horizontal que provee capacidades reutilizables:

| Capacidad | Qué garantiza |
|---|---|
| **Inmutabilidad** | Registros no alterables post-escritura |
| **Criptografía PQC** | Protección a largo plazo contra amenazas cuánticas |
| **Canales** | Aislamiento de datos entre organizaciones |
| **ACL / MSP** | Control granular de quién hace qué |
| **Gobernanza on-chain** | Decisiones colectivas verificables |
| **EVM** | Lógica programable compatible con Ethereum |
| **Light client** | Verificación desde dispositivos ligeros (móvil, IoT) |
| **Ejecución paralela** | Alto throughput para operaciones masivas |

---

## Las 4 verticales

### 1. Credenciales verificables
Emisión y verificación de títulos, certificaciones y documentos digitales.

### 2. Voto electrónico
Votación verificable, auditable y privada para instituciones.

### 3. Trazabilidad supply chain
Cadena de custodia compartida entre múltiples organizaciones.

### 4. Auditoría financiera
Trail de auditoría compartido entre instituciones reguladas.

---

## Matriz de consumo

| Capacidad | Credenciales | Voto | Supply chain | Finanzas |
|---|---|---|---|---|
| Inmutabilidad | Alto | Alto | Alto | Alto |
| PQC | Alto | Alto | Medio | Alto |
| Canales | Medio | Alto | Alto | Alto |
| ACL / MSP | Medio | Alto | Alto | Alto |
| Gobernanza | Mínimo | Alto | Medio | Medio |
| EVM | Mínimo | Mínimo | Bajo | Alto |
| Light client | Alto | Medio | Bajo | Bajo |
| Ejecución paralela | Alto | Alto | Alto | Alto |

### Detalle por cruce

**Inmutabilidad:** Título no se altera · Acta no se altera · Custodia intacta · Trail intocable

**PQC:** Título válido 30+ años · Elección verificable a futuro · Datos de vida corta (medio) · Regulador lo exigirá

**Canales:** Separar emisores · Separar elecciones · Cada consorcio aislado · Cada banco ve lo suyo

**ACL / MSP:** Solo emisor emite · Solo votante registrado vota · Cada actor ve su tramo · Auditor ve todo, operador parcial

**Gobernanza:** Poco uso · Es el producto mismo · Cambios de reglas entre socios · Cambios de política entre bancos

**EVM:** Poco uso · Poco uso · Poco uso · Lógica financiera programable

**Light client:** Verificar desde celular · Verificar resultado desde móvil · Poco uso · Poco uso

**Ejecución paralela:** Emisión masiva (graduación) · Miles de votos simultáneos · Miles de checkpoints · Conciliación batch

---

## Qué significa para el negocio

### El leverage de la horizontal

Cada vertical adicional tiene costo marginal cercano a cero porque reutiliza la misma infraestructura:

| Vertical | Costo relativo | Razón |
|---|---|---|
| Primera | 100% | Construir plataforma + vertical |
| Segunda | 20% | Solo lógica de negocio |
| Tercera | 15% | Menos aún |
| Cuarta | 10% | Casi solo configuración |

### Priorización recomendada

Basado en madurez del producto y facilidad de entrada al mercado:

| Prioridad | Vertical | Razón |
|---|---|---|
| 1 | **Credenciales** | Demo lista, flujo completo, PQC como diferenciador, light client listo |
| 2 | **Voto electrónico** | App Cerulean Voto funcional, gobernanza on-chain es el core, alta visibilidad |
| 3 | **Supply chain** | Canales + private data listos, pero requiere integración con sistemas del cliente |
| 4 | **Auditoría financiera** | EVM listo, pero requiere compliance regulatorio más estricto (auditoría externa, FIPS) |

### Dependencias entre verticales

Las verticales comparten componentes horizontales pero son independientes entre sí. Un piloto de credenciales no bloquea un piloto de votación. Esto permite:

- Pilotos paralelos en verticales distintas
- Casos de éxito en una vertical que generan confianza para las demás
- Un cliente que entra por una vertical y adopta otra después

---

## Resumen

La plataforma se construye una vez. Cada vertical consume lo que necesita sin código nuevo. El valor comercial crece linealmente con cada vertical; el costo crece marginalmente. Ese es el modelo de negocio.
