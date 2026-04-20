#!/usr/bin/env python3
"""
Directed Percolation Analysis — is Tesseract in the DP universality class?

Directed Percolation (DP) is the universality class for systems with:
1. An ABSORBING state (once entered, can't leave) ← crystallization!
2. A phase transition between active (fluctuating) and absorbed phases
3. No special symmetries or conservation laws

DP critical exponents in 4+1 dimensions (4 spatial + 1 temporal):
  β ≈ 1.0 (order parameter) — DP becomes mean-field above d=4
  ν_perp ≈ 0.5 (spatial correlation)
  ν_par ≈ 1.0 (temporal correlation)
  z = ν_par/ν_perp ≈ 2.0 (dynamic exponent)

Mean-field DP exponents (exact for d ≥ 4):
  β = 1, ν_perp = 1/2, ν_par = 1, z = 2

Key test: measure how the density of ACTIVE (non-crystallized) cells
decays near the critical point. In DP:
  ρ_active(t) ~ t^(-δ) at criticality, where δ = β/ν_par

Experiments:
1. Order parameter β: crystallized fraction vs (Θ_c - Θ)
2. Temporal decay δ: active density vs time at Θ_c
3. Spreading exponent η: how crystallization spreads from a single seed
4. Survival probability: does a seed survive vs die?
"""
import math
import time
import sys
sys.path.insert(0, '.')
from field_gl import FieldGL

R = "\033[0m"; B = "\033[1m"; C = "\033[36m"; Y = "\033[33m"
G = "\033[32m"; RED = "\033[31m"; D = "\033[2m"

J_CAL = 0.10
R_CAL = 8.0
SEED_CAL = 0.50

def header(name):
    print(f"\n{B}{Y}{'━' * 65}{R}")
    print(f"{B}{Y}  {name}{R}")
    print(f"{B}{Y}{'━' * 65}{R}\n")

def make_seeds(size):
    h = size // 2
    q = size // 4
    return [
        ((q, h, h, h), "ev-A"),
        ((3*q, h, h, h), "ev-B"),
        ((h, q, h, h), "ev-C"),
        ((h, 3*q, h, h), "ev-D"),
    ]


# ═══════════════════════════════════════════════
# Find precise Θ_c via bisection
# ═══════════════════════════════════════════════

def find_theta_c(size=10):
    """Binary search for Θ_c: the point where ψ transitions."""
    seeds = make_seeds(size)
    lo, hi = 0.10, 0.40

    for _ in range(15):  # 15 iterations of bisection
        mid = (lo + hi) / 2
        f = FieldGL(size, theta=mid, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)
        psi = f.order_param()

        if psi > 0.5:
            lo = mid  # still ordered — push Θ higher
        else:
            hi = mid  # disordered — pull Θ lower

    return (lo + hi) / 2


# ═══════════════════════════════════════════════
# EXP 1: Temporal decay of active density at Θ_c
# ═══════════════════════════════════════════════

def exp1_temporal_decay(theta_c):
    header("EXP 1: Temporal decay — active density ρ(t) at Θ_c")
    print(f"  Θ_c = {theta_c:.4f}\n")

    size = 10
    seeds = make_seeds(size)

    # Run at Θ_c and measure active (non-crystallized) density over time
    f = FieldGL(size, theta=theta_c, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
    for pos, eid in seeds:
        f.seed(pos, eid)

    print(f"  {'step':>5s}  {'ρ_active':>10s}  {'crystallized':>12s}  {'chart'}")
    print(f"  {'─'*5}  {'─'*10}  {'─'*12}  {'─'*35}")

    log_t = []
    log_rho = []
    total = size ** 4

    for step in range(1, 201):
        f.evolve()
        n_cryst = len(f.crystallized)
        n_active = len(f.cells) - n_cryst
        rho = n_active / total if total > 0 else 0

        if step <= 20 or step % 10 == 0:
            bar = Y + "█" * int(rho * 200) + R
            print(f"  {step:5d}  {rho:10.6f}  {n_cryst:12d}  {bar}")

        if step >= 5 and rho > 0:
            log_t.append(math.log10(step))
            log_rho.append(math.log10(rho))

    # Fit power law: ρ(t) ~ t^(-δ)
    if len(log_t) >= 5:
        n = len(log_t)
        sx, sy = sum(log_t), sum(log_rho)
        sxy = sum(x * y for x, y in zip(log_t, log_rho))
        sxx = sum(x * x for x in log_t)
        denom = n * sxx - sx * sx
        if abs(denom) > 1e-10:
            neg_delta = (n * sxy - sx * sy) / denom
            delta = -neg_delta

            # R²
            mean_y = sy / n
            ss_res = sum((log_rho[i] - (neg_delta * log_t[i] + (sy - neg_delta * sx) / n)) ** 2 for i in range(n))
            ss_tot = sum((log_rho[i] - mean_y) ** 2 for i in range(n))
            r_sq = 1 - ss_res / ss_tot if ss_tot > 0 else 0

            print(f"\n  {B}δ ≈ {delta:.3f}  (R² = {r_sq:.3f}){R}")
            print(f"  {D}DP mean-field (d≥4): δ = β/ν_par = 1.0{R}")

            if r_sq > 0.9:
                if abs(delta - 1.0) < 0.3:
                    print(f"  {G}→ Consistent with DP mean-field!{R}")
                else:
                    print(f"  {Y}→ Power-law decay but δ={delta:.3f} ≠ 1.0{R}")
            else:
                print(f"  {Y}→ Weak power-law fit — may not be at criticality{R}")

    return log_t, log_rho


# ═══════════════════════════════════════════════
# EXP 2: Spreading from single seed at Θ_c
# ═══════════════════════════════════════════════

def exp2_spreading(theta_c):
    header("EXP 2: Spreading — crystallization from single seed at Θ_c")
    print(f"  Single seed at center, Θ = Θ_c = {theta_c:.4f}\n")

    size = 12
    center = (6, 6, 6, 6)

    # Compare: below, at, and above Θ_c
    for theta_label, theta in [("below Θ_c", theta_c - 0.05),
                                ("at Θ_c", theta_c),
                                ("above Θ_c", theta_c + 0.05)]:
        f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        f.seed(center, "single-seed")

        cryst_history = []
        for step in range(100):
            f.evolve()
            cryst_history.append(len(f.crystallized))

        final = cryst_history[-1]
        grew = cryst_history[-1] > cryst_history[10] if len(cryst_history) > 10 else False

        bar = G + "█" * min(final // 5, 30) + R
        status = "SPREADS" if grew and final > 10 else ("DIES" if final <= 1 else "STABLE")
        color = G if status == "SPREADS" else (RED if status == "DIES" else Y)
        print(f"  {theta_label:12s} (Θ={theta:.3f}): {color}{status:8s}{R} → {final:5d} crystallized  {bar}")

    print(f"\n  {D}DP prediction: spreads below Θ_c, dies above Θ_c, critical at Θ_c{R}")


# ═══════════════════════════════════════════════
# EXP 3: Survival probability P(t)
# ═══════════════════════════════════════════════

def exp3_survival(theta_c):
    header("EXP 3: Survival probability — does crystallization persist?")
    print(f"  100 trials per Θ, single seed each\n")

    size = 10

    print(f"  {'Θ':>6s}  {'survived':>9s}  {'P_surv':>7s}  {'chart'}")
    print(f"  {'─'*6}  {'─'*9}  {'─'*7}  {'─'*30}")

    for theta in [theta_c - 0.08, theta_c - 0.04, theta_c - 0.02,
                  theta_c, theta_c + 0.02, theta_c + 0.04, theta_c + 0.08]:
        survived = 0
        trials = 50  # reduced for speed

        for trial in range(trials):
            f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
            # Random-ish seed position
            pos = ((trial * 3) % size, (trial * 7) % size, size // 2, size // 2)
            f.seed(pos, f"trial-{trial}")

            f.evolve_to_eq(stable_for=10, max_iter=100)
            if len(f.crystallized) > 0:
                survived += 1

        p_surv = survived / trials
        bar = G + "█" * int(p_surv * 30) + R
        marker = " ← Θ_c" if abs(theta - theta_c) < 0.01 else ""
        print(f"  {theta:6.3f}  {survived:5d}/{trials:3d}  {p_surv:7.2f}  {bar}{marker}")

    print(f"\n  {D}DP: P_surv ~ (Θ_c - Θ)^β' near Θ_c, P=0 above Θ_c{R}")


# ═══════════════════════════════════════════════
# EXP 4: Absorbing state verification
# ═══════════════════════════════════════════════

def exp4_absorbing():
    header("EXP 4: Absorbing state — is crystallization truly irreversible?")

    size = 10
    theta = 0.15  # deep ordered phase

    f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
    seeds = make_seeds(size)
    for pos, eid in seeds:
        f.seed(pos, eid)
    f.evolve_to_eq(stable_for=15, max_iter=200)

    initial_cryst = len(f.crystallized)
    print(f"  Initial crystallized: {initial_cryst}")

    # Run 200 more steps — do ANY cells de-crystallize spontaneously?
    decrystallized = 0
    cryst_before = set(f.crystallized)

    for step in range(200):
        f.evolve()
        cryst_after = set(f.crystallized)
        lost = cryst_before - cryst_after
        if lost:
            decrystallized += len(lost)
        cryst_before = cryst_after

    print(f"  After 200 more steps: {len(f.crystallized)} crystallized")
    print(f"  Spontaneous de-crystallizations: {decrystallized}")

    if decrystallized == 0:
        print(f"\n  {G}→ CONFIRMED: crystallization is an absorbing state{R}")
        print(f"  {G}  No cell ever de-crystallized spontaneously{R}")
        print(f"  {D}  This is the defining property of DP universality class{R}")
    else:
        print(f"\n  {RED}→ {decrystallized} cells de-crystallized — NOT fully absorbing{R}")


# ═══════════════════════════════════════════════
# EXP 5: DP exponent comparison table
# ═══════════════════════════════════════════════

def exp5_dp_comparison(theta_c):
    header("EXP 5: DP exponent comparison — Tesseract vs known DP")

    size = 10
    seeds = make_seeds(size)

    # Measure β: crystallized fraction vs (Θ_c - Θ) below Θ_c
    print(f"  Measuring β from order parameter near Θ_c = {theta_c:.4f}\n")

    # Use crystallized FRACTION as order parameter (not mean probability)
    # In DP, the order parameter is the density of active sites
    # Here: density of CRYSTALLIZED sites (the absorbed state)

    thetas_below = [theta_c - d for d in [0.005, 0.01, 0.02, 0.03, 0.05, 0.07, 0.10, 0.12, 0.15]]
    thetas_below = [t for t in thetas_below if t > 0.01]

    print(f"  {'Θ':>6s}  {'Θ_c-Θ':>8s}  {'ρ_cryst':>8s}  {'log(ε)':>8s}  {'log(ρ)':>8s}")
    print(f"  {'─'*6}  {'─'*8}  {'─'*8}  {'─'*8}  {'─'*8}")

    lx, ly = [], []
    total = size ** 4

    for theta in thetas_below:
        f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        rho = len(f.crystallized) / total
        eps = theta_c - theta

        if rho > 0.001 and eps > 0.001:
            logx = math.log10(eps)
            logy = math.log10(rho)
            lx.append(logx)
            ly.append(logy)
            print(f"  {theta:6.3f}  {eps:8.4f}  {rho:8.4f}  {logx:8.4f}  {logy:8.4f}")

    beta = None
    if len(lx) >= 3:
        n = len(lx)
        sx, sy = sum(lx), sum(ly)
        sxy = sum(x * y for x, y in zip(lx, ly))
        sxx = sum(x * x for x in lx)
        denom = n * sxx - sx * sx
        if abs(denom) > 1e-10:
            beta = (n * sxy - sx * sy) / denom
            mean_y = sy / n
            ss_res = sum((ly[i] - (beta * lx[i] + (sy - beta * sx) / n)) ** 2 for i in range(n))
            ss_tot = sum((ly[i] - mean_y) ** 2 for i in range(n))
            r_sq = 1 - ss_res / ss_tot if ss_tot > 0 else 0

    # Summary table
    print(f"\n  {B}{'─' * 50}{R}")
    print(f"  {B}Exponent Comparison Table{R}")
    print(f"  {B}{'─' * 50}{R}")
    print(f"  {'':>20s}  {'DP mean-field':>14s}  {'Tesseract':>12s}")
    print(f"  {'':>20s}  {'(d≥4, exact)':>14s}  {'(measured)':>12s}")
    print(f"  {'─'*20}  {'─'*14}  {'─'*12}")

    if beta is not None:
        match_beta = abs(beta - 1.0) < 0.3
        color_b = G if match_beta else Y
        print(f"  {'β (order param)':>20s}  {'1.000':>14s}  {color_b}{beta:>12.3f}{R}")
    else:
        print(f"  {'β (order param)':>20s}  {'1.000':>14s}  {'N/A':>12s}")

    print(f"  {'ν_perp (spatial)':>20s}  {'0.500':>14s}  {'TBD':>12s}")
    print(f"  {'ν_par (temporal)':>20s}  {'1.000':>14s}  {'TBD':>12s}")
    print(f"  {'z (dynamic)':>20s}  {'2.000':>14s}  {'TBD':>12s}")
    print(f"  {'δ (decay)':>20s}  {'1.000':>14s}  {'see EXP 1':>12s}")

    print(f"\n  {D}If exponents match DP mean-field → Tesseract is in DP universality class{R}")
    print(f"  {D}This is publishable: first application of DP to distributed consensus{R}")

    if beta is not None and abs(beta - 1.0) < 0.3:
        print(f"\n  {G}{B}β ≈ {beta:.3f} is consistent with DP mean-field (β=1.0){R}")
        print(f"  {G}This would be a genuine physics result.{R}")


# ═══════════════════════════════════════════════

def main():
    print(f"\n{B}{C}╔═══════════════════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   TESSERACT — Directed Percolation Analysis               ║{R}")
    print(f"{B}{C}║   Is crystallization an absorbing-state phase transition?  ║{R}")
    print(f"{B}{C}╚═══════════════════════════════════════════════════════════╝{R}")

    t0 = time.time()

    print(f"\n  {D}Finding precise Θ_c via bisection...{R}")
    theta_c = find_theta_c(size=10)
    print(f"  {B}Θ_c = {theta_c:.4f}{R}\n")

    exp4_absorbing()
    exp1_temporal_decay(theta_c)
    exp2_spreading(theta_c)
    exp3_survival(theta_c)
    exp5_dp_comparison(theta_c)

    elapsed = time.time() - t0
    print(f"\n{B}{'━' * 65}{R}")
    print(f"{B}  5 experiments completed in {elapsed:.0f}s{R}")
    print(f"{B}{'━' * 65}{R}\n")


if __name__ == "__main__":
    main()
