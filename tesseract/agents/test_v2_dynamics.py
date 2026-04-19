#!/usr/bin/env python3
"""
Test v2 field dynamics — do dimensions matter now?
Does correlation respond to Θ? Is self-healing still conditional?

Compares v1 (original) vs v2 (source-aware σ + crystallization cascade).
"""
import math
import time
import sys
sys.path.insert(0, '.')
from field_v2 import FieldV2

# Also import v1 for comparison
from phase_transition_v2 import Field as FieldV1

R = "\033[0m"
B = "\033[1m"
C = "\033[36m"
Y = "\033[33m"
G = "\033[32m"
RED = "\033[31m"
D = "\033[2m"

def header(name):
    print(f"\n{B}{Y}{'━' * 65}{R}")
    print(f"{B}{Y}  {name}{R}")
    print(f"{B}{Y}{'━' * 65}{R}\n")


# ══════════════════════════════════════════════════════════
# TEST 1: Do dimensions matter now?
# ══════════════════════════════════════════════════════════

def test_dimensions():
    header("TEST 1: Dimension effect — v1 vs v2")

    size = 16
    theta = 0.85

    for version_label, FieldClass in [("v1 (original)", FieldV1), ("v2 (source-aware)", FieldV2)]:
        print(f"  {B}{version_label}{R}")

        for dims, label in [(2, "2D"), (3, "3D"), (4, "4D")]:
            f = FieldClass(size, theta=theta)
            center = (8, 8, 8, 8)

            # Seed from DIFFERENT axis directions to test orthogonal support
            # 2D: seed along t and c axes
            # 3D: add o axis
            # 4D: add v axis
            seeds = [
                ((4, 8, 8, 8), "event-t"),   # from t-axis
                ((8, 4, 8, 8), "event-c"),   # from c-axis
            ]
            if dims >= 3:
                seeds.append(((8, 8, 4, 8), "event-o"))  # from o-axis
            if dims >= 4:
                seeds.append(((8, 8, 8, 4), "event-v"))  # from v-axis

            for coord, eid in seeds:
                f.seed(coord, eid)

            f.evolve_to_eq(stable_for=5, max_iter=80)

            nc = len(f.crystallized)
            na = len(f.cells)
            bar = G + "█" * min(nc // 50, 40) + R

            # Check σ at center
            if hasattr(f, 'orthogonal_support'):
                sigma = f.orthogonal_support(center)
            else:
                sigma = "?"

            print(f"    {label}: cryst={nc:6d}  active={na:6d}  σ(center)={sigma}  {bar}")

        print()

    print(f"  {D}v1: σ based on neighbor probability → same for all dimensions{R}")
    print(f"  {D}v2: σ based on source diversity → more dimensions = more diverse sources{R}")


# ══════════════════════════════════════════════════════════
# TEST 2: Correlation length vs Θ — does it respond now?
# ══════════════════════════════════════════════════════════

def test_correlation():
    header("TEST 2: Correlation decay — v1 vs v2")

    size = 32

    for version_label, FieldClass in [("v1 (original)", FieldV1), ("v2 (source-aware)", FieldV2)]:
        print(f"  {B}{version_label}{R}")

        # Seed TWO events so v2 has diverse sources
        center_a = (16, 16, 16, 16)
        center_b = (16, 12, 16, 16)  # offset on c-axis

        for theta in [0.30, 0.60, 0.85, 0.95]:
            f = FieldClass(size, theta=theta, seed_radius=4)
            f.seed(center_a, "probe-A")
            f.seed(center_b, "probe-B")
            f.evolve_to_eq(stable_for=3, max_iter=40)

            # Measure along t-axis from center_a
            probs = []
            for d in range(0, 10):
                coord = ((center_a[0]+d)%size, center_a[1], center_a[2], center_a[3])
                p = f.cells.get(coord, 0.0)
                probs.append(p)

            # Find correlation length: distance where p drops below 0.1
            xi = 0
            for d, p in enumerate(probs):
                if p >= 0.05:
                    xi = d

            prob_str = " ".join(f"{p:.2f}" for p in probs[:7])
            print(f"    Θ={theta:.2f}  ξ={xi}  p(d): {prob_str}")

        print()


# ══════════════════════════════════════════════════════════
# TEST 3: Self-healing with sparse seeds — v1 vs v2
# ══════════════════════════════════════════════════════════

def test_healing_sparse():
    header("TEST 3: Self-healing with 1 and 2 seeds — v1 vs v2")

    size = 16

    for version_label, FieldClass in [("v1 (original)", FieldV1), ("v2 (source-aware)", FieldV2)]:
        print(f"  {B}{version_label}{R}")

        for num_seeds in [1, 2, 3, 4]:
            f = FieldClass(size, theta=0.85)

            seed_positions = [
                (8, 8, 8, 8),
                (8, 4, 8, 8),
                (4, 8, 8, 8),
                (8, 8, 4, 8),
            ][:num_seeds]

            for i, pos in enumerate(seed_positions):
                f.seed(pos, f"seed-{i}")

            f.evolve_to_eq(stable_for=5, max_iter=80)
            before = len(f.crystallized)

            if before == 0:
                print(f"    {num_seeds} seeds → 0 crystallized → (skip)")
                continue

            # Destroy the center cell
            target = (8, 8, 8, 8)
            was_cryst = target in f.crystallized
            if not was_cryst:
                # Find any crystallized cell
                if f.crystallized:
                    target = next(iter(f.crystallized))
                else:
                    print(f"    {num_seeds} seeds → no crystal to destroy")
                    continue

            f.destroy(target)
            steps = f.evolve_to_eq(stable_for=5, max_iter=100)
            healed = target in f.crystallized

            status = G + "HEALED" if healed else RED + "LOST"
            print(f"    {num_seeds} seeds → {before:5d} cryst → destroy → {status} ({steps} steps){R}")

        print()


# ══════════════════════════════════════════════════════════
# TEST 4: Phase transition — v2 with multiple seeds
# ══════════════════════════════════════════════════════════

def test_phase_transition_v2():
    header("TEST 4: Phase transition — v2 with 4 orthogonal seeds")

    size = 16
    seeds = [
        ((4, 8, 8, 8), "ev-t"),
        ((8, 4, 8, 8), "ev-c"),
        ((8, 8, 4, 8), "ev-o"),
        ((8, 8, 8, 4), "ev-v"),
    ]

    thetas = [i/20.0 for i in range(1, 21)]

    print(f"  {'Θ':>5s}  {'cryst':>6s}  {'ψ':>8s}  {'chart'}")
    print(f"  {'─'*5}  {'─'*6}  {'─'*8}  {'─'*35}")

    prev_psi = None
    max_drop = 0
    theta_c = 0

    for theta in thetas:
        f = FieldV2(size, theta=theta)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=5, max_iter=80)

        psi = f.order_param()
        nc = f.cryst_count()

        bar_len = int(psi * 100)
        bar = G + "█" * min(bar_len, 35) + R if psi > 0.001 else D + "·" + R

        if prev_psi is not None:
            drop = prev_psi - psi
            if drop > max_drop:
                max_drop = drop
                theta_c = theta

        print(f"  {theta:5.2f}  {nc:6d}  {psi:8.5f}  {bar}")
        prev_psi = psi

    print(f"\n  {B}Θ_c ≈ {theta_c:.2f} (Δψ = {max_drop:.5f}){R}")
    if max_drop > 0.05:
        print(f"  {G}→ Phase transition detected!{R}")
    else:
        print(f"  {Y}→ Gradual decline — check with larger field{R}")


# ══════════════════════════════════════════════════════════
# TEST 5: Healing time scaling — v2
# ══════════════════════════════════════════════════════════

def test_healing_scaling_v2():
    header("TEST 5: Healing time scaling — v2 (does α still ≈ 0?)")

    size = 16

    # Create dense field with diverse sources
    def make_field():
        f = FieldV2(size, theta=0.85)
        for i in range(6):
            for j in range(6):
                f.seed(((i*2+2)%size, (j*2+2)%size, 8, 8), f"grid-{i}-{j}")
        f.evolve_to_eq(stable_for=5, max_iter=60)
        return f

    base = make_field()
    initial = base.cryst_count()
    print(f"  Base: {initial} crystallized\n")

    print(f"  {'damage':>7s}  {'%':>5s}  {'steps':>6s}  {'recovered':>10s}")
    print(f"  {'─'*7}  {'─'*5}  {'─'*6}  {'─'*10}")

    data = []
    for pct in [1, 5, 10, 20, 30, 50]:
        f = make_field()
        n = max(1, int(f.cryst_count() * pct / 100))
        targets = list(f.crystallized)[:n]
        for t in targets:
            f.destroy(t)
        after_d = f.cryst_count()

        steps = 0
        for s in range(300):
            steps = s + 1
            nc = f.evolve()
            if nc == 0:
                break

        f.evolve_to_eq(stable_for=3, max_iter=50)
        after_h = f.cryst_count()
        rec = after_h - after_d
        rec_pct = (rec / n * 100) if n > 0 else 0

        print(f"  {n:7d}  {pct:4d}%  {steps:6d}  {rec_pct:9.0f}%")
        if n > 0 and steps > 0:
            data.append((n, steps))

    if len(data) >= 3:
        xs = [math.log(d) for d, s in data if d > 0 and s > 0]
        ys = [math.log(s) for d, s in data if d > 0 and s > 0]
        n = len(xs)
        if n >= 2:
            sx, sy = sum(xs), sum(ys)
            sxy = sum(x*y for x, y in zip(xs, ys))
            sxx = sum(x*x for x in xs)
            denom = n*sxx - sx*sx
            if abs(denom) > 1e-10:
                alpha = (n*sxy - sx*sy) / denom
                print(f"\n  {B}α ≈ {alpha:.3f}{R}")
                if abs(alpha) < 0.3:
                    print(f"  {G}→ Near-constant healing time — topological-like{R}")
                elif alpha < 1:
                    print(f"  {Y}→ Sub-linear — good but not topological{R}")
                else:
                    print(f"  {RED}→ Linear or worse{R}")


# ══════════════════════════════════════════════════════════

def main():
    print(f"\n{B}{C}╔══════════════════════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   TESSERACT v2 — Fixed Dynamics Evaluation                   ║{R}")
    print(f"{B}{C}║   Source-aware σ + crystallization cascade                    ║{R}")
    print(f"{B}{C}╚══════════════════════════════════════════════════════════════╝{R}")

    t0 = time.time()

    test_dimensions()
    test_correlation()
    test_healing_sparse()
    test_phase_transition_v2()
    test_healing_scaling_v2()

    elapsed = time.time() - t0
    print(f"\n{B}{'━' * 65}{R}")
    print(f"{B}  5 tests completed in {elapsed:.1f}s{R}")
    print(f"{B}{'━' * 65}{R}\n")


if __name__ == "__main__":
    main()
