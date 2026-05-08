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

Intensidad de uso: alto / medio / bajo / mínimo

| Capacidad horizontal | Credenciales | Voto electrónico | Supply chain | Auditoría financiera |
|---|---|---|---|---|
| **Inmutabilidad** | Alto — título no se altera | Alto — acta no se altera | Alto — custodia intacta | Alto — trail intocable |
| **PQC** | Alto — título válido 30+ años | Alto — elección verificable a futuro | Medio — datos de vida corta | Alto — regulador lo exigirá |
| **Canales** | Medio — separar emisores | Alto — separar elecciones | Alto — cada consorcio aislado | Alto — cada banco ve lo suyo |
| **ACL / MSP** | Medio — solo emisor emite | Alto — solo votante registrado vota | Alto — cada actor ve su tramo | Alto — auditor ve todo, operador parcial |
| **Gobernanza on-chain** | Mínimo | Alto — es el producto mismo | Medio — cambios de reglas entre socios | Medio — cambios de política entre bancos |
| **EVM** | Mínimo | Mínimo | Bajo | Alto — lógica financiera programable |
| **Light client** | Alto — verificar desde celular | Medio — verificar resultado desde móvil | Bajo | Bajo |
| **Ejecución paralela** | Alto — emisión masiva (graduación) | Alto — miles de votos simultáneos | Alto — miles de checkpoints | Alto — conciliación batch |

---

## Qué significa para el negocio

### El leverage de la horizontal

Cada vertical adicional tiene costo marginal cercano a cero porque reutiliza la misma infraestructura:

```
Costo de la primera vertical:   ████████████████████  (construir plataforma + vertical)
Costo de la segunda vertical:   ████                  (solo lógica de negocio)
Costo de la tercera vertical:   ███                   (menos aún)
Costo de la cuarta vertical:    ██                    (casi solo configuración)
```

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
