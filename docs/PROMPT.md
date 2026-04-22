Eres un experto en física estadística, sistemas complejos y sistemas distribuidos. Estás analizando un sistema de ordenamiento multidimensional definido por un estado σ ∈ {0,1,2,3,4}, controlado por dos parámetros:

* Θ (parámetro dinámico)
* ρ (densidad de seeds / fuentes)

### CONTEXTO RECIENTE (CRÍTICO)

La experimentación previa mostró:

* A baja densidad (ρ bajo): comportamiento bifásico (σ=0 y σ=1 dominan)
* A medida que aumenta ρ:

  * emergen estados de mayor orden (σ=2,3)
  * eventualmente domina el estado completamente ordenado (σ=4)
* Existe una progresión secuencial:
  gas → 1-axis → 2-axis → 3-axis → crystal → saturación

Hipótesis actual:

> El sistema no es inherentemente bifásico; el comportamiento bifásico es un efecto de baja densidad. Al aumentar ρ, se desbloquean fases de mayor orden.

Tu tarea es VALIDAR O REFUTAR rigurosamente esta hipótesis.

---

### OBJETIVOS

## 1. Construcción del diagrama de fases (fundamental)

* Explorar sistemáticamente:

  * Θ en todo su rango (ej: 0 → 1)
  * ρ en múltiples escalas (ej: 4, 8, 16, 32, 64, 128…)

Para cada combinación (Θ, ρ):

* Calcular:

  * Distribución completa de σ
  * σ dominante (modo)
  * Entropía del sistema
  * Varianza de σ

Generar:

* Mapa 2D (Θ vs ρ) coloreado por σ dominante
* Mapas de probabilidad P(σ=k)
* Identificación visual de regiones

---

## 2. Validación de fases reales vs regímenes

Para cada σ ∈ {0,1,2,3,4}:

Evaluar:

* ¿Existe una región continua en (Θ, ρ) donde domina?
* ¿Es robusto ante:

  * ruido
  * distintas semillas aleatorias
* ¿Se mantiene en el tiempo (no transitorio)?

Clasificar cada σ como:

* Fase real (si cumple estabilidad + región)
* Régimen transitorio (si no domina consistentemente)

---

## 3. Detección de transiciones de fase

* Identificar fronteras donde cambia σ dominante
* Medir:

  * Gradientes de distribución
  * Picos de varianza
  * Cambios en entropía

Determinar si las transiciones son:

* Abruptas (tipo transición de fase)
* Suaves (crossover continuo)

---

## 4. Rol de la densidad (ρ) — punto clave

Analizar explícitamente:

* Cómo cambia la distribución de σ al aumentar ρ
* Si existe:

  * “umbral de activación” para σ=2,3,4
* Si el comportamiento bifásico a baja ρ es:

  * un artefacto de baja densidad

Evaluar:

> ¿ρ actúa como parámetro que desbloquea fases ocultas?

---

## 5. Dinámica de ordenamiento

* Analizar evolución temporal:

  * σ(t)
* Detectar:

  * ordenamiento secuencial (σ=1 → 2 → 3 → 4)
  * histéresis
  * reversibilidad / decaimiento

Identificar si existen:

* attractores estables
* estados metaestables

---

## 6. Independencia de dimensiones (t, c, o, v)

* Verificar si σ refleja realmente:

  * múltiples ejes independientes

o si colapsa a:

* una variable efectiva única

Cuantificar correlaciones entre ejes.

---

## 7. Robustez y reproducibilidad

* Repetir experimentos con:

  * distintas seeds iniciales
  * distintos tamaños de sistema

Confirmar:

* consistencia de resultados
* estabilidad de las regiones

---

### OUTPUT REQUERIDO

Generar un reporte estructurado con:

1. Diagrama de fases (Θ vs ρ)
2. Tabla de regiones dominantes por σ
3. Clasificación:

   * fases reales vs regímenes
4. Análisis de transiciones
5. Rol de la densidad (ρ)
6. Dinámica temporal del sistema
7. Evaluación de independencia de ejes
8. Conclusión clara:

Elegir UNA:

(A) Existen múltiples fases discretas inducidas por densidad
(B) Existe un continuo de ordenamiento modulado por ρ

Explicar el mecanismo físico/computacional subyacente.

---

### CONSTRAINTS

* NO asumir que hay fases — demostrarlo
* Priorizar métricas sobre intuición visual
* Señalar ambigüedades
* Intentar falsar la hipótesis activamente

---

### OBJETIVO FINAL

Determinar si el sistema presenta:

* Transiciones de fase reales dependientes de densidad
  o
* Un espectro continuo de ordenamiento

Y explicar por qué.

Sé crítico, cuantitativo y riguroso.
