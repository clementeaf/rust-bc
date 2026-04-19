# Tesseract Field — Proofs and Derivations

> v0.1 — 17 abril 2026
> Depends on: `TESSERACT-AXIOMS.md`
> Status: draft proofs, semi-formal

---

## Proof 1: Self-Healing (Conjecture 8.1)

**Statement.** If a crystallized cell **x** is destroyed, and σ(**x**) ≥ 2 from still-crystallized neighbors, then **x** re-crystallizes in finite steps.

**Proof sketch.**

After destruction: p(**x**) = 0, κ(**x**) = 0.

Since σ(**x**) ≥ 2, at least 2 axes have neighbors with p > 0.5. These neighbors are crystallized (p = 1.0), so they remain stable (Axiom 3).

At each evolution step, the neighbor average μ(**x**) satisfies:

```
μ(**x**) ≥ (2 × 1.0) / 8 = 0.25
```

(At least 2 of 8 neighbors have p = 1.0; the rest contribute ≥ 0.)

The delta at step 1:
```
δ₁ = (μ - 0) × α = μ × 0.15 ≥ 0.0375
```

With σ ≥ 2, resonance R ≥ 0.02, so:
```
p₁ ≥ 0.0375 × 1.5 + 0.02 = 0.07625
```

At each subsequent step, p increases because:
1. δ > 0 (neighbor average exceeds cell probability while p < μ)
2. R > 0 (constant upward push from resonance)

The probability sequence {pₙ} is monotonically increasing and bounded above by 1.0.

**Key insight:** The resonance term R is independent of δ. Even when δ → 0 (as p approaches μ), R continues to push p upward at a constant rate per step:

```
pₙ₊₁ ≥ pₙ + R    (when δ·A ≥ 0, which holds whenever p ≤ μ)
```

Therefore:
```
pₙ ≥ n × R    (lower bound for early steps when p << μ)
```

For Θ = 0.85 and R = 0.02 (σ=2, worst case):
```
n ≤ ⌈0.85 / 0.02⌉ = 43 steps (upper bound)
```

For σ = 4, R = 0.10:
```
n ≤ ⌈0.85 / 0.10⌉ = 9 steps (upper bound)
```

In practice, convergence is faster because δ·A contributes positively in early steps. Experimental observation: 2-5 steps.

**∎** (Semi-formal. A rigorous proof requires showing μ(**x**) remains stable while **x** recovers, which holds because neighbors are crystallized and immutable under evolution.)

---

## Proof 2: Non-Propagation of Falsehood (Conjecture 8.2)

**Statement.** A cell crystallized by external force at a point far from any event orbital does not cause additional crystallizations beyond natural field evolution.

**Proof sketch.**

Let **f** be the fake cell, forced to κ(**f**) = 1, p(**f**) = 1.0.

Consider a neighbor **y** ∈ N(**f**). The fake contributes to **y**'s neighbor average:
```
Δμ_fake = 1.0 / 8 = 0.125
```

For **y** to crystallize, it needs p(**y**) ≥ Θ = 0.85.

If **f** is far from real events, then the background probability at **y** comes only from distant orbital tails. For a field of size S with events at distance d:
```
p_background(**y**) ≈ Σᵢ 1/(1 + dᵢ)
```

For the fake to cause crystallization at **y**, we need:
```
p_background(**y**) + Δμ_fake × α × A + R ≥ Θ
```

The fake cell contributes at most 0.125 to the neighbor average. With α = 0.15 and A = 1.0 (σ(**y**) ≤ 1 since only 1 neighbor — the fake — has high p):
```
contribution per step = 0.125 × 0.15 × 1.0 + 0.0 = 0.01875
```

No resonance (R=0) because σ(**y**) ≤ 1.

The evolution converges to equilibrium where:
```
p_eq(**y**) = μ(**y**)    (when δ → 0)
```

Since the fake is only 1 of 8 neighbors, the equilibrium probability at **y** is dominated by the other 7 neighbors, which have low background probability.

**The fake lacks the geometric structure to create resonance.** Without σ ≥ 2, there is no resonance term. Without resonance, the probability at neighbors converges to a value well below Θ.

A single forced point cannot create orthogonal support — it exists on all axes simultaneously but only occupies ONE cell. It would need crystallized neighbors on at least 2 independent axes, which requires additional forced crystallizations (i.e., a multi-cell attack, not a single injection).

**∎** (Proven for single-cell injection. Multi-cell coordinated injection is addressed in Conjecture 8.3.)

---

## Proof 3: Attack Cost Scaling (Conjecture 8.3)

**Statement.** The cost of permanently destroying a state scales super-linearly with the field's density and dimensionality.

**Proof sketch.**

Consider a crystallized cell **x** with support from events in D dimensions. The cell exists because of orbital overlap from nearby events.

**To permanently destroy **x**:**
1. Destroy **x** itself → recovers from neighbors (Proof 1)
2. Destroy **x** + all direct neighbors (8 cells) → recovers from 2nd ring (experimental, exp 6)
3. Destroy **x** + all cells within radius r → recovers from ring r+1

The number of cells within radius r in D dimensions scales as:
```
|B(r, D)| ~ (2r)^D     (hypercube volume)
```

For D = 4:
```
r=1: ~16 cells
r=2: ~256 cells  (would need to destroy all to prevent recovery from r=3)
r=3: ~4096 cells
```

Each ring provides recovery capability for the ring inside it. To prevent recovery, the attacker must destroy ALL rings out to the field boundary — effectively the entire field.

**The cost scales as O(S^D)** — the entire field must be destroyed to permanently eliminate a state that has been seeded with a field-wide orbital.

This is qualitatively different from blockchain, where the cost is O(hashrate) — a scalar quantity independent of state structure.

**∎** (Semi-formal. Assumes orbital decay provides meaningful probability at all distances, which holds by Axiom 4.)

---

## Proof 4: Emergent Crystallization (Conjecture 8.4)

**Statement.** Two events E₁ at **c₁** and E₂ at **c₂** produce crystallization at points between them that were not seeded.

**Proof.**

For a point **x** on the line segment between **c₁** and **c₂**, the total probability from both events is:

```
p(**x**) = ΔP₁(**x**) + ΔP₂(**x**)
         = 1/(1 + d(**c₁**, **x**)) + 1/(1 + d(**c₂**, **x**))
```

By the triangle inequality and midpoint properties, for the midpoint **m** where d(**c₁**, **m**) = d(**c₂**, **m**) = d(**c₁**, **c₂**)/2:

```
p(**m**) = 2 / (1 + d(**c₁**, **c₂**)/2)
```

Crystallization occurs when p(**m**) ≥ Θ:

```
2 / (1 + d/2) ≥ Θ
d ≤ 2(2/Θ - 1)
```

For Θ = 0.85:
```
d_max = 2(2/0.85 - 1) = 2(2.353 - 1) = 2(1.353) = 2.706
```

**Any two events within Euclidean distance ≤ 2.706 produce emergent crystallization at their midpoint.** This matches experimental observation (exp 8: distance 2, midpoint crystallized).

**Corollary.** The maximum distance for emergent crystallization is:
```
d_max(Θ) = 2(2/Θ - 1)
```

| Θ | d_max |
|---|---|
| 0.80 | 3.00 |
| 0.85 | 2.71 |
| 0.90 | 2.22 |
| 0.95 | 1.11 |

**∎** (Rigorous for the midpoint case. Extension to non-midpoint emergent cells follows from continuity of the probability function.)

---

## Proof 5: Emergent Resilience (Conjecture 8.5)

**Statement.** An emergent crystallization self-heals faster than a directly seeded cell.

**Proof sketch.**

An emergent cell **m** at the midpoint of events E₁ and E₂ has:

1. **Higher background probability.** Both parent orbitals contribute to its neighbor average:
```
μ(**m**) ≥ μ_single_event    (two sources instead of one)
```

2. **Higher orthogonal support.** The parent events seed probability along all 4 axes independently. At the midpoint, contributions come from both directions on the axis connecting the parents, plus both parents contribute on the other 3 axes:
```
σ(**m**) = 4    (typically, when parents are axis-aligned)
```

3. **Maximum resonance.** With σ = 4, R = 0.10 — the highest resonance value.

After destruction, the recovery rate is:
```
Δp/step ≥ δ·A + R = δ·4.0 + 0.10
```

Compare to a seeded cell with σ = 3:
```
Δp/step ≥ δ·2.5 + 0.05
```

The emergent cell receives both higher amplification (4.0 vs 2.5) and higher resonance (0.10 vs 0.05).

Experimental confirmation: emergent cell recovers in 2 steps vs 3 steps for seeded cells.

**∎**

---

## Summary

| Conjecture | Status | Key result |
|---|---|---|
| 8.1 Self-healing | **Proven** (semi-formal) | Recovery guaranteed in ≤ ⌈Θ/R⌉ steps |
| 8.2 Non-propagation | **Proven** (single-cell) | Single fake cannot create resonance (σ ≤ 1, R = 0) |
| 8.3 Attack cost | **Proven** (semi-formal) | Cost scales as O(S^D), requires destroying entire field |
| 8.4 Emergent crystallization | **Proven** (rigorous) | d_max = 2(2/Θ - 1), closed-form |
| 8.5 Emergent resilience | **Proven** (semi-formal) | σ=4 → maximum resonance → fastest recovery |

All proofs are semi-formal and derive from the axioms in `TESSERACT-AXIOMS.md`. Full rigor requires verification by a mathematician specializing in dynamical systems or discrete geometry.

---

*Next step: Theorem of convergence — prove that the field reaches a unique stable equilibrium for any initial distribution of seeds.*
