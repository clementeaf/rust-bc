# Tesseract — Modelo de Incentivos

> Abril 2026
> Depends on: TESSERACT-WHITEPAPER.md, TESSERACT-SPACETIME.md

---

## 1. El problema

Blockchain resolvió el incentivo con minería: gastas energía resolviendo puzzles, recibes tokens. El modelo funciona porque separa dos roles:

- **Mineros**: participan en la infraestructura → ganan tokens
- **Usuarios**: usan la infraestructura → pagan fees

Esta separación crea una industria extractiva: los mineros cobran por incluir transacciones. Los usuarios pagan por existir en la red. Hay un intermediario computacional entre las partes.

El teseracto no tiene minería. No tiene puzzles. No tiene validadores. ¿Por qué participar?

---

## 2. La respuesta: participar ES usar

En blockchain, participar y usar son actividades separadas. En el teseracto son **la misma cosa**.

| | Blockchain | Tesseract |
|---|---|---|
| Participar | Minar / validar (rol separado) | Transaccionar (mismo rol) |
| Incentivo | Tokens como recompensa | Peso geométrico como capacidad |
| Costo de participar | Alto (energía, hardware) | Casi cero (aritmética básica) |
| Intermediario | Minero cobra fee | No hay intermediario |
| Para qué corres un nodo | Para ganar | Para existir |

### 2.1 No corres un nodo para ganar — corres un nodo para existir

Tu nodo ES tu presencia en el espacio 4D. Sin nodo, tu región está vacía. Sin cristalizaciones. Sin peso geométrico. No puedes transaccionar porque no existes en el espacio.

Correr un nodo no es "minar" — es tener dirección en el universo económico. Como tener un cuerpo: no lo tienes para ganar algo, lo tienes porque sin él no existes.

### 2.2 No hay fee — hay curvatura

En blockchain, cada transacción paga un fee a un minero. En el teseracto, el "costo" de una transacción es **curvatura**: tu región pierde capacidad geométrica, la región del receptor la gana.

No hay tercero que cobra. El espacio se redistribuye entre las partes. El fee es cero porque no hay intermediario extractivo.

---

## 3. El ciclo de incentivos

```
Transaccionas → ganas peso geométrico → más peso = más capacidad → más transacciones
```

### 3.1 Peso geométrico como incentivo natural

Tu peso geométrico (crystallized_cells × binding_energy × external_influences) crece con la actividad real:

- Cada transacción con otro participante cristaliza celdas en tu región
- Cada cristalización aumenta tu binding energy (estás más anclado en el espacio)
- Cada interacción con un participante distinto suma una influencia externa

Un participante activo con muchas interacciones reales tiene una región densa, cristalizada, con alto peso. Un participante inactivo tiene una región vacía.

### 3.2 El peso no se mina — se gana viviendo

No puedes "minar" peso geométrico. No hay puzzle que resolver. No hay hardware que comprar. Tu peso crece exclusivamente por:

1. **Interactuar con otros** — transacciones reales con participantes reales
2. **Ser interactuado** — otros eligen transaccionar contigo
3. **Permanecer** — tu historia de cristalizaciones se acumula

Es como reputación, pero geométrica: no es un número que alguien asigna, es la densidad de cristalización que la red creó en tu región.

### 3.3 Anti-farming

¿Puede alguien crear transacciones falsas consigo mismo para inflar su peso? No:

- Transacciones entre dos identidades de la misma persona usan curvatura de la misma región — no ganan peso neto
- El peso depende de **external_influences** — necesitas interacciones con OTROS participantes reales
- La resistencia a Sybil (identity.rs) garantiza que identidades falsas sin interacciones reales tienen peso = 0

---

## 4. Costo de participar

### 4.1 Bitcoin

```
Hardware:    ASIC ~$5,000-15,000
Energía:     ~100 kWh/día
Operación:   Enfriamiento, mantenimiento, hosting
Barrera:     MUY ALTA — solo viable para operaciones industriales
```

### 4.2 Ethereum (PoS)

```
Staking:     32 ETH (~$100,000+ al precio actual)
Hardware:    PC estándar + almacenamiento
Energía:     ~1 kWh/día
Barrera:     ALTA — requiere capital significativo
```

### 4.3 Tesseract

```
Hardware:    Cualquier dispositivo con CPU (teléfono, laptop, Raspberry Pi)
Energía:     Negligible (aritmética de punto flotante, sin crypto)
Capital:     Cero
Almacenamiento: Solo tu proyección (~2-5% del campo)
Barrera:     CASI CERO — si tienes internet, puedes participar
```

La barrera de entrada es **dos órdenes de magnitud menor** que Bitcoin y un orden menor que Ethereum. Esto permite participación masiva — no una élite de mineros industriales.

---

## 5. Incentivo temprano (bootstrap)

### El problema del huevo y la gallina

¿Por qué ser el primer nodo si no hay nadie con quién transaccionar?

### La respuesta

Los primeros participantes obtienen:

1. **Las regiones con más historia.** Cuando la red crece, sus regiones son las más densamente cristalizadas. Tienen la mayor binding energy. Son las más difíciles de desplazar.

2. **Más peso geométrico.** Cada nueva interacción se suma a las anteriores. Los primeros acumulan más capas de cristalización que los tardíos.

3. **Curvatura fundacional.** La genesis allocation define la capacidad geométrica inicial. Los primeros participantes reciben la curvatura que luego circula en toda la economía.

Es análogo a Bitcoin: los primeros mineros obtuvieron bloques con recompensa de 50 BTC cuando nadie más competía. Los primeros participantes del teseracto obtienen regiones densas en un espacio aún vacío.

### Pero sin la asimetría de Bitcoin

En Bitcoin, los primeros mineros hoy tienen millones de dólares en BTC que minaron con una laptop. Eso es asimétrico — la recompensa fue desproporcionada al esfuerzo.

En el teseracto, el peso geométrico no se vende. No es un token con precio de mercado. Es capacidad de operar en el espacio. Los primeros participantes tienen más capacidad, pero esa capacidad solo vale algo si la usan — transaccionando con otros. No hay "hodl" de peso geométrico. Se usa o no sirve.

---

## 6. Comparación de modelos de incentivo

| Aspecto | Bitcoin (PoW) | Ethereum (PoS) | Tesseract |
|---|---|---|---|
| **Qué se recompensa** | Cómputo (hashes) | Capital bloqueado (stake) | Actividad real (transacciones) |
| **Quién gana** | Quien gasta más energía | Quien tiene más capital | Quien más interactúa |
| **Barrera de entrada** | Hardware industrial | 32 ETH ($100K+) | Un teléfono |
| **Costo por tx** | Fee al minero (~$1-50) | Fee al validador (~$0.1-5) | Curvatura (sin fee a tercero) |
| **Intermediario** | Sí (minero) | Sí (validador) | No |
| **Puede "hodlear" recompensa** | Sí (tokens) | Sí (tokens) | No (peso = uso, no activo) |
| **Farming / gaming** | Pool mining | MEV, front-running | Resistente (requiere interacciones reales) |
| **Huella energética** | ~150 TWh/año | ~0.01 TWh/año | Negligible |
| **Concentración** | Pools dominan | Ballenas dominan | Distribuido (peso = red de interacciones) |

---

## 7. La idea más profunda

En Bitcoin, el incentivo es **extrínseco**: minas para ganar algo externo a la actividad (tokens). Es trabajo → recompensa.

En el teseracto, el incentivo es **intrínseco**: participas porque participar ES el beneficio. No hay recompensa separada de la actividad. Transaccionar te da peso, y peso es capacidad de transaccionar más.

Es la diferencia entre:
- **Trabajo asalariado**: haces algo para recibir algo distinto (dinero)
- **Vivir**: vives porque vivir te permite seguir viviendo

El teseracto no paga por participar. Participar ES la recompensa — porque participar es existir en el espacio económico.

---

## 8. Resumen

```
Bitcoin:     Gasta energía → gana tokens → paga fees para usar
Ethereum:    Bloquea capital → gana rewards → paga fees para usar
Tesseract:   Usa → gana peso → usa más (sin fees, sin intermediario)
```

No hay mineros. No hay validadores. No hay fees. No hay intermediarios extractivos. No hay hardware especializado. No hay barrera de capital.

Hay un espacio. Existes en él. Transaccionas. Tu región se cristaliza. Eso es tu peso. Tu peso es tu capacidad. Tu capacidad te permite transaccionar.

El incentivo no es una recompensa. El incentivo es la existencia.

---

*Este modelo de incentivos es teórico. Requiere validación económica, simulación de teoría de juegos, y análisis de equilibrios de Nash para confirmar que los incentivos son estables bajo competencia y comportamiento adversarial.*
