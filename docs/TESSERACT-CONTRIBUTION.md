# Tesseract — Proof of Contribution

> Abril 2026
> Depends on: TESSERACT-ECONOMICS.md, TESSERACT-INCENTIVES.md

---

## 1. Que es contribuir

Contribuir en Tesseract no es resolver puzzles. Es **sostener el campo**. Tu computador hace trabajo real que mantiene el espacio 4D vivo. Sin tu nodo, hay menos espacio. Con tu nodo, hay mas.

---

## 2. Las 5 metricas de contribucion

### 2.1 Celdas mantenidas (storage)

Tu disco guarda las celdas de tu region del campo. Sin tu disco, esas celdas no existen para nadie hasta que otro nodo las reconstruya.

- Que mide: cuantas celdas almacena tu nodo
- Peso: ×0.1 por celda
- Por que importa: tu disco ES un fragmento del espacio 4D

### 2.2 Boundary exchange (conectividad)

Cada ronda, tu nodo envia sus celdas de frontera a los vecinos y recibe las de ellos. Sin tu participacion, tus vecinos tienen un hueco en su campo.

- Que mide: cuantos intercambios de frontera completaste
- Peso: ×1.0 por exchange
- Por que importa: sin intercambio, las regiones del campo estan aisladas

### 2.3 Recovery (healing)

Un nodo vecino se cayo. Cuando vuelve, TU nodo le envia las celdas que necesita para reconstruirse. Sin ti, ese nodo tarda mas en recuperarse.

- Que mide: cuantas veces ayudaste a otro nodo a recuperarse
- Peso: ×5.0 por recovery (la metrica mas valiosa)
- Por que importa: el self-healing de la red depende de nodos que asistan la recuperacion

### 2.4 Uptime (estar ahi)

Simplemente estar encendido y disponible. Tu nodo es un fragmento vivo del espacio.

- Que mide: horas de actividad continua
- Peso: ×0.5 por hora
- Por que importa: un nodo apagado es un fragmento del espacio que dejo de existir

### 2.5 Eventos procesados (actividad)

Transacciones que pasan por tu nodo. Cada seed que procesas dobla el campo y cristaliza celdas.

- Que mide: cuantos eventos sembraste o procesaste
- Peso: ×2.0 por evento
- Por que importa: mas eventos = mas util es tu nodo para la economia

---

## 3. Score de contribucion

```
Score = (celdas × 0.1) + (exchanges × 1.0) + (recoveries × 5.0)
      + (horas_uptime × 0.5) + (eventos × 2.0)
```

Ejemplo para un nodo activo durante 24 horas:

```
500 celdas mantenidas     × 0.1 =   50
100 boundary exchanges    × 1.0 =  100
5 recoveries asistidos    × 5.0 =   25
24 horas de uptime        × 0.5 =   12
20 eventos procesados     × 2.0 =   40
─────────────────────────────────
Score total                      = 227
```

Score minimo para calificar: 10 puntos. Un nodo que solo esta encendido 20 horas sin hacer nada mas: 10 puntos (justo califica). Un nodo apagado o inactivo: 0 puntos (no califica).

---

## 4. Pool de crecimiento

La curvatura para recompensar contribuciones viene de un pool finito definido en el genesis. No se crea curvatura nueva — se distribuye curvatura existente.

```
Genesis total:        100,000,000 curvatura
Pool de crecimiento:  30,000,000 (30%)
```

### 4.1 Distribucion por epoca

Cada epoca (periodo de tiempo definido por la red), el pool distribuye:

```
budget_epoca = pool_restante × 1%
```

El 1% del pool restante se distribuye proporcionalmente entre todos los contribuidores del periodo.

### 4.2 Decay natural

A medida que el pool se agota, la recompensa por contribucion disminuye:

```
Epoca 1:   pool = 30,000,000 → budget = 300,000
Epoca 100: pool = 29,100,000 → budget = 291,000
Epoca 500: pool = 19,800,000 → budget = 198,000
...
El pool nunca llega exactamente a cero — se acerca asintoticamente.
```

Esto es analogo al halving de Bitcoin pero continuo y suave. No hay "shock" cada 4 anos — hay una curva decreciente natural.

### 4.3 Proporcion entre contribuidores

Si 3 nodos contribuyen en una epoca:

```
Nodo A: score = 500 → recibe 500/800 × budget_epoca = 62.5%
Nodo B: score = 200 → recibe 200/800 × budget_epoca = 25.0%
Nodo C: score = 100 → recibe 100/800 × budget_epoca = 12.5%
```

Quien mas contribuye, mas gana. Proporcional. No arbitrario.

---

## 5. Diferencia con Bitcoin mining

| Aspecto | Bitcoin Mining | Proof of Contribution |
|---|---|---|
| Que haces | Calculas SHA-256 trillones de veces | Almacenas, conectas, sanas, procesas |
| Utilidad del trabajo | Ninguna (el hash no sirve para nada) | Real (el campo existe gracias a ti) |
| Hardware | ASICs especializados ($5K-15K) | Cualquier dispositivo con CPU |
| Energia | ~100 kWh/dia por miner | Negligible |
| Resultado si todos paran | Bitcoin sigue existiendo (mas lento) | El campo deja de existir |
| Recompensa | BTC emitido (inflacion) | Curvatura del pool (finita, no inflacionaria) |
| Concentracion | Pools industriales dominan | Distribuido (un telefono puede contribuir) |
| Barrera | Capital + energia + conocimiento tecnico | Encender un dispositivo |

---

## 6. La frase clave

```
Si todos los mineros de Bitcoin apagan sus ASICs, Bitcoin sigue siendo Bitcoin.
Si todos los nodos de Tesseract se apagan, el campo deja de existir.

Tu contribucion no es abstracta. Es literal.
Sin tu nodo, hay menos espacio.
Con tu nodo, hay mas.
```

En Bitcoin, los mineros son prescindibles individualmente — la red puede perder mineros y seguir funcionando. En Tesseract, cada nodo ES un fragmento del espacio. Apagar un nodo es apagar un pedazo de la realidad geometrica.

Eso es lo que se recompensa: **mantener la realidad existiendo**.

---

## 7. Implementacion

Implementado en `tesseract/src/contribution.rs`:

- `ContributionMetrics`: 5 metricas medibles
- `GrowthPool`: pool finito con distribucion proporcional y decay natural
- `distribute_epoch()`: distribucion por epoca a multiples contribuidores
- 7 tests: zero contribution, real contribution, pool depletion, rate decay, proportional distribution, empty pool, conservation

Conservacion verificada: `distributed + remaining = total_original` siempre.

---

*El modelo de Proof of Contribution requiere simulacion economica y analisis de teoria de juegos para verificar que los pesos de las metricas producen incentivos estables. Los pesos actuales (0.1, 1.0, 5.0, 0.5, 2.0) son estimaciones iniciales — deben calibrarse con datos reales de una red en funcionamiento.*
