#!/usr/bin/env python3
"""
GL Calibration — find parameters that produce a genuine phase transition.

The first GL run failed: seeds too weak, field stayed disordered always.
This script systematically scans (seed_strength, r, J) to find the
region where both phases exist and a transition occurs.

Approach: for each parameter set, check if there's a Θ where
ψ_high > 0.3 (ordered phase exists) and ψ_low < 0.05 (disordered exists).
If both → transition exists at some Θ_c in between.

Stage 1 only — just find working parameters. No publication-quality data.
"""
import math
import sys
import time

sys.path.insert(0, '.')
from field_gl import FieldGL

R = "\033[0m"; B = "\033[1m"; C = "\033[36m"; Y = "\033[33m"
G = "\033[32m"; RED = "\033[31m"; D = "\033[2m"


def scan_transition(size, J, r, seed_str, label=""):
    """Check if a transition exists for these parameters.
    Returns (theta_c, delta_psi) or (0, 0) if no transition."""

    seeds = [
        ((size//4, size//2, size//2, size//2), "A"),
        ((3*size//4, size//2, size//2, size//2), "B"),
        ((size//2, size//4, size//2, size//2), "C"),
        ((size//2, 3*size//4, size//2, size//2), "D"),
    ]

    thetas = [0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80]
    psis = []

    for theta in thetas:
        f = FieldGL(size, theta=theta, J=J, r=r, seed_strength=seed_str)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=10, max_iter=200)
        psi = f.order_param()
        psis.append((theta, psi))

    # Find max drop
    max_drop = 0
    theta_c = 0
    psi_high = max(p for _, p in psis)
    psi_low = min(p for _, p in psis)

    for i in range(1, len(psis)):
        drop = psis[i-1][1] - psis[i][1]
        if drop > max_drop:
            max_drop = drop
            theta_c = psis[i][0]

    return theta_c, max_drop, psi_high, psi_low


def main():
    print(f"\n{B}{C}╔═══════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   GL Calibration — Parameter Scan             ║{R}")
    print(f"{B}{C}╚═══════════════════════════════════════════════╝{R}\n")

    size = 10  # small for speed
    t0 = time.time()

    print(f"  Field: {size}⁴ = {size**4} cells | 4 seeds\n")
    print(f"  {'J':>5s}  {'r':>5s}  {'seed':>5s}  {'ψ_max':>6s}  {'ψ_min':>6s}  {'Δψ':>6s}  {'Θ_c':>5s}  {'verdict'}")
    print(f"  {'─'*5}  {'─'*5}  {'─'*5}  {'─'*6}  {'─'*6}  {'─'*6}  {'─'*5}  {'─'*20}")

    found = []

    for J in [0.05, 0.1, 0.2, 0.3, 0.5]:
        for r in [1.0, 2.0, 4.0, 8.0]:
            for seed_str in [0.3, 0.5, 0.8, 1.0]:
                tc, dd, phi, plo = scan_transition(size, J, r, seed_str)

                has_ordered = phi > 0.15
                has_disordered = plo < 0.08
                has_transition = dd > 0.05

                if has_ordered and has_disordered and has_transition:
                    verdict = f"{G}TRANSITION{R}"
                    found.append((J, r, seed_str, tc, dd, phi, plo))
                elif has_ordered and not has_disordered:
                    verdict = f"{Y}always ordered{R}"
                elif not has_ordered:
                    verdict = f"{RED}always disordered{R}"
                else:
                    verdict = f"{D}crossover{R}"

                print(f"  {J:5.2f}  {r:5.1f}  {seed_str:5.2f}  {phi:6.3f}  {plo:6.3f}  {dd:6.3f}  {tc:5.2f}  {verdict}")

    elapsed = time.time() - t0
    print(f"\n  {B}Scanned in {elapsed:.0f}s{R}")

    if found:
        print(f"\n  {G}{B}Found {len(found)} parameter sets with phase transition:{R}\n")
        # Sort by sharpness of transition
        found.sort(key=lambda x: -x[4])
        for J, r, ss, tc, dd, phi, plo in found[:5]:
            print(f"    J={J:.2f}  r={r:.1f}  seed={ss:.2f}  →  Θ_c≈{tc:.2f}  Δψ={dd:.3f}  (ψ: {plo:.3f}→{phi:.3f})")

        best = found[0]
        print(f"\n  {B}Best: J={best[0]:.2f}, r={best[1]:.1f}, seed={best[2]:.2f}{R}")
        print(f"  {D}Use these for detailed experiments (EXP A-F){R}")
    else:
        print(f"\n  {RED}No transition found. Need wider parameter scan.{R}")


if __name__ == "__main__":
    main()
