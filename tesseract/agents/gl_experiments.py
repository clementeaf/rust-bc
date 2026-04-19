#!/usr/bin/env python3
"""
GL Experiments — calibrated parameters.
Best params: J=0.10, r=8.0, seed_strength=0.50

6 experiments for physicist evaluation:
A. Phase transition (fine Θ scan)
B. Finite-size scaling (Θ_c vs S)
C. Critical exponent β
D. Correlation length divergence
E. Self-healing in ordered phase
F. Dimension effect
"""
import math
import time
import sys
sys.path.insert(0, '.')
from field_gl import FieldGL

R = "\033[0m"; B = "\033[1m"; C = "\033[36m"; Y = "\033[33m"
G = "\033[32m"; RED = "\033[31m"; D = "\033[2m"

# Calibrated parameters
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
# EXP A: Fine Θ scan
# ═══════════════════════════════════════════════

def exp_a():
    header("EXP A: Phase transition — fine Θ scan (J=0.10, r=8.0)")

    size = 10
    seeds = make_seeds(size)
    thetas = [i/40.0 for i in range(1, 32)]  # 0.025 to 0.775

    print(f"  {'Θ':>6s}  {'ψ':>8s}  {'cryst':>6s}  {'chart'}")
    print(f"  {'─'*6}  {'─'*8}  {'─'*6}  {'─'*40}")

    prev = None
    max_drop = 0
    tc = 0
    data = []

    for theta in thetas:
        f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        psi = f.order_param()
        nc = len(f.crystallized)
        data.append((theta, psi, nc))

        if prev is not None:
            drop = prev - psi
            if drop > max_drop:
                max_drop = drop
                tc = theta

        bar_n = int(psi * 40)
        color = G if psi > 0.3 else (Y if psi > 0.05 else D)
        bar = color + "█" * min(bar_n, 40) + R
        marker = " ← Θ_c" if prev is not None and (prev - psi) > max_drop * 0.9 and max_drop > 0.1 else ""
        print(f"  {theta:6.3f}  {psi:8.4f}  {nc:6d}  {bar}{marker}")
        prev = psi

    print(f"\n  {B}Θ_c ≈ {tc:.3f}  |  Δψ = {max_drop:.4f}{R}")
    if max_drop > 0.5:
        print(f"  {G}→ Sharp transition — possibly first order{R}")
    elif max_drop > 0.1:
        print(f"  {Y}→ Transition detected — needs β analysis{R}")
    return tc, data


# ═══════════════════════════════════════════════
# EXP B: Finite-size scaling
# ═══════════════════════════════════════════════

def exp_b():
    header("EXP B: Finite-size scaling — Θ_c(S)")

    sizes = [8, 10, 12]
    results = []

    for size in sizes:
        seeds = make_seeds(size)
        thetas = [i/40.0 for i in range(2, 30)]
        prev = None
        max_drop = 0
        tc = 0

        for theta in thetas:
            f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
            for pos, eid in seeds:
                f.seed(pos, eid)
            f.evolve_to_eq(stable_for=12, max_iter=250)
            psi = f.order_param()

            if prev is not None:
                drop = prev - psi
                if drop > max_drop:
                    max_drop = drop
                    tc = theta
            prev = psi

        results.append((size, tc, max_drop))
        print(f"  S={size:2d} ({size**4:6d} cells)  Θ_c ≈ {tc:.3f}  Δψ = {max_drop:.4f}")

    # Check if Θ_c shifts with S
    if len(results) >= 2:
        shift = abs(results[-1][1] - results[0][1])
        if shift > 0.01:
            print(f"\n  {G}Θ_c shifts by {shift:.3f} — finite-size effects present{R}")

            # Crude ν estimate: Θ_c(S) = Θ_c(∞) + a·S^(-1/ν)
            if len(results) >= 3:
                s1, tc1, _ = results[0]
                s3, tc3, _ = results[-1]
                if abs(tc1 - tc3) > 0.001:
                    try:
                        inv_nu = math.log(abs(tc1 - tc3)) / math.log(s1 / s3) if s1 != s3 else 0
                        if abs(inv_nu) > 0.01:
                            nu = 1.0 / abs(inv_nu)
                            print(f"  {B}Crude ν ≈ {nu:.2f}{R}")
                            if abs(nu - 0.5) < 0.15:
                                print(f"  {G}→ Close to mean-field (ν=0.5){R}")
                            elif abs(nu - 0.63) < 0.15:
                                print(f"  {C}→ Close to Ising 3D (ν=0.63){R}")
                    except (ValueError, ZeroDivisionError):
                        pass
        else:
            print(f"\n  {Y}Θ_c stable — mean-field or insufficient size range{R}")

    return results


# ═══════════════════════════════════════════════
# EXP C: Critical exponent β
# ═══════════════════════════════════════════════

def exp_c(theta_c, data):
    header("EXP C: Critical exponent β")

    if theta_c <= 0 or not data:
        print(f"  {RED}Need Θ_c from EXP A{R}")
        return

    print(f"  Θ_c = {theta_c:.3f}\n")
    print(f"  {'Θ':>6s}  {'Θ_c-Θ':>8s}  {'ψ':>8s}  {'log(Θ_c-Θ)':>11s}  {'log(ψ)':>8s}")
    print(f"  {'─'*6}  {'─'*8}  {'─'*8}  {'─'*11}  {'─'*8}")

    lx, ly = [], []

    for theta, psi, _ in data:
        if theta < theta_c and psi > 0.02:
            diff = theta_c - theta
            if diff > 0.005:
                logx = math.log10(diff)
                logy = math.log10(psi)
                lx.append(logx)
                ly.append(logy)
                print(f"  {theta:6.3f}  {diff:8.4f}  {psi:8.4f}  {logx:11.4f}  {logy:8.4f}")

    if len(lx) >= 3:
        n = len(lx)
        sx, sy = sum(lx), sum(ly)
        sxy = sum(x*y for x, y in zip(lx, ly))
        sxx = sum(x*x for x in lx)
        denom = n * sxx - sx * sx
        if abs(denom) > 1e-10:
            beta = (n * sxy - sx * sy) / denom
            # R² for fit quality
            mean_y = sy / n
            ss_res = sum((ly[i] - (beta * lx[i] + (sy - beta * sx) / n)) ** 2 for i in range(n))
            ss_tot = sum((ly[i] - mean_y) ** 2 for i in range(n))
            r_sq = 1 - ss_res / ss_tot if ss_tot > 0 else 0

            print(f"\n  {B}β ≈ {beta:.3f}  (R² = {r_sq:.3f}){R}")

            if r_sq < 0.8:
                print(f"  {RED}→ Poor fit — may not be power-law{R}")
            elif abs(beta - 0.5) < 0.15:
                print(f"  {G}→ Mean-field (β=0.5) — expected for d≥4{R}")
            elif abs(beta - 0.125) < 0.05:
                print(f"  {C}→ Ising 2D (β=1/8){R}")
            elif abs(beta - 0.326) < 0.08:
                print(f"  {C}→ Ising 3D (β=0.326){R}")
            elif beta > 0 and beta < 2:
                print(f"  {Y}→ β={beta:.3f} — possibly novel exponent{R}")
    else:
        print(f"\n  {RED}Not enough data points near Θ_c for β fit{R}")


# ═══════════════════════════════════════════════
# EXP D: Correlation length
# ═══════════════════════════════════════════════

def exp_d(theta_c):
    header("EXP D: Correlation length ξ(Θ)")

    size = 12
    seeds = make_seeds(size)
    center = (size//2, size//2, size//2, size//2)

    # Scan Θ around Θ_c
    thetas = sorted(set([
        0.05, 0.10, max(0.025, theta_c - 0.15), max(0.025, theta_c - 0.10),
        max(0.025, theta_c - 0.05), theta_c, min(0.95, theta_c + 0.05),
        min(0.95, theta_c + 0.10), min(0.95, theta_c + 0.15),
        0.40, 0.60, 0.80
    ]))

    print(f"  Θ_c ≈ {theta_c:.3f}\n")
    print(f"  {'Θ':>6s}  {'ξ':>4s}  {'ψ':>8s}  {'p(d=0..6)'}")
    print(f"  {'─'*6}  {'─'*4}  {'─'*8}  {'─'*35}")

    xi_values = []

    for theta in thetas:
        f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        psi = f.order_param()
        xi = f.correlation_length(center)
        xi_values.append((theta, xi))

        profile = []
        for d in range(7):
            coord = ((center[0]+d) % size, center[1], center[2], center[3])
            p = f.cells.get(coord, 0.0)
            profile.append(f"{p:.2f}")

        bar = C + "█" * int(xi * 2) + R
        marker = " ← Θ_c" if abs(theta - theta_c) < 0.01 else ""
        print(f"  {theta:6.3f}  {xi:4.0f}  {psi:8.4f}  {' '.join(profile)}  {bar}{marker}")

    # Check for divergence
    if xi_values:
        max_xi = max(xi_values, key=lambda x: x[1])
        print(f"\n  {B}Max ξ = {max_xi[1]:.0f} at Θ = {max_xi[0]:.3f}{R}")
        if abs(max_xi[0] - theta_c) < 0.1:
            print(f"  {G}→ ξ peaks near Θ_c — consistent with critical phenomenon{R}")
        else:
            print(f"  {Y}→ ξ peak not at Θ_c — check parameter regime{R}")


# ═══════════════════════════════════════════════
# EXP E: Self-healing
# ═══════════════════════════════════════════════

def exp_e(theta_c):
    header("EXP E: Self-healing in GL ordered phase")

    size = 10

    # Test at different Θ: deep ordered, near Θ_c, disordered
    test_thetas = [
        max(0.025, theta_c - 0.15),
        max(0.025, theta_c - 0.05),
        min(0.95, theta_c + 0.05),
    ]

    for theta in test_thetas:
        seeds = make_seeds(size)
        f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        initial = len(f.crystallized)
        psi = f.order_param()
        phase = "ordered" if psi > 0.3 else "disordered"

        if initial == 0:
            print(f"  Θ={theta:.3f} ({phase}): 0 crystallized — no healing test possible")
            continue

        # Destroy 10%
        damage = max(1, initial // 10)
        targets = list(f.crystallized)[:damage]
        for t in targets:
            f.destroy(t)

        after_d = len(f.crystallized)
        steps = f.evolve_to_eq(stable_for=10, max_iter=200)
        after_h = len(f.crystallized)
        rec = after_h - after_d
        pct = (rec / damage * 100) if damage > 0 else 0

        status = G + "FULL" if rec >= damage else (Y + f"{pct:.0f}%" if rec > 0 else RED + "NONE")
        print(f"  Θ={theta:.3f} ({phase:10s}): {initial:5d} cryst → destroy {damage:4d} → {status} recovery ({steps} steps){R}")

    print(f"\n  {D}Self-healing should work in ordered phase, fail in disordered{R}")


# ═══════════════════════════════════════════════
# EXP F: Dimension effect
# ═══════════════════════════════════════════════

def exp_f(theta_c):
    header("EXP F: Dimension effect in GL")

    size = 10
    theta = max(0.025, theta_c - 0.10)  # ordered phase
    h = size // 2

    for dims, label in [(2, "2D"), (3, "3D"), (4, "4D")]:
        f = FieldGL(size, theta=theta, J=J_CAL, r=R_CAL, seed_strength=SEED_CAL)

        seed_list = [
            ((size//4, h, h, h), "ax-t"),
            ((h, size//4, h, h), "ax-c"),
        ]
        if dims >= 3:
            seed_list.append(((h, h, size//4, h), "ax-o"))
        if dims >= 4:
            seed_list.append(((h, h, h, size//4), "ax-v"))

        for pos, eid in seed_list:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        nc = len(f.crystallized)
        psi = f.order_param()
        bar = G + "█" * min(nc // 10, 40) + R
        print(f"  {label}: cryst={nc:6d}  ψ={psi:.4f}  {bar}")

    print(f"\n  {D}More dimensions → more coupling paths → easier ordering{R}")


# ═══════════════════════════════════════════════

def main():
    print(f"\n{B}{C}╔═══════════════════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   TESSERACT GL — Calibrated Experiments                   ║{R}")
    print(f"{B}{C}║   J=0.10  r=8.0  seed=0.50                               ║{R}")
    print(f"{B}{C}╚═══════════════════════════════════════════════════════════╝{R}")

    t0 = time.time()

    theta_c, data = exp_a()
    exp_b()
    exp_c(theta_c, data)
    exp_d(theta_c)
    exp_e(theta_c)
    exp_f(theta_c)

    elapsed = time.time() - t0
    print(f"\n{B}{'━' * 65}{R}")
    print(f"{B}  6 experiments completed in {elapsed:.0f}s{R}")
    print(f"{B}{'━' * 65}{R}\n")


if __name__ == "__main__":
    main()
