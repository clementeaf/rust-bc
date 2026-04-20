#!/usr/bin/env python3
"""
Phase Transition Diagnostics — the 3 tests a referee would demand.

1. Hysteresis: is the transition first-order?
2. Bimodal distribution: is there phase coexistence?
3. Critical nucleus: minimum seeds for survival vs Θ

If hysteresis + bimodal + Θ-dependent nucleus size →
"Discontinuous absorbing-state transition with nucleation"
— a specific, publishable result.
"""
import math
import time
import random
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

def make_seeds(size, n=4):
    h = size // 2
    q = size // 4
    all_seeds = [
        ((q, h, h, h), "ev-A"),
        ((3*q, h, h, h), "ev-B"),
        ((h, q, h, h), "ev-C"),
        ((h, 3*q, h, h), "ev-D"),
    ]
    return all_seeds[:n]


# ═══════════════════════════════════════════════
# TEST 1: Hysteresis
# ═══════════════════════════════════════════════

def test_hysteresis():
    header("TEST 1: Hysteresis — sweep Θ up then down")

    size = 10
    seeds = make_seeds(size)
    thetas = [i/40.0 for i in range(2, 24)]  # 0.05 to 0.575

    # --- Sweep UP: start ordered, increase Θ ---
    print(f"  {B}Sweep UP (ordered → disordered){R}\n")

    # Start with low Θ (ordered phase)
    f_up = FieldGL(size, theta=thetas[0], J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
    for pos, eid in seeds:
        f_up.seed(pos, eid)
    f_up.evolve_to_eq(stable_for=15, max_iter=200)

    up_data = []
    for theta in thetas:
        f_up.theta = theta
        f_up.evolve_to_eq(stable_for=10, max_iter=100)
        psi = f_up.order_param()
        up_data.append((theta, psi))

    # --- Sweep DOWN: start disordered, decrease Θ ---
    print(f"  {B}Sweep DOWN (disordered → ordered){R}\n")

    f_down = FieldGL(size, theta=thetas[-1], J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
    for pos, eid in seeds:
        f_down.seed(pos, eid)
    f_down.evolve_to_eq(stable_for=15, max_iter=200)

    down_data = []
    for theta in reversed(thetas):
        f_down.theta = theta
        f_down.evolve_to_eq(stable_for=10, max_iter=100)
        psi = f_down.order_param()
        down_data.append((theta, psi))
    down_data.reverse()

    # --- Compare ---
    print(f"  {'Θ':>6s}  {'ψ_up':>8s}  {'ψ_down':>8s}  {'Δψ':>8s}  {'up':>15s}  {'down':>15s}")
    print(f"  {'─'*6}  {'─'*8}  {'─'*8}  {'─'*8}  {'─'*15}  {'─'*15}")

    max_gap = 0
    gap_theta = 0
    for i in range(len(thetas)):
        theta = up_data[i][0]
        psi_up = up_data[i][1]
        psi_down = down_data[i][1]
        gap = abs(psi_up - psi_down)
        if gap > max_gap:
            max_gap = gap
            gap_theta = theta

        bar_up = G + "█" * int(psi_up * 15) + R
        bar_down = C + "█" * int(psi_down * 15) + R
        marker = " ←" if gap > 0.1 else ""
        print(f"  {theta:6.3f}  {psi_up:8.4f}  {psi_down:8.4f}  {gap:8.4f}  {bar_up}  {bar_down}{marker}")

    print(f"\n  {B}Max hysteresis gap: Δψ = {max_gap:.4f} at Θ = {gap_theta:.3f}{R}")
    if max_gap > 0.1:
        print(f"  {G}→ HYSTERESIS DETECTED — first-order transition confirmed{R}")
    elif max_gap > 0.03:
        print(f"  {Y}→ Weak hysteresis — borderline first/second order{R}")
    else:
        print(f"  {D}→ No hysteresis — consistent with second-order or crossover{R}")

    return max_gap


# ═══════════════════════════════════════════════
# TEST 2: Bimodal distribution
# ═══════════════════════════════════════════════

def test_bimodal():
    header("TEST 2: Order parameter distribution at Θ_c")

    size = 10
    seeds = make_seeds(size)

    # Find Θ_c via bisection first
    lo, hi = 0.10, 0.35
    for _ in range(12):
        mid = (lo + hi) / 2
        f = FieldGL(size, theta=mid, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=200)
        if f.order_param() > 0.5:
            lo = mid
        else:
            hi = mid
    theta_c = (lo + hi) / 2

    print(f"  Θ_c ≈ {theta_c:.4f}")
    print(f"  Running 50 independent simulations at Θ_c...\n")

    # Run many independent simulations at Θ_c
    psi_values = []
    for trial in range(50):
        f = FieldGL(size, theta=theta_c, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        # Slightly different seed positions per trial for independence
        for i, (pos, eid) in enumerate(seeds):
            offset = ((trial * 3 + i) % 3) - 1  # -1, 0, or +1
            shifted = ((pos[0] + offset) % size, pos[1], pos[2], pos[3])
            f.seed(shifted, f"{eid}-{trial}")
        f.evolve_to_eq(stable_for=15, max_iter=200)
        psi = f.order_param()
        psi_values.append(psi)

    # Build histogram (10 bins from 0 to 1)
    bins = [0] * 10
    for psi in psi_values:
        idx = min(int(psi * 10), 9)
        bins[idx] += 1

    print(f"  {'ψ range':>12s}  {'count':>6s}  {'histogram'}")
    print(f"  {'─'*12}  {'─'*6}  {'─'*30}")

    for i in range(10):
        lo_bin = i / 10
        hi_bin = (i + 1) / 10
        bar = Y + "█" * (bins[i] * 2) + R
        print(f"  {lo_bin:.1f} — {hi_bin:.1f}  {bins[i]:6d}  {bar}")

    # Check bimodality: are there peaks at both low and high ψ?
    low_count = sum(bins[:3])   # ψ < 0.3
    mid_count = sum(bins[3:7])  # 0.3 ≤ ψ < 0.7
    high_count = sum(bins[7:])  # ψ ≥ 0.7

    print(f"\n  Low (ψ<0.3): {low_count}  |  Mid (0.3-0.7): {mid_count}  |  High (ψ>0.7): {high_count}")

    if low_count >= 5 and high_count >= 5:
        print(f"  {G}→ BIMODAL — phase coexistence confirmed (first-order){R}")
    elif low_count >= 5 and mid_count >= 5 and high_count < 3:
        print(f"  {Y}→ Unimodal with tail — could be continuous{R}")
    elif high_count >= 40:
        print(f"  {D}→ Concentrated at high ψ — below Θ_c (ordered){R}")
    elif low_count >= 40:
        print(f"  {D}→ Concentrated at low ψ — above Θ_c (disordered){R}")
    else:
        print(f"  {Y}→ Spread distribution — needs more trials{R}")

    return psi_values


# ═══════════════════════════════════════════════
# TEST 3: Critical nucleus size
# ═══════════════════════════════════════════════

def test_critical_nucleus():
    header("TEST 3: Critical nucleus — minimum seeds for survival vs Θ")

    size = 10
    trials_per = 20

    thetas = [0.05, 0.10, 0.15, 0.18, 0.20, 0.22, 0.25, 0.30]
    seed_counts = [1, 2, 3, 4, 5, 6, 8]

    print(f"  {trials_per} trials per (Θ, n_seeds) combination")
    print(f"  Survival = at least 1 crystallized cell after equilibrium\n")

    # Header
    header_str = f"  {'Θ':>6s}"
    for n in seed_counts:
        header_str += f"  {n:>5d}s"
    header_str += "  nucleus"
    print(header_str)
    print(f"  {'─'*6}" + "  ─────" * len(seed_counts) + "  ─────")

    nucleus_vs_theta = []

    for theta in thetas:
        row = f"  {theta:6.3f}"
        critical_n = None

        for n_seeds in seed_counts:
            survived = 0
            for trial in range(trials_per):
                f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)

                # Place n_seeds at spread positions
                h = size // 2
                positions = [
                    (h - 2, h, h, h),
                    (h + 2, h, h, h),
                    (h, h - 2, h, h),
                    (h, h + 2, h, h),
                    (h, h, h - 2, h),
                    (h, h, h + 2, h),
                    (h - 2, h - 2, h, h),
                    (h + 2, h + 2, h, h),
                ][:n_seeds]

                for i, pos in enumerate(positions):
                    f.seed(pos, f"seed-{i}-t{trial}")

                f.evolve_to_eq(stable_for=10, max_iter=150)

                if len(f.crystallized) > 0:
                    survived += 1

            p_surv = survived / trials_per
            if p_surv > 0.5:
                color = G
                if critical_n is None:
                    critical_n = n_seeds
            elif p_surv > 0:
                color = Y
            else:
                color = RED

            row += f"  {color}{p_surv:5.2f}{R}"

        if critical_n is not None:
            nucleus_vs_theta.append((theta, critical_n))
            row += f"  n*={critical_n}"
        else:
            row += f"  n*>8"

        print(row)

    # Analyze: does nucleus size change with Θ?
    print(f"\n  {B}Critical nucleus size vs Θ:{R}")
    for theta, n_star in nucleus_vs_theta:
        bar = C + "█" * n_star + R
        print(f"    Θ={theta:.2f} → n* = {n_star}  {bar}")

    if len(nucleus_vs_theta) >= 2:
        sizes = [n for _, n in nucleus_vs_theta]
        if max(sizes) > min(sizes):
            print(f"\n  {G}→ Nucleus size CHANGES with Θ — nucleation-driven transition!{R}")
            print(f"  {D}  This is characteristic of hybrid/discontinuous transitions{R}")
            print(f"  {D}  with absorbing states — a specific, publishable phenomenon.{R}")
        else:
            print(f"\n  {Y}→ Nucleus size constant — simple threshold behavior{R}")

    return nucleus_vs_theta


# ═══════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════

def summarize(hysteresis_gap, bimodal_vals, nucleus_data):
    header("SUMMARY — Diagnostic Results")

    has_hysteresis = hysteresis_gap > 0.1
    has_bimodal = False
    if bimodal_vals:
        low = sum(1 for v in bimodal_vals if v < 0.3)
        high = sum(1 for v in bimodal_vals if v > 0.7)
        has_bimodal = low >= 5 and high >= 5

    has_nucleation = False
    if nucleus_data and len(nucleus_data) >= 2:
        sizes = [n for _, n in nucleus_data]
        has_nucleation = max(sizes) > min(sizes)

    print(f"  {'Test':>25s}  {'Result':>12s}  {'Implication'}")
    print(f"  {'─'*25}  {'─'*12}  {'─'*35}")

    h_color = G if has_hysteresis else RED
    print(f"  {'Hysteresis':>25s}  {h_color}{'YES' if has_hysteresis else 'NO':>12s}{R}  {'First-order confirmed' if has_hysteresis else 'May be continuous'}")

    b_color = G if has_bimodal else Y
    print(f"  {'Bimodal distribution':>25s}  {b_color}{'YES' if has_bimodal else 'NO':>12s}{R}  {'Phase coexistence' if has_bimodal else 'Single phase at Θ_c'}")

    n_color = G if has_nucleation else Y
    print(f"  {'Θ-dependent nucleus':>25s}  {n_color}{'YES' if has_nucleation else 'NO':>12s}{R}  {'Nucleation-driven' if has_nucleation else 'Simple threshold'}")

    a_status = G + "CONFIRMED" + R
    print(f"  {'Absorbing state':>25s}  {a_status}  {'Zero de-crystallizations (prev EXP)'}")

    # Classification
    print(f"\n  {B}Classification:{R}")

    if has_hysteresis and has_nucleation:
        print(f"  {G}{B}→ DISCONTINUOUS ABSORBING-STATE TRANSITION WITH NUCLEATION{R}")
        print(f"  {G}  This is a specific phenomenon in non-equilibrium stat-mech.{R}")
        print(f"  {G}  Publishable in Physical Review E or J. Stat. Mech.{R}")
        print(f"\n  {D}  Key references:{R}")
        print(f"  {D}  - Grassberger & Janssen (1979): DP universality conjecture{R}")
        print(f"  {D}  - Hinrichsen (2000): 'Nonequilibrium Critical Phenomena'{R}")
        print(f"  {D}  - da Costa et al. (2010): 'Explosive percolation transition'{R}")
        print(f"  {D}  - D'Souza & Nagler (2015): 'Anomalous critical phenomena'{R}")
    elif has_hysteresis:
        print(f"  {Y}→ First-order transition with absorbing state{R}")
        print(f"  {D}  Less exotic but still publishable{R}")
    else:
        print(f"  {Y}→ Continuous or crossover — needs larger systems to confirm{R}")

    print(f"\n  {B}Suggested paper title:{R}")
    if has_hysteresis and has_nucleation:
        print(f"  {C}'Nucleation-driven discontinuous transition in a 4D absorbing-state")
        print(f"   field: geometric convergence as non-equilibrium phase dynamics'{R}")
    else:
        print(f"  {C}'Absorbing-state phase transition in a discrete field on a 4D")
        print(f"   toroidal lattice with irreversible crystallization'{R}")


# ═══════════════════════════════════════════════

def main():
    print(f"\n{B}{C}╔═══════════════════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   TESSERACT — Phase Transition Diagnostics               ║{R}")
    print(f"{B}{C}║   The 3 tests a referee would demand                     ║{R}")
    print(f"{B}{C}╚═══════════════════════════════════════════════════════════╝{R}")

    t0 = time.time()

    hyst = test_hysteresis()
    bimodal = test_bimodal()
    nucleus = test_critical_nucleus()
    summarize(hyst, bimodal, nucleus)

    elapsed = time.time() - t0
    print(f"\n{B}{'━' * 65}{R}")
    print(f"{B}  3 diagnostics completed in {elapsed:.0f}s{R}")
    print(f"{B}{'━' * 65}{R}\n")


if __name__ == "__main__":
    main()
