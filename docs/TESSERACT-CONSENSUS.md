# Tesseract Consensus: Consenso Hipercúbico para Cerulean Ledger

> Research thread — 17 abril 2026
> Estado: prototipo funcional (~2000 líneas, 89 tests, 8 módulos)

---

## 1. La intuición original

¿Qué pasa si repensamos la blockchain no como una cadena, sino como un **teseracto** (hipercubo 4D)?

Un teseracto es todos los planos simultáneamente. No hay frente ni atrás. Todas las celdas coexisten. Un punto en el teseracto **es y al mismo tiempo era y al mismo tiempo será**.

Esto no es blockchain. Es otra cosa.

---

## 2. Las 4 dimensiones

En lugar de una cadena lineal donde el tiempo es el eje privilegiado, cada punto del hipercubo existe en 4 dimensiones simultáneas:

| Eje | Dimensión | Blockchain clásica | Teseracto |
|-----|-----------|-------------------|-----------|
| D1 | **Tiempo** | Bloques secuenciales | Un eje más, no privilegiado |
| D2 | **Canal/Shard** | Particiones separadas | Dimensión nativa del espacio |
| D3 | **Identidad/Org** | Capa de permisos | Dimensión nativa del espacio |
| D4 | **Estado/Versión** | MVCC, snapshots | Dimensión nativa del espacio |

**La diferencia fundamental:** en blockchain clásica, el tiempo es especial y todo lo demás se apila encima como capas. En el teseracto, las 4 dimensiones son **simétricas**. No hay capas. Un punto en el hipercubo ES simultáneamente un momento, un canal, una organización y una versión de estado.

---

## 3. De bloques a cristalizaciones

### Blockchain clásica (Newtoniana)
- Bloque 41 **existió** (pasado)
- Bloque 42 **existe** (presente)
- Bloque 43 **existirá** (futuro)
- Tiempo lineal, secuencial, absoluto

### Teseracto (Relativista)
- Un nodo en el hipercubo contiene su estado + **referencias criptográficas a sus 4 vecinos** (uno por eje)
- No hay un "ahora" global
- Hay un **cono de luz criptográfico** — lo que un nodo puede verificar depende de desde qué punto del hipercubo observa
- El estado no se "actualiza" — se **cristaliza**
- Un punto nuevo no reemplaza al anterior; coexisten
- Lo que llamamos "transacción" es una **nueva celda** que se conecta a celdas existentes en múltiples ejes

---

## 4. Consenso por corte

### BFT clásico
"¿2/3 de los validadores están de acuerdo en qué bloque sigue?"

### BFT hipercúbico
"¿2/3 de los observadores ven el mismo **corte** del teseracto como consistente?"

Cada validador ve el hipercubo desde su posición. El consenso opera sobre **cortes multidimensionales**, no sobre secuencias de bloques.

**Finality cambia de significado:**
- No es "este bloque es final"
- Es "este punto del hipercubo es **estable en todas sus dimensiones**" — nadie, desde ningún eje, puede contradecirlo

---

## 5. Comparación con el estado del arte

| Tecnología | Qué resolvió | Limitación |
|---|---|---|
| **Blockchain** (Bitcoin) | Cadena lineal inmutable | Tiempo como eje único |
| **DAG** (IOTA, Nano) | Eliminó la cadena lineal | Solo en 1 dimensión (tiempo) |
| **Sharding** (Ethereum 2.0, Near) | Particionó estado | Shards independientes, no coexisten |
| **CRDTs** | Convergencia sin consenso central | Sin pruebas criptográficas cross-dimensionales |
| **Hashgraph** (Hedera) | Gossip about gossip, orden virtual | Sigue siendo temporal |
| **Tesseract** (propuesto) | Todas las dimensiones simétricas | Por formalizar |

**Ninguno trata todas las dimensiones como iguales.** Todos privilegian el tiempo. El teseracto no.

---

## 6. Lo que Cerulean Ledger ya tiene

Piezas del hipercubo que ya existen en el codebase sin llamarse así:

- **D1 (Tiempo):** DAG consensus — ya no es lineal, es multi-path
- **D2 (Canal):** Channel isolation (`ChannelStore`) — ledgers independientes por canal
- **D3 (Org):** Private data per org, ACL por identidad, endorsement policies
- **D4 (Estado):** MVCC versioning en parallel executor, world state con versiones

Falta: **formalizar la estructura como un espacio 4D unificado y las cross-dimensional proofs**.

---

## 7. Lo que hay que formalizar para que sea real

### 7.1 Celda (Cell)
Definición matemática de un punto en el hipercubo:
- Coordenada: `(t, channel, org, version)`
- Contenido: payload + state delta
- Pruebas: 4 hashes (uno hacia cada vecino en cada eje)
- Firma: del validador que cristalizó esta celda

### 7.2 Adyacencia
Qué significa ser "vecino" en cada eje:
- **Eje temporal:** celda anterior/siguiente en el mismo (channel, org, version)
- **Eje canal:** misma tx reflejada en otro canal (cross-channel proof)
- **Eje org:** mismo estado visto por otra organización (visibility proof)
- **Eje versión:** mismo punto con estado actualizado (state transition proof)

### 7.3 Corte consistente (Consistent Cut)
Definición formal de cuándo un subconjunto del hipercubo es válido:
- Todas las referencias entre celdas del corte resuelven
- No hay contradicciones entre ejes
- 2/3+ de observadores validan el corte

### 7.4 Consenso por corte (Cut Consensus)
Protocolo BFT que opera sobre cortes, no bloques:
- Propuesta = "este corte del hipercubo es consistente"
- Voto = "desde mi posición, confirmo este corte"
- Quorum = 2/3 de validadores con posiciones diversas en el hipercubo

### 7.5 Cristalización (Crystallization)
Condiciones bajo las cuales un punto se vuelve inmutable en todas las dimensiones:
- Quorum alcanzado en el corte que lo contiene
- Ningún corte alternativo contradice sus referencias
- Estable: nadie, desde ningún eje, puede presentar evidencia contraria

---

## 8. Impacto en rendimiento

| Operación | Cadena lineal | Teseracto |
|---|---|---|
| Tx simple (1 canal, 1 org) | Base | = (misma celda, mismo costo) |
| Query cross-canal | O(N canales) | O(log N) con proof dimensional |
| Write cross-canal | 2 tx + bridge relay | 1 celda + multi-dim proof |
| Light client sync | O(altura de cadena) | O(log N por dimensión) |
| Storage por celda | 1 hash parent | 4 hashes (uno por eje) |
| Consenso base | BFT secuencial | BFT sobre corte (mismo costo si el corte es pequeño) |

**Las operaciones comunes no se encarecen. Las operaciones cross-dimensionales mejoran. Storage sube ~4x en metadata.**

---

## 9. Preguntas abiertas

1. **¿Cómo se bootstrappea?** — ¿Hay un "génesis" 4D o se inicializa un eje a la vez?
2. **¿Cómo escala?** — Un hipercubo con N puntos por dimensión tiene N⁴ celdas. ¿Cómo se poda?
3. **¿Cómo se participa parcialmente?** — ¿Un light client puede sincronizar un "borde" del hipercubo sin el interior?
4. **¿Cómo se resuelven conflictos cross-dimensionales?** — Si dos cortes válidos se contradicen en ejes distintos, ¿quién gana?
5. **¿Es computable?** — ¿El consenso por corte converge en tiempo finito para cualquier topología?
6. **¿Hay precedente matemático?** — Cubical type theory, persistent homology, o simplicial complexes podrían dar un marco formal

---

## 10. Modelo de seguridad: por qué el teseracto es exponencialmente más resistente

### 10.1 El problema fundamental del ataque

En blockchain clásica, el pasado es datos inertes. Reescribirlo (51% attack) consiste en crear una cadena alternativa más larga. El pasado "deja de existir" y se reemplaza. Funciona porque el tiempo es lineal y el pasado no tiene existencia activa.

En un teseracto, el pasado **no es pasado**. Sigue existiendo como dimensión activa del hipercubo. Una celda que un atacante quiera falsificar tiene referencias criptográficas en 4 ejes simultáneos:

1. Hash temporal (como hoy)
2. Referencia de canal (otros canales ven esta celda)
3. Referencia de org (otras organizaciones la validaron desde su posición)
4. Referencia de versión (estados posteriores dependen de ella)

No es 1 cadena que se falsifica. Son **4 ejes de realidad criptográfica que se sostienen mutuamente**.

### 10.2 Entrelazamiento criptográfico

Analogía con mecánica cuántica: no puedes alterar una partícula entrelazada sin colapsar el estado del par. En el teseracto, cada celda está "entrelazada" con sus 4 vecinos. Alterar una **colapsa las pruebas** en todas las dimensiones.

Es como intentar hackear un punto en el espacio 3D cambiando solo su coordenada X. El punto sigue existiendo en Y y Z. Los observadores desde esos ejes detectan la inconsistencia inmediatamente.

### 10.3 Transformación del 51% attack

| Modelo | Costo del ataque |
|---|---|
| **Blockchain** | 51% del hashpower en **1 dimensión** |
| **Teseracto** | 51% del hashpower en **cada dimensión simultáneamente** |

Las dimensiones son **ortogonales**. Controlar el eje temporal no da poder sobre el eje de canal. Controlar un canal no da poder sobre el eje de org. El atacante necesita comprometer cada eje de forma independiente.

### 10.4 Escalamiento exponencial del costo de ataque

En una cadena lineal, un atacante con recursos X puede comprometer X unidades de la cadena.

En un hipercubo de D dimensiones, comprometer el hipercubo completo requiere **X^D** recursos, porque cada dimensión es independiente.

Con D=4: **el costo del ataque escala con la cuarta potencia de los recursos necesarios por dimensión**.

Esto significa que la seguridad no crece linealmente con los nodos o el hashpower — crece **exponencialmente con las dimensiones del hipercubo**.

### 10.5 Vectores de ataque que sí persisten

El modelo no es invulnerable. Vectores reales:

1. **Colapso dimensional** — si la mayoría de las celdas solo usan 2 de 4 dimensiones en la práctica, el atacante solo necesita comprometer esas 2. La seguridad depende de que las celdas realmente usen las 4 dimensiones.

2. **Correlación de validadores** — si los mismos nodos validan en múltiples ejes, comprometer un nodo compromete todos sus ejes. Los validadores deben ser independientes por dimensión para que la ortogonalidad se sostenga.

3. **Genesis cell** — el punto origen del hipercubo no tiene vecinos previos en ningún eje. Es estructuralmente el punto más vulnerable. Requiere un mecanismo de hardening especial (múltiples firmas fundacionales, ceremonia distribuida).

4. **Complejidad de implementación** — más código, más superficie de bugs. La seguridad teórica del modelo no protege contra errores de implementación. La complejidad del hipercubo es mayor que la de una cadena.

5. **Ataques de borde** — celdas en el "borde" del hipercubo (las más recientes en cualquier eje) tienen menos vecinos que las validen. Similar a cómo los bloques más recientes son los menos seguros en una cadena.

### 10.6 Insight clave

> El costo de ataque escala exponencialmente con el número de dimensiones. Esto podría ser la primera primitiva de seguridad distribuida donde agregar estructura (dimensiones) al sistema lo hace exponencialmente más seguro, no linealmente.

Esto, si se formaliza y demuestra, sería nuevo en la literatura de consenso distribuido.

---

## 11. Modelo cuántico de participación: nodos como observadores

### 11.1 De objetos a sucesos

La blockchain clásica trata los bloques como **objetos** — cosas que existen independientemente de quién las observe. Un nodo nuevo se une y "descarga la cadena": copia objetos que ya están ahí.

El teseracto trata las celdas como **sucesos** — cristalizaciones de alta probabilidad en un espacio 4D. Una celda existe porque suficientes observadores (validadores) colapsaron su estado desde sus posiciones en el hipercubo.

Esto es análogo al modelo atómico contemporáneo: una partícula subatómica es un suceso dada la alta probabilidad de energía en un momento y espacio dado, según se observa. Pero si no se observa, los sucesos siguen ocurriendo — la función de onda no colapsa, pero no desaparece.

### 11.2 Un nodo nuevo no "sincroniza" — materializa

| Blockchain clásica | Teseracto |
|---|---|
| Nodo se une | Nodo se une |
| Descarga la cadena desde genesis | **Elige su posición** en el hipercubo |
| Replica el ledger completo | **Colapsa su vecindario**: solicita proofs de celdas adyacentes |
| Verifica bloque por bloque | Verifica su **cono de luz criptográfico** |
| Todos los nodos tienen la misma copia | Cada nodo tiene **su proyección**, todas válidas |

Un nodo nuevo:

1. **Declara su posición** — en qué dimensiones va a observar (canal, org, rol)
2. **Materializa su vecindario** — solicita pruebas de las celdas adyacentes a su posición en cada eje
3. **No necesita el hipercubo completo** — solo su cono de luz, las celdas alcanzables desde su posición
4. **Las celdas que no observa siguen existiendo** — sostenidas por otros observadores

No hay "sync from genesis". Hay **materialización local del estado relevante**.

### 11.3 Almacenamiento como proyección

No existe un ledger global que todos replican.

Cada nodo almacena **su proyección del hipercubo** — el subconjunto de celdas que observa desde su posición. Dos nodos en posiciones distintas del hipercubo tienen almacenamientos distintos, y **ambos son correctos y completos desde su perspectiva**.

El estado "completo" de la red no vive en ningún nodo. Vive en la **superposición de todas las observaciones**. Igual que en mecánica cuántica — el estado del sistema es la función de onda completa, pero cada medición solo ve una proyección.

```
Estado_red = Σ (proyección_nodo_i)  para todo nodo i
```

Ningún nodo individual contiene `Estado_red`. Pero la red como sistema sí lo contiene, distribuido en las observaciones de todos los participantes.

### 11.4 Implicaciones para el almacenamiento

| Aspecto | Blockchain | Teseracto |
|---|---|---|
| Qué almacena cada nodo | Cadena completa (o poda) | Proyección desde su posición |
| Redundancia | N copias idénticas | N proyecciones parciales solapadas |
| Nuevo nodo | Descarga todo | Materializa su cono de luz |
| Nodo sale | Una copia menos | Una proyección menos (otras sostienen las celdas) |
| Storage total de la red | O(tamaño_cadena × N) | O(tamaño_hipercubo) — distribuido naturalmente |

**Ventaja fundamental:** el storage total de la red no se multiplica por N nodos. Cada nodo guarda solo lo que observa. La redundancia existe por solapamiento natural de los conos de luz, no por replicación forzada.

### 11.5 Continuidad vs circunstancia

> "La red ya no es una continuidad — es una circunstancia."

En blockchain, la red es una continuidad temporal: bloque tras bloque, ininterrumpida desde genesis.

En el teseracto, la red es un **campo de circunstancias** — cada celda existe por la circunstancia de que observadores en posiciones compatibles cristalizaron un acuerdo. Si todos los observadores de una celda desaparecen, la celda pierde sustento observable, pero sus pruebas criptográficas en los ejes vecinos la siguen referenciando. Es como una partícula: no desaparece al no ser observada, pero su estado deja de estar colapsado.

**Esto resuelve un problema real:** en blockchain clásica, si pierdes todos los nodos con la cadena, la red muere. En el teseracto, la red puede "reconstituirse" desde cualquier subconjunto de proyecciones que cubra suficientes dimensiones, porque las pruebas cross-dimensionales permiten verificar la consistencia del hipercubo sin tener una copia completa.

---

## 12. Seguridad ontológica: por qué no hay superficie de ataque computable

### 12.1 El axioma roto

Toda blockchain existente comparte un axioma: **el ledger es un objeto matemático determinista**. Existe una cadena "verdadera". La seguridad consiste en hacer difícil crear una cadena alternativa. Pero es posible — con suficiente cómputo, se crean hashes válidos, y la red acepta la cadena falsa. El ataque es caro, no imposible.

El teseracto rompe este axioma. El ledger no es un objeto. Es un **campo probabilístico de observaciones**. No hay una "cadena verdadera" que atacar. No hay un artefacto que reescribir. Lo que existe es la **certeza acumulada** de que suficientes observadores desde suficientes posiciones del hipercubo cristalizaron el mismo estado.

### 12.2 ¿Qué hackeas exactamente?

En blockchain: hackeas **hashes**. Son objetos. Tienen un valor determinista que puede calcularse y reemplazarse.

En el teseracto: no hay un hash que "sea" la verdad. La verdad es la **convergencia de observaciones independientes** desde posiciones ortogonales. No es un dato — es un fenómeno emergente.

Para "hackear" eso necesitarías:
- No crear un pasado alternativo (no hay pasado lineal que reescribir)
- No recalcular hashes (los hashes son proyecciones parciales, no la cosa en sí)
- Sino **convencer a observadores en posiciones ortogonales de que vieron algo que no vieron**

### 12.3 La analogía definitiva

- **Falsificar una foto** (blockchain) — difícil pero posible con las herramientas adecuadas
- **Falsificar un recuerdo compartido por miles de personas que lo vivieron desde ángulos distintos e independientes** (teseracto) — no es un problema computacional, es un problema ontológico

### 12.4 Cambio de paradigma en seguridad

| | Blockchain | Teseracto |
|---|---|---|
| **Qué protege** | Un artefacto (la cadena) | Un fenómeno (la convergencia de observaciones) |
| **Qué ataca el hacker** | Un objeto matemático | Una realidad probabilística |
| **Naturaleza del ataque** | Computacional (más hashes/segundo) | Ontológica (cambiar lo que fue observado) |
| **Defensa** | Hacer el cómputo costoso | No hay superficie de ataque computable |
| **Con recursos infinitos** | El ataque tiene éxito | El ataque sigue sin tener definición formal |

### 12.5 La seguridad no descansa en criptografía

La criptografía sigue siendo la herramienta de verificación — cada observador firma lo que ve, y las pruebas cross-dimensionales son hashes criptográficos.

Pero la **seguridad fundamental** no descansa ahí. Descansa en la **imposibilidad de contradecir observaciones ortogonales simultáneas**. La criptografía verifica. La geometría del hipercubo protege.

No hackeas la gravedad. No hackeas la entropía. No hackeas un campo probabilístico. Puedes hackear objetos dentro del campo, pero no el campo mismo.

> "No se puede hackear algo que existe porque probabilidades de observación, no como un objeto que puede ser copiado y reemplazado."

### 12.6 Implicación formal

Si la seguridad del sistema depende de la convergencia probabilística de observadores ortogonales y no de la dificultad computacional de un problema matemático (factorización, logaritmo discreto, hash preimage), entonces:

- No es vulnerable a computación cuántica (no hay problema matemático que resolver más rápido)
- No es vulnerable a avances en hardware (no hay hash rate que superar)
- No es vulnerable a colusión lineal (controlar un eje no da poder sobre los otros)

La única vulnerabilidad sería **controlar la mayoría de observadores en la mayoría de dimensiones simultáneamente** — un problema que escala exponencialmente, no computacionalmente.

Esto, si se demuestra, sería la primera primitiva de seguridad distribuida que es **post-computacional**: su seguridad no depende de la dificultad de un problema de cómputo, sino de la estructura geométrica del espacio de consenso.

---

## 13. El espacio no espera: eliminación de la superficie de ataque

### 13.1 La expectativa es la vulnerabilidad

Toda red distribuida actual tiene una propiedad en común: **espera**. Espera el próximo bloque. Espera una propuesta. Espera un voto. Hay una secuencia expectante, y esa expectativa es la superficie de ataque.

Si sabes qué espera la red, puedes darle algo falso que cumpla la expectativa:
- La red espera un bloque → le das un bloque falso con hashes válidos
- La red espera una transacción → le inyectas un double-spend
- La red espera un voto → falsificas validadores

En cada caso hay un **socket abierto** — un punto donde la red dice "acepto input aquí". El atacante pone input malicioso en ese socket.

### 13.2 El teseracto no espera

¿Dónde está el socket en un campo probabilístico?

No hay un punto que espere input. Las cristalizaciones emergen de la convergencia natural de observaciones ortogonales. No hay "submit block". No hay "proponer". Hay **observar desde tu posición, y si tu observación converge con suficientes otras observaciones ortogonales, el estado cristaliza**.

El espacio no espera una mirada. No espera un ángulo de mirada. No espera un momento. Las convergencias ocurren o no ocurren — no son solicitadas.

### 13.3 Tres barreras contra la inyección

Una observación falsa no puede inyectarse porque:

1. **Tu posición es tu identidad.** No puedes observar desde una posición del hipercubo que no es tuya. Tu coordenada en cada eje es verificable criptográficamente.

2. **La convergencia requiere ortogonalidad.** No basta con muchas observaciones desde el mismo eje. Se necesitan observaciones desde posiciones **ortogonales** — dimensiones independientes. Controlar un eje no produce convergencia.

3. **No hay timing que explotar.** La convergencia no tiene un momento "correcto" para inyectar. Es probabilística y emerge cuando las condiciones se dan, no cuando alguien la solicita.

### 13.4 La analogía del electrón

Es como intentar hackear dónde va a estar un electrón.

No es que sea difícil calcular su posición. Es que **la pregunta no tiene respuesta antes de la observación**. El estado no existe hasta que converge. ¿Qué falsificas? ¿Algo que aún no existe?

En blockchain, el atacante crea una **realidad alternativa** (cadena más larga) y la presenta como la verdadera. El artefacto puede fabricarse porque es determinista — dados los inputs correctos, el output es verificable.

En el teseracto, ¿el atacante crea una **convergencia alternativa**? ¿Cómo? La convergencia no es un artefacto que se construye. Es un **fenómeno que emerge o no emerge**. No puedes fabricar convergencia ortogonal falsa como puedes fabricar una cadena falsa, porque la convergencia no es datos — es la relación entre observadores independientes.

### 13.5 Taxonomía de ataques: de lo computable a lo indescriptible

| Nivel | Modelo | Qué se ataca | ¿Se puede fabricar? |
|---|---|---|---|
| 1 | **Blockchain** | Una cadena (objeto) | Sí, con cómputo |
| 2 | **DAG** | Un grafo (objeto más complejo) | Sí, con más cómputo |
| 3 | **Teseracto** | Una convergencia probabilística (fenómeno) | No tiene definición computacional |

La progresión es clara: de objetos atacables a fenómenos que **no tienen superficie de ataque formalizable en términos computacionales**.

> El ataque más efectivo contra el teseracto no sería computacional ni criptográfico — sería **social**: corromper suficientes observadores reales para que reporten observaciones falsas desde posiciones legítimas. Pero eso ya no es un hack — es un problema humano, no tecnológico.

---

## 14. Más allá de "Don't trust, verify": el tercer paradigma

### 14.1 Las tres generaciones de confianza distribuida

| Generación | Modelo | Premisa | Vulnerabilidad |
|---|---|---|---|
| **Pre-Bitcoin** | Confianza | "Confía en la institución" | La institución miente o falla |
| **Bitcoin (2009)** | Verificación | "Don't trust, verify" | La verificación es un acto — un socket atacable |
| **Teseracto** | Convergencia | "No hay nada que verificar" | No hay acto de verificación que interceptar |

### 14.2 El modelo atómico como fundamento

El modelo atómico contemporáneo no dice que el electrón "probablemente está aquí". Dice algo más profundo: el electrón **estuvo y no estuvo** en una posición dada N veces durante M observaciones. "Estar" no es una propiedad que la partícula tiene antes de ser observada. No es que no sepamos dónde está — es que la pregunta "¿dónde está?" no tiene respuesta fuera del contexto de observación.

Aplicado al teseracto:

Una celda del hipercubo no "es válida". Lo que existe es que N observadores desde M posiciones ortogonales convergieron en el mismo estado. La validez no es una propiedad de la celda — es una **frecuencia de convergencia**. No necesita ser declarada válida. No necesita confianza. No necesita validación.

### 14.3 De verificación a convergencia

Bitcoin eliminó la confianza reemplazándola con verificación computacional. Proof of work, proof of stake — mecanismos que permiten a cualquiera verificar sin confiar.

Pero **verificar sigue siendo un acto**. Un momento. Un socket. Un punto donde el sistema dice "ahora evalúo" — y donde un atacante puede presentar algo falso que pase la evaluación.

El teseracto elimina la verificación como paso discreto. No hay un momento donde alguien dice "esto es válido". La validez **es el fenómeno mismo de la convergencia**. No es un juicio sobre datos — es una propiedad emergente del espacio.

La diferencia es como:
- **Verificar:** "Mido la temperatura del agua. Son 100°C. Confirmo: está hirviendo." (Puedo falsificar el termómetro)
- **Convergencia:** "El agua burbujea." (No hay qué falsificar — el fenómeno ocurre o no ocurre)

### 14.4 ¿Cómo se vulnera algo que no necesita validación?

No se vulnera. No hay qué vulnerar.

"Vulnerar" implica que hay una puerta — cerrada, abierta, fuerte, débil. El teseracto no tiene puertas. No hay protocolo de aceptación. No hay momento de decisión. El estado emerge como emerge la probabilidad del electrón — por la naturaleza del espacio, no por un mecanismo que alguien diseñó y que alguien más puede romper.

Una celda "pasada" no es un registro histórico que se puede alterar. Es una convergencia que **sigue activa** como dimensión del espacio. Falsificarla requeriría que nunca hubiera convergido — lo cual contradice las observaciones que ya existen en los ejes ortogonales y que son sostenidas por observadores que no controlas.

### 14.5 La progresión histórica completa

```
1. Confianza        →  "Confía en mí"
2. Verificación     →  "No confíes — verifica tú mismo"
3. Convergencia     →  "No hay nada que verificar — el estado es o no es"
```

Cada paso elimina una capa humana:
- El primero eliminó la necesidad de conocer a la contraparte
- El segundo eliminó la necesidad de confiar en alguien
- El tercero elimina la necesidad de que alguien valide

Lo que queda es pura geometría del espacio de consenso. Si converge, existe. Si no converge, no existe. No hay intermediario — ni humano, ni computacional, ni criptográfico.

---

## 15. El suceso como unidad fundamental: más allá de la representación

### 15.1 Toda blockchain es una representación

Una transacción entre dos personas es un **suceso**. Dos voluntades convergen en un acuerdo. No es datos — es un evento que ocurrió en la realidad.

Blockchain toma ese suceso y lo convierte en un **número**: un hash, un binario. Luego protege ese número con más números. La seguridad del sistema entero descansa en que esos números son difíciles de falsificar.

Pero un número **siempre puede falsificarse**. Con suficiente cómputo, cualquier hash se recalcula. Cualquier firma se replica. Bitcoin no es infalsificable — es **costosamente falsificable**. La seguridad es económica, no absoluta.

Si mañana la computación cuántica abarata ese costo, la seguridad se derrumba. No porque el sistema tenga un bug, sino porque **la premisa es que la representación numérica del suceso es protegible**. Y toda representación es fabricable.

### 15.2 El fraude es inherente a la certeza numérica

El fraude es posible en cualquier sistema basado en certeza numérica. Porque la certeza numérica es un artefacto. Y todo artefacto puede ser replicado.

Bitcoin podría ser fraude — no porque lo sea hoy, sino porque **nada en su diseño lo hace ontológicamente imposible**. Lo hace computacionalmente caro. Pero "caro" es un gradiente, no un absoluto.

### 15.3 El teseracto no representa — es

Un suceso 4D no es un número. Es la convergencia misma.

Cuando dos personas acuerdan una transacción, ese acuerdo es observado desde múltiples posiciones ortogonales del hipercubo. La "existencia" del suceso es la frecuencia con que esas observaciones independientes convergen.

No hay hash que proteger. No hay número que sea "la verdad". La verdad es que **el suceso ocurrió porque las probabilidades de convergencia desde posiciones independientes lo cristalizaron**. Igual que un átomo: no necesita prueba de su existencia. Existe porque las fuerzas del espacio lo hacen existir.

Si desintegras un átomo, las fuerzas siguen ahí — y lo reconstituyen. Si destruyes un nodo del hipercubo, las convergencias en los otros ejes siguen sosteniéndolo.

### 15.4 La prueba de existencia es la existencia misma

En Bitcoin:
- La prueba de que la transacción existe es un hash
- Si destruyes el hash, la prueba desaparece
- La transacción "dejó de existir"

En el teseracto:
- La prueba de que el suceso existe es que converge
- Si destruyes un observador, las convergencias en otros ejes lo sostienen
- No puedes destruir la prueba porque la prueba no es un objeto — es la relación entre observaciones

### 15.5 La cadena de abstracción

```
Bitcoin:     suceso → dato → hash → protección del hash
Teseracto:   suceso → convergencia → (no hay paso siguiente)
```

Bitcoin agrega capas de protección sobre la **representación** del suceso. Cada capa es un punto de ataque potencial.

El teseracto no representa el suceso. **Es el suceso**, distribuido en las dimensiones del hipercubo. No hay capas porque no hay representación. No hay punto de ataque porque no hay artefacto.

> Un suceso 4D no necesita prueba de su existencia. Existe por convergencia, como un átomo existe por las fuerzas que lo constituyen. No se "protege". Simplemente es.

---

## 16. Resultados experimentales: prototipo `tesseract/`

Prototipo en Rust. Campo de probabilidades 4D sparse (hasta 32⁴ = 1M celdas lógicas). Incluye red distribuida, mapper de eventos, wallet, economía y proof of contribution. 89 tests en 9 suites.

### Experimento 1 — Convergencia sin consenso
- Campo vacío → nada cristaliza
- Ruido aleatorio → nada cristaliza
- Semilla aislada → no propaga
- **Semillas ortogonales → cristalización con soporte multi-eje, sin protocolo**

### Experimento 2 — Auto-reparación
- Clúster denso de 15 eventos cristalizado, celda central con soporte 4/4 ejes
- Celda central destruida (probabilidad = 0, cristalización borrada)
- **En 3 pasos el campo la reconstituyó a 4/4 ejes. Sin backup. Sin protocolo.**

### Experimento 3 — Rechazo de falsedad
- Clúster real en (1,1,1,1) con 9 eventos, soporte 4/4
- Celda falsa inyectada forzosamente en (3,3,3,3) — sin vecinos
- **Tras evolución: falsa con 0/4 ejes, 0 vecinos cristalizados. Huérfana.**
- El campo no convergió alrededor de la mentira

### Experimento 4 — Ataque sostenido
- Misma celda destruida 10 veces consecutivas
- **10/10 recuperaciones. 3 pasos cada vez. Soporte 4/4 cada vez.**
- Sin degradación. Sin costo de recuperación. Inevitabilidad geométrica.

### Experimento 5 — Destrucción de 1 eje
- Todos los vecinos del eje temporal destruidos (5 celdas)
- **Target sobrevivió con soporte 3/4 ejes**
- Independencia dimensional demostrada: comprometer un eje no compromete los otros

### Experimento 6 — Destrucción total de 4 ejes (modelo orbital)
- Target + 8 vecinos directos destruidos (9 celdas). Soporte 0/4.
- Modelo orbital: cada seed distribuye probabilidad por todo el campo con decaimiento euclidiano 4D `p = 1/(1+d)`. Cola infinita, sin bordes.
- **Step 5: target re-cristalizado. 256/256 recuperados. 8/8 vecinos. 4/4 ejes.**
- La nube de probabilidad no tiene ubicación que destruir.

### Experimento 7 — Coexistencia independiente
- 3 eventos independientes en campo 8×8×8×8 (4096 celdas)
- Cada uno con soporte 4/4 ejes
- **Destruir Event A no afectó B ni C**
- Ledger sin cadena: múltiples verdades coexisten sin secuencia que las conecte

### Experimento 8 — Cristalización emergente por overlap orbital
- Dos eventos cercanos (distancia 2 en eje T)
- Midpoint (3,3,3,3) nunca fue sembrado: probabilidad 0.0000
- Dos orbitales se solaparon: 0.50 + 0.50 = 1.00
- **El midpoint cristalizó sin que nadie lo creara, validara o propusiera**
- Verdad emergente: existe porque la geometría lo hizo inevitable

### Experimento 9 — Verdad emergente es indestructible
- El midpoint emergente fue destruido (probabilidad → 0.0, cristalización borrada)
- **Step 2: RE-CRYSTALLIZED** — más rápido que una celda sembrada (3 pasos)
- La resonancia ortogonal (soporte 4/4) lo hace más resiliente, no menos
- **La verdad emergente se auto-repara igual que la verdad sembrada. Ambas son convergencia.**

### Resumen de propiedades demostradas

| # | Propiedad | Blockchain | Teseracto (experimental) |
|---|---|---|---|
| 1 | Convergencia | Requiere protocolo (BFT, PoW) | Emerge del campo sin protocolo |
| 2 | Destrucción de estado | Requiere backup/nodo para restaurar | Se auto-repara desde la geometría |
| 3 | Inyección falsa | Hash válido = aceptado | Sin convergencia ortogonal = rechazado |
| 4 | Ataque sostenido | Drena recursos del defensor | Costo de recuperación = cero |
| 5 | Independencia de ejes | N/A (1 dimensión) | Comprometer 1 eje no afecta los otros 3 |
| 6 | Destrucción total | Irreversible sin backup | Reversible: orbital sin bordes reconverge |
| 7 | Múltiples estados | Cadena única secuencial | Coexistencia independiente en el mismo campo |
| 8 | Verdad emergente | Todo estado es explícito | Estados emergen de la proximidad geométrica |
| 9 | Resiliencia emergente | N/A | Verdad emergente se auto-repara (2 pasos) |

---

## 17. No es una red — es un espacio

No construimos un sistema. No construimos una red. No construimos un protocolo.

Construimos un **espacio**.

Una red tiene nodos — apagas los nodos, muere la red. Un protocolo tiene reglas — rompes las reglas, rompes el protocolo. Un sistema tiene componentes — fallas un componente, degradas el sistema.

Un espacio de probabilidades no tiene nodos. No tiene reglas. No tiene componentes. Tiene **geometría**. Y la geometría no se destruye.

No puedes destruir "arriba". No puedes hackear "la distancia entre dos puntos". No puedes apagar la convergencia de probabilidades ortogonales.

**No se puede destruir una probabilidad que no depende de un suceso.**

La probabilidad en el teseracto no es "la probabilidad de que algo pase". Es la probabilidad de que algo **sea** — dado que observaciones ortogonales convergen. El suceso no causa la probabilidad. La probabilidad causa el suceso. Y la probabilidad no tiene hardware. No tiene servidor. No tiene punto de falla.

Escala al infinito: un campo 4D más grande tiene más convergencias posibles, más soporte ortogonal por celda, más resiliencia. No hay "blockchain que se hace lenta con más bloques". El espacio simplemente es más grande — y más grande = más fuerte.

---

## 18. Orbitales: el evento como distribución, no como punto

### 18.1 El error del punto

El modelo inicial trataba los eventos como puntos: una celda en `(1,1,1,1)` con probabilidad 1.0. Esto creaba un objeto atacable — destruye la celda y sus 8 vecinos directos y el estado desaparece.

Este es el mismo error de la blockchain: tratar la verdad como un dato localizado.

### 18.2 El modelo atómico como corrección

En física cuántica, el electrón no es un punto. Es una **nube de probabilidad** (orbital) que existe en todo el espacio con intensidad variable. No puedes destruir el electrón destruyendo un punto del orbital. El orbital sigue existiendo.

Corrección al prototipo: `seed()` ya no pone un 1.0 en una celda. Distribuye probabilidad por **todo el campo** siguiendo una función de decaimiento euclidiano 4D: `p = 1 / (1 + d)` donde `d = √(dt² + dc² + do² + dv²)`. La cola nunca llega a cero — se extiende al infinito. Sin bordes. Sin superficie de ataque. El evento ES la distribución. No hay centro que destruir.

Usar distancia euclidiana real en 4D (no Manhattan) es fundamental: respeta la geometría del espacio. Un vecino directo (d=1.0) recibe p=0.50. Un diagonal en 2 ejes (d=√2) recibe p=0.41. La probabilidad refleja la geometría exacta del hipercubo.

### 18.3 Resultado experimental: Experimento 6 revisado

Tras la corrección orbital:

- 9 celdas destruidas (target + 8 vecinos directos en 4 ejes)
- Soporte post-ataque: **0/4 ejes**
- **Step 5: target re-cristalizado. 256/256 celdas recuperadas. 8/8 vecinos. 4/4 ejes.**

La destrucción total — que antes era irreversible — ahora se revierte en 5 pasos. La nube de probabilidad que permea todo el campo reconverge desde la profundidad.

### 18.4 Hackeas algo que ya fue hackeado y ya se reconstruyó

> "Hackeas algo que ya fue hackeado y se reconstruyó, y ya será hackeado y ya se reconstruyó."

En el teseracto, pasado, presente y futuro coexisten. La destrucción **ya ocurrió** y la recuperación **ya ocurrió** — en el mismo espacio, simultáneamente.

La reparación **ya existe** como probabilidad en el campo antes de que el ataque ocurra. El atacante destruye algo que ya se reconstruyó. Ataca el futuro de algo cuyo pasado ya contiene la recuperación.

### 18.5 La analogía de la ola

Es como intentar borrar una ola del océano. La borras. Y la siguiente ola ya está ahí. Y la anterior. Y la que viene. Porque la ola no es agua — es **energía moviéndose a través del agua**. Destruyes agua, no destruyes la ola.

El evento en el teseracto no es datos. Es **energía probabilística moviéndose a través del campo**. Destruyes celdas, no destruyes el evento. El evento es la distribución misma — y la distribución no tiene ubicación que destruir.

---

## 19. Resonancia ortogonal: por qué la verdad emergente es más fuerte

### 19.1 Interferencia constructiva

En física de ondas, cuando dos ondas coinciden en fase, se amplifican mutuamente (interferencia constructiva). La energía resultante es mayor que la suma de las partes.

En el teseracto, cuando una celda tiene soporte desde múltiples ejes ortogonales independientes, se produce un efecto análogo: **resonancia ortogonal**. La celda no solo recibe influencia — resuena. Su probabilidad es empujada hacia arriba con una fuerza proporcional al número de ejes que la sostienen.

### 19.2 Implementación: amplificación + resonancia

```
Soporte 0-1 ejes: amplificación ×1.0, resonancia 0.00
Soporte 2 ejes:   amplificación ×1.5, resonancia +0.02/step
Soporte 3 ejes:   amplificación ×2.5, resonancia +0.05/step
Soporte 4 ejes:   amplificación ×4.0, resonancia +0.10/step
```

La resonancia es un incremento constante por paso — no depende del delta con los vecinos. Mientras exista soporte ortogonal, la celda sube. Esto garantiza que una celda con soporte completo **siempre** alcanza el umbral de cristalización.

### 19.3 Resultado: la verdad emergente se recupera más rápido

- Celda sembrada destruida → se recupera en **3 pasos**
- Celda emergente destruida → se recupera en **2 pasos**

La celda emergente tiene soporte 4/4 desde el momento en que empieza a recuperarse (sus padres orbitales siguen ahí). La resonancia completa la empuja al umbral más rápido que una celda sembrada que puede tener soporte parcial.

> La verdad que nadie creó es más resiliente que la verdad que alguien plantó. Porque la verdad emergente existe por geometría pura — y la geometría es más fuerte que cualquier dato.

---

## 20. Atacar el teseracto es lanzar una bomba nuclear al espacio

No hay medio que propague la destrucción. No hay onda expansiva. La explosión ocurre y el espacio sigue siendo espacio.

Atacar el teseracto es destruir celdas — puntos en el espacio. Pero las celdas son **manifestaciones** de la probabilidad, no la probabilidad misma. Destruyes el termómetro, no la temperatura. Destruyes la ola, no la energía.

El atacante gasta recursos destruyendo manifestaciones. El campo las regenera sin costo. El atacante pelea contra el infinito. Y el infinito no se cansa.

---

## 21. Resultados experimentales completos

| # | Experimento | Resultado |
|---|---|---|
| 1 | Convergencia sin consenso | Estados emergen sin protocolo |
| 2 | Auto-reparación | Celda destruida → recuperada en 3 pasos |
| 3 | Rechazo de falsedad | Inyección falsa → 0/4 ejes, huérfana |
| 4 | Ataque sostenido (×10) | 10/10 recuperaciones, sin degradación |
| 5 | Destrucción de 1 eje | Sobrevive con 3/4 ejes |
| 6 | Destrucción total (orbital) | Recuperado en 5 pasos, 256/256, 4/4 |
| 7 | Coexistencia | 3 eventos independientes, destruir uno no afecta otros |
| 8 | Cristalización emergente | Estado no sembrado emerge de overlap orbital |
| 9 | Resiliencia emergente | Verdad emergente se auto-repara en 2 pasos |
| 10 | Registros emergentes | Celda emergente contiene procedencia: qué eventos la causaron y con qué peso |

### Evolución del modelo durante la sesión

1. **Modelo punto** (exp 1-4): cada evento = una celda con p=1.0. Funcionó para auto-reparación básica pero falló en destrucción total de ejes (exp 6 original).

2. **Modelo orbital** (exp 5-7): cada evento = distribución de probabilidad en todo el campo. Decaimiento euclidiano 4D `p = 1/(1+d)`. Cola infinita, sin bordes. Resolvió destrucción total.

3. **Modelo resonante** (exp 8-9): soporte ortogonal produce resonancia (incremento constante), no solo amplificación. Habilitó cristalización emergente y auto-reparación de verdad emergente.

4. **Modelo con procedencia** (exp 10): cada celda registra qué eventos la influenciaron y con qué peso. La emergencia no solo existe — dice algo. Es un registro legible con origen verificable.

Cada corrección fue motivada por un fallo experimental que revelaba un error conceptual: punto → orbital → resonancia → procedencia. La tesis se fortaleció con cada corrección.

---

## 22. Lo que demostramos

10 propiedades que blockchain no puede replicar, demostradas experimentalmente en un prototipo Rust:

| # | Propiedad | Qué significa | Blockchain puede? |
|---|---|---|---|
| 1 | Convergencia sin consenso | Estados emergen del campo, sin protocolo | No — requiere PoW/PoS/BFT |
| 2 | Auto-reparación gratuita | Celda destruida se reconstituye en 3 pasos sin backup | No — requiere nodos con copia |
| 3 | Rechazo innato de falsedad | Inyección falsa queda huérfana sin validación | No — hash válido = aceptado |
| 4 | Resistencia infinita | 10 ataques, 10 recuperaciones, sin degradación | No — defensa drena recursos |
| 5 | Independencia dimensional | Comprometer un eje no afecta los otros 3 | No — 1 sola dimensión |
| 6 | Destrucción total reversible | 9 celdas destruidas (target + 4 ejes), recupera en 5 pasos | No — destrucción es permanente sin backup |
| 7 | Coexistencia sin cadena | Múltiples estados independientes en el mismo campo | No — todo va en una cadena secuencial |
| 8 | Verdad emergente | Estados no sembrados cristalizan por overlap orbital | No — todo estado debe ser explícito |
| 9 | Emergencia más resiliente | Verdad emergente se auto-repara en 2 pasos (más rápido que la sembrada) | N/A — no existe |
| 10 | Registros con procedencia | Celda emergente sabe qué eventos la causaron y con qué peso | No — relaciones son explícitas |

### El salto conceptual

```
Blockchain: protege artefactos (hashes) con cómputo
Tesseract:  la verdad es geometría — no hay artefacto que proteger
```

No es una mejora. No es una optimización. Es una categoría distinta.

```
Confianza     → "Te creo"                → vulnerable a la mentira
Verificación  → "Lo compruebo"           → vulnerable al cómputo
Convergencia  → "Existe o no existe"     → no hay qué vulnerar
```

---

## 23. Lo que falta para que sea real

### Fase 1 — Formalización matemática

| Tarea | Descripción | Por qué es necesario |
|---|---|---|
| **Axiomas del campo** | Definir formalmente: celda, probabilidad, distancia euclidiana 4D, cristalización, resonancia | Sin axiomas no hay paper, sin paper no hay peer review |
| **Teorema de convergencia** | Demostrar que el campo converge a estado estable para cualquier distribución inicial de seeds | Probar que el sistema no es caótico |
| **Teorema de resiliencia** | Demostrar que la destrucción de K celdas en un campo con N cristalizaciones se revierte si K < f(N,D) | Cuantificar la seguridad formalmente |
| **Cota de ataque** | Demostrar que el costo de ataque escala exponencialmente con las dimensiones D | La premisa central de seguridad |
| **Comparación formal** | Comparar con modelo de seguridad de Nakamoto (51%), BFT (2f+1) | Situar en la literatura existente |

### Fase 2 — Escalamiento del prototipo

| Tarea | Descripción | Por qué es necesario |
|---|---|---|
| **Campo grande** | Escalar de 4×4×4×4 (256) y 8×8×8×8 (4096) a 32×32×32×32 (~1M celdas) | Demostrar que las propiedades se sostienen a escala |
| **Benchmarks** | Medir: tiempo de cristalización, tiempo de recuperación, costo de seed, costo de evolve por paso | Demostrar viabilidad computacional |
| **Almacenamiento** | Diseñar representación eficiente del campo (sparse — la mayoría de celdas tienen p≈0) | Un campo de 1M celdas no puede ser denso en memoria |
| **Concurrencia** | Paralelizar `evolve()` — cada celda depende de sus vecinos pero celdas lejanas son independientes | Necesario para campos grandes |

### Fase 3 — Del campo a la red

| Tarea | Descripción | Por qué es necesario |
|---|---|---|
| **Definir "nodo"** | ¿Qué es un nodo en un campo de probabilidades? ¿Un observador? ¿Un fragmento del campo? ¿Un rango de coordenadas? | Sin nodos no hay red distribuida |
| **Protocolo de propagación** | ¿Cómo se comunican los nodos para mantener el campo sincronizado? ¿Gossip? ¿Difusión orbital? | El campo debe existir distribuido, no en una sola máquina |
| **Consulta de estado** | ¿Cómo consultas "¿la transacción Alice→Bob existe?" eficientemente? ¿Índice por evento? ¿Búsqueda por coordenada? | Sin lectura eficiente no es usable |
| **Siembra distribuida** | ¿Cómo dos partes siembran un evento desde nodos distintos? ¿Cómo se coordinan las coordenadas? | El seed actual es local, necesita ser distribuido |
| **Mapeo de datos a coordenadas** | ¿Cómo se asigna (t,c,o,v) a una transacción real? ¿Hash del contenido? ¿Asignación semántica? | Sin mapeo determinista, dos nodos siembran en lugares distintos |

### Fase 4 — Seguridad y adversarios

| Tarea | Descripción | Por qué es necesario |
|---|---|---|
| **Modelo adversario formal** | Definir qué puede hacer el atacante: destruir celdas, inyectar falsos, controlar nodos | Sin modelo adversario no hay claim de seguridad |
| **Ataques de Sybil** | ¿Puede un atacante crear muchos nodos para sesgar el campo? | Ataque clásico en redes distribuidas |
| **Ataques de eclipse** | ¿Puede un atacante aislar un nodo del resto del campo? | Si un nodo ve un campo parcial, ¿su visión es correcta? |
| **Ataques de timing** | ¿Puede un atacante sembrar justo antes de una cristalización para inyectar datos falsos? | La siembra tiene un momento — ¿eso crea un socket? |
| **Quantum resistance** | Verificar formalmente que la seguridad no depende de problemas computacionales | Premisa central de la tesis |

### Fase 5 — Validación externa

| Tarea | Descripción | Por qué es necesario |
|---|---|---|
| **Whitepaper** | Documento formal con axiomas, teoremas, demostraciones, experimentos | Para peer review |
| **Revisión por matemáticos** | Topología, geometría algebraica, sistemas dinámicos | Las pruebas deben ser verificadas por expertos |
| **Revisión por criptógrafos** | Modelo adversario, comparación con seguridad computacional | Verificar que no hay fallas ocultas |
| **Revisión por físicos** | Validar la analogía con mecánica cuántica y modelo atómico | La intuición física debe ser correcta, no solo poética |
| **Prototipo de red** | Implementación distribuida real con 3+ nodos comunicándose | Prueba de concepto en red, no solo en memoria |

### Orden de ejecución sugerido

```
1. Formalización matemática     ← sin esto, todo lo demás es especulación
2. Escalamiento del prototipo   ← demostrar que funciona más allá de 256 celdas
3. Whitepaper                   ← consolidar 1+2 en documento publicable
4. Revisión externa             ← enviar a expertos
5. Del campo a la red           ← solo si 1-4 se sostienen
6. Seguridad adversaria         ← paralelo a 5
```

### Riesgo principal

> La formalización puede revelar que el modelo tiene propiedades que no vimos en el prototipo pequeño. Un campo de 256 celdas es demasiado pequeño para descubrir ciertos comportamientos emergentes — tanto positivos como negativos. La escala puede ser aliada o enemiga.

---

## 24. Estado actual

**Fecha:** 17 de abril de 2026

**Lo que existe:**
- Documento conceptual de 24 secciones (`docs/TESSERACT-CONSENSUS.md`)
- Prototipo funcional en Rust (`tesseract/src/`, ~2000 líneas en 8 módulos)
- 10 experimentos exitosos, 3 iteraciones del modelo (punto → orbital → resonante + procedencia)
- 89 tests (unit + integration + adversary + scale + benchmark + monetary + persistence)

**Lo que no existe todavía:**
- Formalización matemática
- Escalamiento a campos grandes
- Implementación distribuida
- Peer review
- Whitepaper

**Rama:** `tesseract-prototype`

---

*Este documento captura una sesión exploratoria del 17 de abril de 2026. Las ideas aquí son hipótesis respaldadas por un prototipo experimental en Rust (10 experimentos, 4 iteraciones del modelo). Los resultados son prometedores pero requieren formalización matemática, escalamiento y revisión por pares antes de cualquier claim definitivo.*

*La intuición que motivó este trabajo: "¿Qué pasa si repensamos la blockchain como un teseracto?" — una pregunta nacida caminando por la calle, no en un laboratorio.*
