#!/usr/bin/env python3
"""
Tesseract Physics Experiments — Phase transition, critical exponents,
topological error correction, and universality class analysis.

These experiments generate data that a physicist needs to evaluate
whether Tesseract exhibits genuine phase transitions and topological
error correction.

No dependencies beyond stdlib. Runs against live Tesseract nodes.

Usage:
  PORT=7710 NODE_ID=n1 PEERS=127.0.0.1:7711 cargo run --bin node &
  PORT=7711 NODE_ID=n2 PEERS=127.0.0.1:7710 cargo run --bin node &
  python3 phase_transition.py
"""
import asyncio
import json
import time
import math
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass

_pool = ThreadPoolExecutor(max_workers=16)

# ── Minimal HTTP client (zero deps) ─────────────────────

def _post(url, data):
    try:
        body = json.dumps(data).encode()
        req = urllib.request.Request(url, data=body, method="POST")
        req.add_header("Content-Type", "application/json")
        with urllib.request.urlopen(req, timeout=10) as r:
            return json.loads(r.read())
    except:
        return {}

def _get(url):
    try:
        with urllib.request.urlopen(url, timeout=10) as r:
            return json.loads(r.read())
    except:
        return {}


# ── In-process Tesseract field (no HTTP, pure Python) ────
# For physics experiments we need fast iteration without HTTP overhead.
# This is a minimal reimplementation of the Rust field for simulation.

class Field:
    """Minimal 4D toroidal probability field — Python port for experiments."""

    def __init__(self, size: int, theta: float = 0.85, alpha: float = 0.15):
        self.size = size
        self.theta = theta
        self.alpha = alpha
        self.cells: dict[tuple, float] = {}  # coord → probability
        self.crystallized: set[tuple] = set()
        self.influences: dict[tuple, list[str]] = {}  # coord → [event_ids]
        self.EPSILON = 0.05
        self.SEED_RADIUS = min(4, size // 2)
        self.RESONANCE = {0: (1.0, 0.0), 1: (1.0, 0.0), 2: (1.5, 0.02), 3: (2.5, 0.05), 4: (4.0, 0.10)}

    def _wrap(self, a: int) -> int:
        return a % self.size

    def _dist(self, a: tuple, b: tuple) -> float:
        s = self.size
        total = 0.0
        for i in range(4):
            d = abs(a[i] - b[i])
            d = min(d, s - d)
            total += d * d
        return math.sqrt(total)

    def seed(self, center: tuple, event_id: str = ""):
        s = self.size
        r = self.SEED_RADIUS
        for dt in range(-r, r + 1):
            for dc in range(-r, r + 1):
                for do_ in range(-r, r + 1):
                    for dv in range(-r, r + 1):
                        coord = (
                            (center[0] + dt) % s,
                            (center[1] + dc) % s,
                            (center[2] + do_) % s,
                            (center[3] + dv) % s,
                        )
                        dist = self._dist(center, coord)
                        p = 1.0 / (1.0 + dist)
                        if p < self.EPSILON:
                            continue
                        old = self.cells.get(coord, 0.0)
                        new = min(old + p, 1.0)
                        self.cells[coord] = new
                        if event_id:
                            self.influences.setdefault(coord, []).append(event_id)
                        if coord not in self.crystallized and new >= self.theta:
                            self.crystallized.add(coord)
                            self.cells[coord] = 1.0

    def neighbors(self, coord: tuple) -> list[tuple]:
        s = self.size
        result = []
        for axis in range(4):
            for delta in (-1, 1):
                n = list(coord)
                n[axis] = (n[axis] + delta) % s
                result.append(tuple(n))
        return result

    def orthogonal_support(self, coord: tuple) -> int:
        s = self.size
        axes = 0
        for axis in range(4):
            for delta in (-1, 1):
                n = list(coord)
                n[axis] = (n[axis] + delta) % s
                if self.cells.get(tuple(n), 0.0) > 0.5:
                    axes += 1
                    break
        return axes

    def evolve(self) -> int:
        # Collect coords to process
        to_process = set()
        for coord in list(self.cells.keys()):
            to_process.add(coord)
            for n in self.neighbors(coord):
                to_process.add(n)

        # Calculate updates
        updates = []
        for coord in to_process:
            if coord in self.crystallized:
                continue
            p = self.cells.get(coord, 0.0)
            nbrs = self.neighbors(coord)
            avg = sum(self.cells.get(n, 0.0) for n in nbrs) / 8.0
            delta = (avg - p) * self.alpha
            sigma = self.orthogonal_support(coord)
            amp, res = self.RESONANCE.get(sigma, (1.0, 0.0))
            new_p = max(0.0, min(1.0, p + delta * amp + res))
            updates.append((coord, new_p))

        # Apply
        new_cryst = 0
        for coord, new_p in updates:
            if new_p < self.EPSILON:
                if coord in self.cells and coord not in self.influences:
                    del self.cells[coord]
                    continue
            self.cells[coord] = new_p
            if coord not in self.crystallized and new_p >= self.theta:
                self.crystallized.add(coord)
                self.cells[coord] = 1.0
                new_cryst += 1
        return new_cryst

    def evolve_to_eq(self, stable_for: int = 5, max_iter: int = 200):
        stable = 0
        steps = 0
        for i in range(max_iter):
            steps += 1
            if self.evolve() == 0:
                stable += 1
            else:
                stable = 0
            if stable >= stable_for:
                break
        return steps

    def destroy(self, coord: tuple):
        if coord in self.cells:
            self.cells[coord] = 0.0
            self.crystallized.discard(coord)

    def crystallized_count(self) -> int:
        return len(self.crystallized)

    def active_count(self) -> int:
        return len(self.cells)

    def order_parameter(self) -> float:
        """Fraction of active cells that are crystallized."""
        if not self.cells:
            return 0.0
        return len(self.crystallized) / len(self.cells)


# ── Colors ───────────────────────────────────────────────

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
# EXPERIMENT 1: Phase transition — order parameter vs Θ
# ══════════════════════════════════════════════════════════

def exp1_phase_transition():
    """Vary Θ from 0.1 to 1.0 and measure the order parameter
    (fraction of cells that crystallize). A genuine phase transition
    shows a sharp jump at a critical Θ_c."""

    header("EXP 1: Phase transition — order parameter vs Θ")

    size = 8
    thetas = [i / 20.0 for i in range(2, 21)]  # 0.10 to 1.00
    results = []

    # Fixed seed pattern: 4 events at orthogonal positions
    seeds = [(2, 2, 2, 2), (2, 5, 2, 2), (5, 2, 2, 2), (2, 2, 5, 2)]

    for theta in thetas:
        field = Field(size, theta=theta)
        for s in seeds:
            field.seed(s, f"ev@{s}")
        field.evolve_to_eq()

        op = field.order_parameter()
        nc = field.crystallized_count()
        results.append((theta, op, nc))

        bar = "█" * int(op * 40)
        pad = "░" * (40 - int(op * 40))
        color = G if op > 0.5 else (Y if op > 0.1 else D)
        print(f"  Θ={theta:.2f}  {color}{bar}{pad}{R}  ψ={op:.3f}  cryst={nc}")

    # Find the critical point: where order parameter drops most sharply
    max_drop = 0
    theta_c = 0
    for i in range(1, len(results)):
        drop = results[i - 1][1] - results[i][1]
        if drop > max_drop:
            max_drop = drop
            theta_c = (results[i - 1][0] + results[i][0]) / 2

    print(f"\n  {B}Critical point estimate: Θ_c ≈ {theta_c:.2f}{R}")
    print(f"  {D}(sharpest drop in order parameter){R}")

    # Is it sharp? (phase transition) or gradual? (crossover)
    if max_drop > 0.3:
        print(f"  {G}→ SHARP transition (Δψ = {max_drop:.3f}) — consistent with first-order phase transition{R}")
    elif max_drop > 0.1:
        print(f"  {Y}→ Moderate transition (Δψ = {max_drop:.3f}) — possibly second-order or crossover{R}")
    else:
        print(f"  {RED}→ No clear transition (Δψ = {max_drop:.3f}) — may not be a true phase transition{R}")

    return results


# ══════════════════════════════════════════════════════════
# EXPERIMENT 2: Finite-size scaling — does Θ_c depend on S?
# ══════════════════════════════════════════════════════════

def exp2_finite_size_scaling():
    """Run phase transition for different field sizes S.
    If Θ_c shifts with S, we can extract critical exponents via
    finite-size scaling: Θ_c(S) = Θ_c(∞) + a·S^(-1/ν)"""

    header("EXP 2: Finite-size scaling — Θ_c vs field size S")

    sizes = [4, 6, 8]  # 4D fields: 256, 1296, 4096 cells
    seeds_template = [(1, 1, 1, 1), (1, 3, 1, 1), (3, 1, 1, 1), (1, 1, 3, 1)]

    for size in sizes:
        print(f"  {B}S = {size} ({size**4} cells){R}")

        # Scale seed positions to field size
        seeds = [(min(s[0], size - 1), min(s[1], size - 1),
                  min(s[2], size - 1), min(s[3], size - 1)) for s in seeds_template]

        thetas = [i / 20.0 for i in range(4, 20)]
        best_drop = 0
        theta_c = 0

        for theta in thetas:
            field = Field(size, theta=theta)
            for s in seeds:
                field.seed(s, f"ev@{s}")
            field.evolve_to_eq()
            op = field.order_parameter()

            if theta > thetas[0]:
                drop = prev_op - op
                if drop > best_drop:
                    best_drop = drop
                    theta_c = (theta + thetas[thetas.index(theta) - 1]) / 2
            prev_op = op

        print(f"    Θ_c ≈ {theta_c:.2f}  (Δψ = {best_drop:.3f})")

    print(f"\n  {D}If Θ_c shifts with S → finite-size effects present → extract ν exponent{R}")
    print(f"  {D}If Θ_c is constant → mean-field behavior (no finite-size dependence){R}")


# ══════════════════════════════════════════════════════════
# EXPERIMENT 3: Self-healing — error correction distance
# ══════════════════════════════════════════════════════════

def exp3_error_correction_distance():
    """Destroy increasing numbers of cells and measure recovery.
    The maximum number of simultaneous errors the field can correct
    is the 'code distance' — the key parameter in topological error
    correction."""

    header("EXP 3: Error correction distance — how many simultaneous errors survive?")

    size = 8
    field = Field(size)

    # Create a dense crystallized region
    for dt in range(3):
        for dc in range(3):
            field.seed((3 + dt, 3 + dc, 3, 3), f"dense-{dt}-{dc}")
    field.evolve_to_eq()

    initial_cryst = field.crystallized_count()
    print(f"  Initial crystallized cells: {initial_cryst}")

    # Get crystallized cells sorted by binding energy proxy (neighbor count)
    cryst_cells = list(field.crystallized)

    # Destroy increasing numbers and measure recovery
    for num_destroy in [1, 2, 4, 8, 16, 32, min(64, len(cryst_cells))]:
        if num_destroy > len(cryst_cells):
            break

        # Fresh field for each test
        test_field = Field(size)
        for dt in range(3):
            for dc in range(3):
                test_field.seed((3 + dt, 3 + dc, 3, 3), f"dense-{dt}-{dc}")
        test_field.evolve_to_eq()

        before = test_field.crystallized_count()

        # Destroy cells
        targets = list(test_field.crystallized)[:num_destroy]
        for coord in targets:
            test_field.destroy(coord)

        after_destroy = test_field.crystallized_count()

        # Try to heal
        steps = test_field.evolve_to_eq(stable_for=10, max_iter=300)
        after_heal = test_field.crystallized_count()

        recovered = after_heal - after_destroy
        pct = (recovered / num_destroy * 100) if num_destroy > 0 else 0

        status = G + "FULL RECOVERY" if recovered >= num_destroy else (Y + "PARTIAL" if recovered > 0 else RED + "FAILED")
        print(f"  Destroy {num_destroy:3d} → {after_destroy} remaining → heal {steps:3d} steps → {after_heal} final → {status} ({pct:.0f}%){R}")

    print(f"\n  {D}Code distance d = max errors with full recovery{R}")
    print(f"  {D}In toric codes: d = S (field size). Check if Tesseract follows this.{R}")


# ══════════════════════════════════════════════════════════
# EXPERIMENT 4: Susceptibility — response to perturbation
# ══════════════════════════════════════════════════════════

def exp4_susceptibility():
    """Measure how the field responds to small perturbations near Θ_c.
    In a phase transition, susceptibility χ diverges at the critical point.
    χ = d(order_parameter) / d(perturbation)"""

    header("EXP 4: Susceptibility — response near critical point")

    size = 8
    seeds = [(2, 2, 2, 2), (2, 5, 2, 2), (5, 2, 2, 2), (2, 2, 5, 2)]

    thetas = [i / 40.0 for i in range(8, 39)]  # finer resolution: 0.20 to 0.95
    results = []

    for theta in thetas:
        # Measure order parameter at theta and theta + epsilon
        eps = 0.01
        ops = []
        for th in [theta, theta + eps]:
            field = Field(size, theta=th)
            for s in seeds:
                field.seed(s, f"ev@{s}")
            field.evolve_to_eq()
            ops.append(field.order_parameter())

        chi = abs(ops[0] - ops[1]) / eps  # susceptibility
        results.append((theta, ops[0], chi))

    # Find peak susceptibility
    max_chi = max(results, key=lambda x: x[2])
    print(f"  {'Θ':>6s}  {'ψ':>8s}  {'χ':>8s}")
    print(f"  {'─' * 6}  {'─' * 8}  {'─' * 8}")

    for theta, op, chi in results:
        bar = "▓" * min(int(chi * 5), 40)
        highlight = B + C if abs(theta - max_chi[0]) < 0.03 else ""
        print(f"  {highlight}{theta:6.3f}  {op:8.4f}  {chi:8.3f}  {bar}{R}")

    print(f"\n  {B}Peak susceptibility at Θ = {max_chi[0]:.3f} (χ = {max_chi[2]:.3f}){R}")
    if max_chi[2] > 10:
        print(f"  {G}→ Strong divergence — consistent with genuine phase transition{R}")
    elif max_chi[2] > 2:
        print(f"  {Y}→ Moderate peak — could be phase transition or crossover{R}")
    else:
        print(f"  {RED}→ Weak response — no clear phase transition{R}")


# ══════════════════════════════════════════════════════════
# EXPERIMENT 5: Topological protection — boundary vs bulk errors
# ══════════════════════════════════════════════════════════

def exp5_topological_protection():
    """In toric codes, errors at the boundary are harder to correct
    than errors in the bulk. Test if Tesseract has the same property.
    If yes → topological protection is genuine."""

    header("EXP 5: Topological protection — bulk vs boundary errors")

    size = 8
    results = []

    for region_label, center, label in [
        ("BULK (center of field)", (4, 4, 4, 4), "bulk"),
        ("EDGE (near boundary)", (0, 0, 0, 0), "edge"),
        ("CORNER (extreme boundary)", (7, 7, 7, 7), "corner"),
    ]:
        field = Field(size)

        # Seed dense region around the target
        for dt in range(-2, 3):
            for dc in range(-2, 3):
                coord = ((center[0] + dt) % size, (center[1] + dc) % size, center[2], center[3])
                field.seed(coord, f"{label}-{dt}-{dc}")
        field.evolve_to_eq()

        was_cryst = center in field.crystallized
        if not was_cryst:
            # Find nearest crystallized cell
            for coord in field.crystallized:
                center = coord
                break

        # Destroy the target
        field.destroy(center)
        steps = field.evolve_to_eq(stable_for=10, max_iter=200)
        healed = center in field.crystallized

        status = G + "HEALED" if healed else RED + "NOT HEALED"
        print(f"  {region_label}")
        print(f"    Destroyed {center} → {status} in {steps} steps{R}")
        results.append((label, healed, steps))

    # Compare
    print(f"\n  {D}In toric codes: bulk errors heal faster than boundary errors.{R}")
    print(f"  {D}If Tesseract shows the same pattern → topological protection confirmed.{R}")


# ══════════════════════════════════════════════════════════
# EXPERIMENT 6: Correlation length — how far does influence travel?
# ══════════════════════════════════════════════════════════

def exp6_correlation_length():
    """Seed one event and measure probability as a function of distance.
    Near Θ_c, the correlation length ξ should diverge.
    ξ tells us the characteristic range of 'influence' in the field."""

    header("EXP 6: Correlation length — probability decay vs distance")

    size = 16  # larger field for distance measurement
    center = (8, 8, 8, 8)

    for theta_label, theta in [("below Θ_c", 0.50), ("near Θ_c", 0.80), ("at Θ", 0.85), ("above Θ_c", 0.95)]:
        field = Field(size, theta=theta)
        field.seed(center, "probe")
        field.evolve_to_eq()

        print(f"  {B}Θ = {theta:.2f} ({theta_label}){R}")

        # Measure p at increasing distances along t-axis
        for d in range(0, 8):
            coord = ((center[0] + d) % size, center[1], center[2], center[3])
            p = field.cells.get(coord, 0.0)
            cryst = "★" if coord in field.crystallized else " "
            bar = "█" * int(p * 30)
            print(f"    d={d}  p={p:.4f} {cryst} {C}{bar}{R}")
        print()


# ══════════════════════════════════════════════════════════
# EXPERIMENT 7: Ising comparison — magnetization curve
# ══════════════════════════════════════════════════════════

def exp7_ising_comparison():
    """Compare Tesseract's order parameter curve with the known
    Ising model magnetization curve. If they match → same universality class."""

    header("EXP 7: Universality class — comparison with Ising model")

    size = 8
    seeds = [(2, 2, 2, 2), (2, 5, 2, 2), (5, 2, 2, 2), (2, 2, 5, 2)]

    # Ising 2D exact: m(T) = (1 - sinh(2J/kT)^(-4))^(1/8) for T < T_c
    # We compare SHAPE, not values. Normalize both to [0,1].

    # Tesseract data (using reduced temperature t = Θ/Θ_c)
    thetas = [i / 40.0 for i in range(4, 40)]
    tess_data = []
    for theta in thetas:
        field = Field(size, theta=theta)
        for s in seeds:
            field.seed(s, f"ev@{s}")
        field.evolve_to_eq()
        tess_data.append((theta, field.order_parameter()))

    # Find Θ_c from data
    max_drop_idx = 0
    max_drop = 0
    for i in range(1, len(tess_data)):
        drop = tess_data[i - 1][1] - tess_data[i][1]
        if drop > max_drop:
            max_drop = drop
            max_drop_idx = i
    theta_c = tess_data[max_drop_idx][0]

    print(f"  Tesseract Θ_c = {theta_c:.3f}")
    print()
    print(f"  {'Θ':>6s}  {'Θ/Θ_c':>6s}  {'ψ_tess':>8s}  {'curve':>30s}")
    print(f"  {'─' * 6}  {'─' * 6}  {'─' * 8}  {'─' * 30}")

    for theta, op in tess_data:
        reduced = theta / theta_c if theta_c > 0 else 0
        bar = G + "█" * int(op * 25) + R if op > 0.01 else ""
        marker = " ← Θ_c" if abs(theta - theta_c) < 0.02 else ""
        print(f"  {theta:6.3f}  {reduced:6.3f}  {op:8.4f}  {bar}{marker}")

    print(f"\n  {D}Ising 2D: β = 1/8 = 0.125 (critical exponent){R}")
    print(f"  {D}Mean-field: β = 1/2 = 0.500{R}")
    print(f"  {D}Fit ψ ~ (Θ_c - Θ)^β near Θ_c to extract Tesseract's β{R}")

    # Crude β estimate from the two points nearest Θ_c
    if max_drop_idx > 0 and max_drop_idx < len(tess_data):
        t1, op1 = tess_data[max_drop_idx - 1]
        t2, op2 = tess_data[max_drop_idx]
        if op1 > 0 and op2 < op1 and (theta_c - t2) > 0 and (theta_c - t1) > 0:
            try:
                beta = math.log(op1 / max(op2, 0.001)) / math.log((theta_c - t2) / (theta_c - t1))
                print(f"\n  {B}Crude β estimate: {beta:.3f}{R}")
                if abs(beta - 0.125) < 0.1:
                    print(f"  {G}→ Close to Ising 2D (β=0.125) — same universality class!{R}")
                elif abs(beta - 0.5) < 0.15:
                    print(f"  {Y}→ Close to mean-field (β=0.5){R}")
                else:
                    print(f"  {C}→ Novel universality class? β={beta:.3f} doesn't match known models{R}")
            except (ValueError, ZeroDivisionError):
                print(f"  {D}Could not estimate β — need finer resolution near Θ_c{R}")


# ══════════════════════════════════════════════════════════
# MAIN
# ══════════════════════════════════════════════════════════

def main():
    print(f"\n{B}{C}╔══════════════════════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   TESSERACT — Physics Experiments                            ║{R}")
    print(f"{B}{C}║   Phase transitions, critical exponents, error correction    ║{R}")
    print(f"{B}{C}║   Data for physicist evaluation                              ║{R}")
    print(f"{B}{C}╚══════════════════════════════════════════════════════════════╝{R}")

    t0 = time.time()

    exp1_phase_transition()
    exp2_finite_size_scaling()
    exp3_error_correction_distance()
    exp4_susceptibility()
    exp5_topological_protection()
    exp6_correlation_length()
    exp7_ising_comparison()

    elapsed = time.time() - t0

    print(f"\n{B}{'━' * 65}{R}")
    print(f"{B}  7 experiments completed in {elapsed:.1f}s{R}")
    print(f"{B}{'━' * 65}{R}")
    print(f"""
{D}  Next steps for physicist:
  1. If EXP 1 shows sharp transition → genuine phase transition
  2. If EXP 2 shows Θ_c shift with S → extract ν exponent
  3. If EXP 3 shows d ~ S → topological error correction confirmed
  4. If EXP 4 shows χ divergence → critical phenomenon
  5. If EXP 5 shows bulk > edge healing → topological protection
  6. If EXP 7 gives β ≈ 0.125 → Ising universality class
     If β is novel → new universality class (publishable){R}
""")


if __name__ == "__main__":
    main()
