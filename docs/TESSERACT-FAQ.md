# Tesseract — Preguntas Técnicas de Producción

> Respuestas basadas en el prototipo funcional (48+ tests) y la formalización teórica.

---

## 1. Coordenadas: asignación, fijación, migración

### ¿Cada usuario/organización tiene coordenadas fijas?

Sí. Deterministas. El mapper asigna coordenadas por hash del identificador:

```
o-axis = hash(org_id) % S     → posición fija por identidad
c-axis = hash(channel) % S    → posición fija por contexto
t-axis = timestamp / bucket_size % S  → posición por tiempo
v-axis = hash(event_id) % S   → posición por evento único
```

Misma org = mismo o-axis siempre. Es determinista desde cualquier nodo, sin coordinación.

### ¿Cómo se asignan?

No se "asignan" — se derivan. Cualquier nodo puede calcular la coordenada de cualquier evento con los mismos inputs. No hay registro central de coordenadas. Es como una función hash: el input determina el output.

### ¿Cómo migras coordenadas?

No se migran. En el modelo espaciotemporal, tu historia ES tu posición. Si una organización cambia de nombre o se fusiona:

1. Las cristalizaciones existentes en la coordenada vieja **siguen existiendo** — son hechos geométricos inmutables.
2. La nueva identidad crea deformaciones en su nueva coordenada.
3. Un evento de "enlace" conecta ambas regiones: un seed que referencia a ambas, creando un puente orbital entre la posición vieja y la nueva.

No hay "migración" porque no hay base de datos que actualizar. Hay geometría que se extiende.

---

## 2. Latencia y velocidad de cristalización

### ¿Cuánto tarda en cristalizar?

La cristalización es **local e inmediata**. El seed crea una deformación y el campo local cristaliza sin esperar a nadie. Tiempos medidos:

| Operación | Tiempo (debug build) |
|---|---|
| Seed de un evento | 3-6 ms |
| Cristalización local | Instantánea al seed (p ≥ Θ al centro) |
| Evolución a equilibrio (8⁴) | ~100 ms |
| Propagación por boundary exchange | 1 RTT por salto |

### ¿Qué pasa con latencia real de red?

Con 50-300ms de latencia (Chile ↔ Europa):

- **Seed**: instantáneo en el nodo local. No espera red.
- **Propagación**: la deformación se propaga por boundary exchange. Cada ronda = 1 RTT.
- **Visibilidad global**: diámetro de la red × RTT.
  - Red de 4 nodos, 100ms links: ~400ms para visibilidad total.
  - Red de 20 nodos, 200ms links: ~2-4 segundos (gossip, no all-to-all).

Comparación:

| Sistema | Tiempo a finality |
|---|---|
| Bitcoin | ~60 minutos (6 confirmaciones) |
| Ethereum | ~12 segundos |
| BFT (Fabric) | 1-3 segundos |
| **Tesseract** | **Inmediata local + propagación en RTTs** |

### ¿La finality es probabilística o determinística?

**Determinística.** Una vez cristalizado, el estado es un punto fijo de una contraction mapping (Teorema de Convergencia). No es "probablemente final" — es matemáticamente final. La cristalización es irreversible bajo evolución (solo reversible por destrucción externa, y se auto-repara).

Esto es más fuerte que Bitcoin (probabilístico) y equivalente a BFT (determinístico) pero sin la condición de honest majority.

---

## 3. Ancho de banda y escalabilidad

### ¿Propagar distribuciones de probabilidad es costoso?

No propagamos distribuciones completas. Propagamos **celdas de frontera** — solo las celdas en el borde de cada región del nodo.

Datos por celda en boundary exchange:
```
Coord:        4 × 8 bytes = 32 bytes
Probability:  8 bytes
Crystallized: 1 byte
Influences:   variable (solo las > INFLUENCE_EPSILON)
─────────────────────────────
Total:        ~50-100 bytes por celda (sin influences pesadas)
```

Con SEED_RADIUS=4, una boundary típica tiene ~100-500 celdas: **5-50 KB por ronda de exchange**. Comparable a una transacción de Bitcoin (~250 bytes) o un bloque ligero.

### ¿Qué pasa con 100,000 nodos?

No necesitan all-to-all. El protocolo de propagación es **gossip**:

1. Cada nodo intercambia con sus vecinos de región (los que comparten frontera en el campo 4D).
2. La deformación se propaga como una onda — cada salto cubre un RTT.
3. En una topología razonable (mesh, ~10 vecinos por nodo), la propagación alcanza todos los nodos en O(log N) saltos.

Para 100K nodos:
```
Saltos = log₁₀(100,000) ≈ 5
Latencia = 5 × RTT_promedio
Con RTT 100ms: ~500ms para propagación global
```

**Escalabilidad de storage**: con campo sparse, cada nodo solo almacena las celdas de su región. Un nodo no necesita el campo entero — solo su proyección. Medido: en un campo 32⁴ (1M lógicas), un nodo con 2 eventos almacena ~25K celdas = 2.4% del campo.

### ¿Y el cómputo de evolución?

`evolve()` es O(celdas_activas), no O(campo_total). Y es trivialmente paralelizable — cada celda depende solo de sus 8 vecinos. En GPU o multi-thread, un step de evolución con 25K celdas activas se procesa en microsegundos.

---

## 4. Compatibilidad con casos de uso reales

### Private data collections

Mapeo directo al **eje c (channel/context)**:

```
channel: "private-org1-org2"  →  c-axis = hash("private-org1-org2") % S
```

Canales diferentes = regiones diferentes del campo. Los orbitales no cruzan entre canales a menos que un evento sea explícitamente sembrado en ambos. La privacidad es geométrica — los datos de un canal no tienen probabilidad en otro canal.

### Channels multi-organización

Múltiples orgs en un mismo channel comparten el eje c pero difieren en eje o:

```
Evento de org1 en channel "supply":  (t, hash("supply"), hash("org1"), v)
Evento de org2 en channel "supply":  (t, hash("supply"), hash("org2"), v)
```

Mismo canal = mismo c-axis. Diferentes orgs = diferentes o-axis. Los orbitales se solapan en el eje c → conexión emergente por proximidad geométrica, sin necesidad de "endorsement" explícito.

### Endorsement policies (AnyOf, AllOf)

El endorsement se convierte en **densidad de deformación**:

- **AnyOf(N)**: basta que 1 de N orgs siembre el evento. El orbital de una sola org es suficiente para cristalizar.
- **AllOf(N)**: todas las N orgs siembran el mismo evento (distributed seeding). El overlap de N orbitales produce cristalización más fuerte que cualquier orbital individual. Si falta una org, el orbital combinado puede no alcanzar el umbral.
- **NOutOf(K, N)**: al menos K de N orgs siembran. El umbral de cristalización se calibra para que K orbitales sumados superen Θ pero K-1 no.

La política de endorsement no es una regla — es una **propiedad geométrica** del campo. Más participantes = más orbitales solapados = cristalización más fuerte.

### Smart contracts Wasm

Un contrato es una **deformación con lógica**:

1. Desplegar contrato = sembrar un evento en una coordenada fija (hash del contract_id).
2. Ejecutar contrato = un nuevo evento cuya coordenada está determinada por contract_id + inputs.
3. El resultado de la ejecución se registra en las influences de la cristalización.
4. El estado del contrato = la región del campo alrededor de su coordenada.

El Wasm se ejecuta off-chain (como en Fabric). El resultado se siembra en el campo. La cristalización confirma el resultado.

### Auditoría regulatoria (CMF, SII, eIDAS)

Cada celda cristalizada retiene sus **influences**: qué eventos la crearon, con qué peso, desde qué org. Eso ES el audit trail.

Exportar para regulador:

```sql
-- Pseudo-query sobre el campo
SELECT coord, event_id, weight, org, timestamp
FROM crystallized_cells
WHERE region = hash("org-auditada")
ORDER BY timestamp
```

Resultado: tabla plana con trazabilidad completa. Cada registro muestra:
- Qué se cristalizó (coordenada)
- Quién lo causó (influences con event_id y org)
- Cuándo (eje t → timestamp)
- Con qué fuerza (weight)

Para eIDAS: las influences incluyen el identificador de la org firmante. La cristalización con influences de múltiples orgs es equivalente a un documento multi-firmado.

Para CMF/SII: el audit trail es la lista de cristalizaciones en la región de la entidad regulada, con provenance completa. No hay "bloque" que inspeccionar — hay una región del espacio con todas las deformaciones registradas.

---

## 5. Resumen de comparación práctica

| Aspecto | Blockchain (Fabric) | Tesseract |
|---|---|---|
| Finality | Determinística (BFT) | Determinística (convergencia) |
| Latencia | 1-3s (3 rondas BFT) | Instantánea local + RTTs de propagación |
| Bandwidth por tx | ~2-5 KB (bloque + endosos) | ~50-100 bytes (boundary cell) |
| Storage por nodo | Cadena completa del canal | Solo celdas de su región (~2-5% del campo) |
| Private data | Collections explícitas | Separación por eje c (geométrica) |
| Endorsement | Políticas evaluadas por gateway | Densidad orbital (propiedad del campo) |
| Audit trail | Bloques + transacciones | Cristalizaciones + influences |
| Escalabilidad | ~3000 TPS (Fabric 2.5) | Por determinar a escala de producción |
| Fault tolerance | < n/3 Byzantine | Cualquier cantidad de destrucciones |
| Quantum safe | Requiere PQC (ML-DSA-65) | Inherente (no usa crypto para seguridad) |

---

*Estas respuestas son teóricas para los puntos no implementados (gossip, 100K nodos) y experimentales para los implementados (latencia de seed, boundary exchange, audit trail). Los números de producción requieren un prototipo distribuido real.*
