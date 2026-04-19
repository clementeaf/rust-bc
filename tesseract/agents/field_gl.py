#!/usr/bin/env python3
"""
Tesseract Field — Ginzburg-Landau formulation.

Evolution equation:
  p_new(x) = p_old(x) + dt × [
      J(σ) × Laplacian(p)     ← neighbor coupling (diffusion)
    + r × p × (1-p) × (p-Θ)   ← double-well potential
    + R(σ)                     ← resonance (source-diversity driven)
  ]

Where:
  - J(σ): coupling constant, depends on source-diverse orthogonal support
  - Laplacian(p) = Σ_neighbors(p_n) - 2D×p  (discrete Laplacian on 4D lattice)
  - r: reaction strength (controls barrier height between phases)
  - Θ: crystallization threshold (analogous to critical temperature)
  - R(σ): resonance boost for diverse multi-source support

Phase structure:
  - p ≈ 0: disordered phase (no crystallization)
  - p ≈ 1: ordered phase (crystallized)
  - Θ: controls the barrier between phases
  - J: controls spatial correlation length
  - At critical (J_c, Θ_c): correlation length diverges → phase transition

Seeds are SMALL perturbations, not the dominant force.
The field dynamics create order — seeds just nucleate it.
"""
import math
import time
from collections import defaultdict


class FieldGL:
    """4D probability field with Ginzburg-Landau dynamics."""

    def __init__(self, size: int, theta: float = 0.5, J: float = 0.3,
                 r: float = 2.0, dt: float = 0.05, seed_strength: float = 0.3):
        self.size = size
        self.theta = theta          # barrier position (like T_c in Ising)
        self.J = J                  # coupling constant
        self.r = r                  # reaction strength
        self.dt = dt                # time step
        self.seed_strength = seed_strength  # how strong seeds are (small!)
        self.cells: dict[tuple, float] = {}
        self.crystallized: set[tuple] = set()
        self.sources: dict[tuple, set[str]] = {}
        self.EPSILON = 0.02
        self.CRYST_THRESHOLD = 0.90  # crystallization at high probability
        self.SEED_RADIUS = 3        # smaller radius — seeds are perturbations
        self.RES = {0: 0.0, 1: 0.0, 2: 0.01, 3: 0.03, 4: 0.06}

    def _wrap(self, a: int) -> int:
        return a % self.size

    def _dist(self, a: tuple, b: tuple) -> float:
        s = self.size
        return math.sqrt(sum(
            min(abs(a[i] - b[i]), s - abs(a[i] - b[i])) ** 2
            for i in range(4)
        ))

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
        """Source-aware: count axes with diverse event sources."""
        my_sources = self.sources.get(coord, set())
        if not my_sources:
            return 0
        s = self.size
        axes = 0
        for axis in range(4):
            for delta in (-1, 1):
                n = list(coord)
                n[axis] = (n[axis] + delta) % s
                n_sources = self.sources.get(tuple(n), set())
                if n_sources - my_sources:  # neighbor has sources I don't
                    axes += 1
                    break
        return axes

    def seed(self, center: tuple, event_id: str = ""):
        """Seed as a SMALL perturbation — not the dominant force."""
        s = self.size
        r = self.SEED_RADIUS
        for dt in range(-r, r + 1):
            for dc in range(-r, r + 1):
                for do_ in range(-r, r + 1):
                    for dv in range(-r, r + 1):
                        coord = (
                            (center[0] + dt) % s, (center[1] + dc) % s,
                            (center[2] + do_) % s, (center[3] + dv) % s,
                        )
                        d = self._dist(center, coord)
                        # Seed is WEAK — perturbation, not domination
                        p_add = self.seed_strength / (1.0 + d * d)
                        if p_add < self.EPSILON:
                            continue
                        old = self.cells.get(coord, 0.0)
                        self.cells[coord] = min(old + p_add, 1.0)
                        if event_id:
                            self.sources.setdefault(coord, set()).add(event_id)

    def evolve(self) -> int:
        """One GL evolution step. Returns number of new crystallizations."""
        # Collect all coords to process
        to_process = set()
        for coord in list(self.cells.keys()):
            to_process.add(coord)
            for n in self.neighbors(coord):
                to_process.add(n)

        # Calculate updates using GL equation
        updates = []
        for coord in to_process:
            if coord in self.crystallized:
                continue

            p = self.cells.get(coord, 0.0)
            nbrs = self.neighbors(coord)

            # Discrete Laplacian: Σ(p_neighbor) - 2D×p
            # In 4D: 8 neighbors, so Laplacian = Σp_n - 8p
            lap = sum(self.cells.get(n, 0.0) for n in nbrs) - 8.0 * p

            # Source-aware coupling
            sigma = self.orthogonal_support(coord)
            j_eff = self.J * (1.0 + 0.5 * sigma)  # coupling grows with diversity

            # GL evolution: diffusion + reaction + resonance
            diffusion = j_eff * lap
            reaction = self.r * p * (1.0 - p) * (p - self.theta)
            resonance = self.RES.get(sigma, 0.0)

            dp = self.dt * (diffusion + reaction + resonance)
            new_p = max(0.0, min(1.0, p + dp))

            updates.append((coord, new_p))

        # Apply updates
        new_cryst = 0
        for coord, new_p in updates:
            if new_p < self.EPSILON:
                if coord in self.cells and not self.sources.get(coord):
                    del self.cells[coord]
                continue

            self.cells[coord] = new_p
            if coord not in self.crystallized and new_p >= self.CRYST_THRESHOLD:
                self.crystallized.add(coord)
                self.cells[coord] = 1.0
                new_cryst += 1

        return new_cryst

    def evolve_to_eq(self, stable_for: int = 10, max_iter: int = 500) -> int:
        stable = 0
        for i in range(max_iter):
            nc = self.evolve()
            # Check if field is changing
            if nc == 0:
                stable += 1
            else:
                stable = 0
            if stable >= stable_for:
                return i + 1
        return max_iter

    def destroy(self, coord: tuple):
        if coord in self.cells:
            self.cells[coord] = 0.0
            self.crystallized.discard(coord)

    def order_param(self) -> float:
        """Global order parameter: mean probability of active cells."""
        if not self.cells:
            return 0.0
        return sum(self.cells.values()) / len(self.cells)

    def cryst_fraction(self) -> float:
        """Fraction of total field that is crystallized."""
        return len(self.crystallized) / (self.size ** 4)

    def magnetization(self) -> float:
        """Ising-like magnetization: <2p - 1> mapped to [-1, 1]."""
        if not self.cells:
            return -1.0  # empty = disordered
        m = sum(2.0 * p - 1.0 for p in self.cells.values()) / (self.size ** 4)
        return m

    def correlation(self, origin: tuple, max_dist: int = 10) -> list[tuple[int, float]]:
        """Measure correlation function C(r) along t-axis from origin."""
        p0 = self.cells.get(origin, 0.0)
        mean_p = self.order_param()
        result = []
        for d in range(max_dist + 1):
            coord = ((origin[0] + d) % self.size, origin[1], origin[2], origin[3])
            p_r = self.cells.get(coord, 0.0)
            c_r = (p0 - mean_p) * (p_r - mean_p)
            result.append((d, c_r))
        return result

    def correlation_length(self, origin: tuple) -> float:
        """Estimate ξ from exponential fit of C(r)."""
        corr = self.correlation(origin, max_dist=min(self.size // 2, 12))
        if len(corr) < 3 or corr[0][1] <= 0:
            return 0.0
        c0 = corr[0][1]
        for d, c_r in corr[1:]:
            if c_r <= 0 or c_r < c0 * 0.01:
                return max(0, d - 1)
        return corr[-1][0]


# ── Colors ───────────────────────────────────────────────
R = "\033[0m"; B = "\033[1m"; C = "\033[36m"; Y = "\033[33m"
G = "\033[32m"; RED = "\033[31m"; D = "\033[2m"

def header(name):
    print(f"\n{B}{Y}{'━' * 65}{R}")
    print(f"{B}{Y}  {name}{R}")
    print(f"{B}{Y}{'━' * 65}{R}\n")


# ══════════════════════════════════════════════════════════
# EXP A: Phase diagram (J, Θ)
# ══════════════════════════════════════════════════════════

def exp_a_phase_diagram():
    header("EXP A: Phase diagram — order parameter vs Θ for different J")

    size = 12
    seeds = [
        ((3, 6, 6, 6), "ev-A"),
        ((9, 6, 6, 6), "ev-B"),
        ((6, 3, 6, 6), "ev-C"),
        ((6, 9, 6, 6), "ev-D"),
    ]

    for J in [0.1, 0.2, 0.3, 0.5, 0.8]:
        print(f"  {B}J = {J:.1f}{R}")
        thetas = [i / 20.0 for i in range(1, 20)]
        prev_m = None
        max_drop = 0
        theta_c = 0

        for theta in thetas:
            f = FieldGL(size, theta=theta, J=J, r=2.0, seed_strength=0.25)
            for pos, eid in seeds:
                f.seed(pos, eid)
            f.evolve_to_eq(stable_for=15, max_iter=300)

            m = f.order_param()
            nc = len(f.crystallized)

            if prev_m is not None:
                drop = prev_m - m
                if drop > max_drop:
                    max_drop = drop
                    theta_c = theta

            bar = G + "█" * int(m * 30) + R if m > 0.01 else D + "·" + R
            print(f"    Θ={theta:.2f}  ψ={m:.4f}  cryst={nc:5d}  {bar}")
            prev_m = m

        sharpness = "SHARP" if max_drop > 0.1 else ("moderate" if max_drop > 0.03 else "none")
        color = G if max_drop > 0.1 else (Y if max_drop > 0.03 else RED)
        print(f"    {color}Θ_c ≈ {theta_c:.2f}  Δψ = {max_drop:.4f}  ({sharpness}){R}\n")


# ══════════════════════════════════════════════════════════
# EXP B: Finite-size scaling
# ══════════════════════════════════════════════════════════

def exp_b_finite_size():
    header("EXP B: Finite-size scaling — Θ_c vs system size S")

    J = 0.3
    sizes = [8, 10, 12]

    print(f"  J = {J}\n")

    for size in sizes:
        seeds = [
            ((size // 4, size // 2, size // 2, size // 2), "ev-A"),
            ((3 * size // 4, size // 2, size // 2, size // 2), "ev-B"),
            ((size // 2, size // 4, size // 2, size // 2), "ev-C"),
            ((size // 2, 3 * size // 4, size // 2, size // 2), "ev-D"),
        ]

        thetas = [i / 40.0 for i in range(4, 38)]
        prev_m = None
        max_drop = 0
        theta_c = 0

        for theta in thetas:
            f = FieldGL(size, theta=theta, J=J, r=2.0, seed_strength=0.25)
            for pos, eid in seeds:
                f.seed(pos, eid)
            f.evolve_to_eq(stable_for=15, max_iter=300)
            m = f.order_param()

            if prev_m is not None:
                drop = prev_m - m
                if drop > max_drop:
                    max_drop = drop
                    theta_c = theta
            prev_m = m

        print(f"  S={size:2d} ({size**4:6d} cells)  Θ_c ≈ {theta_c:.3f}  Δψ = {max_drop:.4f}")

    print(f"\n  {D}If Θ_c shifts with S → fit Θ_c(S) = Θ_c(∞) + a·S^(-1/ν) to get ν{R}")


# ══════════════════════════════════════════════════════════
# EXP C: Critical exponent β
# ══════════════════════════════════════════════════════════

def exp_c_critical_exponent():
    header("EXP C: Critical exponent β — log(ψ) vs log(Θ_c - Θ)")

    size = 12
    J = 0.3
    seeds = [
        ((3, 6, 6, 6), "ev-A"),
        ((9, 6, 6, 6), "ev-B"),
        ((6, 3, 6, 6), "ev-C"),
        ((6, 9, 6, 6), "ev-D"),
    ]

    # First find Θ_c
    thetas = [i / 40.0 for i in range(4, 38)]
    data = []
    prev_m = None
    max_drop = 0
    theta_c = 0.5  # default

    for theta in thetas:
        f = FieldGL(size, theta=theta, J=J, r=2.0, seed_strength=0.25)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)
        m = f.order_param()
        data.append((theta, m))

        if prev_m is not None:
            drop = prev_m - m
            if drop > max_drop:
                max_drop = drop
                theta_c = theta
        prev_m = m

    print(f"  Θ_c ≈ {theta_c:.3f}\n")

    # Log-log plot near Θ_c (below it)
    print(f"  {'Θ':>6s}  {'Θ_c-Θ':>8s}  {'ψ':>8s}  {'log(Θ_c-Θ)':>11s}  {'log(ψ)':>8s}")
    print(f"  {'─'*6}  {'─'*8}  {'─'*8}  {'─'*11}  {'─'*8}")

    log_x = []
    log_y = []

    for theta, m in data:
        if theta < theta_c and m > 0.01:
            diff = theta_c - theta
            if diff > 0.005:
                lx = math.log10(diff)
                ly = math.log10(m)
                log_x.append(lx)
                log_y.append(ly)
                print(f"  {theta:6.3f}  {diff:8.4f}  {m:8.4f}  {lx:11.4f}  {ly:8.4f}")

    # Linear fit: log(ψ) = β × log(Θ_c - Θ) + const
    if len(log_x) >= 3:
        n = len(log_x)
        sx = sum(log_x)
        sy = sum(log_y)
        sxy = sum(x * y for x, y in zip(log_x, log_y))
        sxx = sum(x * x for x in log_x)
        denom = n * sxx - sx * sx
        if abs(denom) > 1e-10:
            beta = (n * sxy - sx * sy) / denom
            print(f"\n  {B}β ≈ {beta:.3f}{R}")

            if abs(beta - 0.5) < 0.15:
                print(f"  {G}→ Mean-field (β=0.5) — expected for 4D!{R}")
                print(f"  {D}  (Upper critical dimension for Ising is d=4, so mean-field is correct){R}")
            elif abs(beta - 0.125) < 0.05:
                print(f"  {C}→ Ising 2D (β=0.125){R}")
            elif abs(beta - 0.326) < 0.05:
                print(f"  {C}→ Ising 3D (β=0.326){R}")
            elif 0.5 < beta < 1.0:
                print(f"  {Y}→ Novel exponent — investigate further{R}")
            else:
                print(f"  {Y}→ β={beta:.3f} — check fit quality{R}")


# ══════════════════════════════════════════════════════════
# EXP D: Correlation length divergence
# ══════════════════════════════════════════════════════════

def exp_d_correlation():
    header("EXP D: Correlation length ξ vs Θ (should diverge at Θ_c)")

    size = 16
    J = 0.3
    center = (8, 8, 8, 8)
    seeds = [
        ((4, 8, 8, 8), "ev-A"),
        ((12, 8, 8, 8), "ev-B"),
        ((8, 4, 8, 8), "ev-C"),
        ((8, 12, 8, 8), "ev-D"),
    ]

    print(f"  {'Θ':>6s}  {'ξ':>4s}  {'ψ':>8s}  {'correlation decay'}")
    print(f"  {'─'*6}  {'─'*4}  {'─'*8}  {'─'*40}")

    for theta in [0.15, 0.25, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60, 0.70, 0.80]:
        f = FieldGL(size, theta=theta, J=J, r=2.0, seed_strength=0.25)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        xi = f.correlation_length(center)
        m = f.order_param()

        # Show p(d) profile
        profile = []
        for d in range(8):
            coord = ((center[0] + d) % size, center[1], center[2], center[3])
            p = f.cells.get(coord, 0.0)
            profile.append(f"{p:.2f}")

        prof_str = " ".join(profile)
        bar = C + "█" * int(xi * 3) + R
        print(f"  {theta:6.2f}  {xi:4.0f}  {m:8.4f}  p(d): {prof_str}  {bar}")

    print(f"\n  {D}If ξ peaks near Θ_c → correlation length divergence (genuine phase transition){R}")


# ══════════════════════════════════════════════════════════
# EXP E: Self-healing in GL formulation
# ══════════════════════════════════════════════════════════

def exp_e_self_healing():
    header("EXP E: Self-healing — does GL formulation preserve it?")

    size = 12
    J = 0.3

    seeds = [
        ((3, 6, 6, 6), "ev-A"),
        ((9, 6, 6, 6), "ev-B"),
        ((6, 3, 6, 6), "ev-C"),
        ((6, 9, 6, 6), "ev-D"),
        ((6, 6, 3, 6), "ev-E"),
        ((6, 6, 9, 6), "ev-F"),
    ]

    # Use Θ below critical so field is in ordered phase
    for theta in [0.20, 0.35, 0.45]:
        f = FieldGL(size, theta=theta, J=J, r=2.0, seed_strength=0.25)
        for pos, eid in seeds:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        initial = len(f.crystallized)
        if initial == 0:
            print(f"  Θ={theta:.2f}: 0 crystallized — skip")
            continue

        # Destroy 10%
        damage_n = max(1, initial // 10)
        targets = list(f.crystallized)[:damage_n]
        for t in targets:
            f.destroy(t)

        after_d = len(f.crystallized)
        steps = f.evolve_to_eq(stable_for=15, max_iter=300)
        after_h = len(f.crystallized)
        recovered = after_h - after_d
        pct = (recovered / damage_n * 100) if damage_n > 0 else 0

        status = G + "FULL" if recovered >= damage_n else (Y + f"{pct:.0f}%" if recovered > 0 else RED + "NONE")
        print(f"  Θ={theta:.2f}: {initial} cryst → destroy {damage_n} → {status} recovery ({steps} steps){R}")


# ══════════════════════════════════════════════════════════
# EXP F: Dimension effect in GL
# ══════════════════════════════════════════════════════════

def exp_f_dimensions():
    header("EXP F: Dimension effect in GL — 2D vs 3D vs 4D")

    size = 12
    theta = 0.35
    J = 0.3
    center = (6, 6, 6, 6)

    for dims, label in [(2, "2D"), (3, "3D"), (4, "4D")]:
        f = FieldGL(size, theta=theta, J=J, r=2.0, seed_strength=0.25)

        # Seeds along different axes
        seed_list = [
            ((3, 6, 6, 6), "ax-t"),
            ((6, 3, 6, 6), "ax-c"),
        ]
        if dims >= 3:
            seed_list.append(((6, 6, 3, 6), "ax-o"))
        if dims >= 4:
            seed_list.append(((6, 6, 6, 3), "ax-v"))

        for pos, eid in seed_list:
            f.seed(pos, eid)
        f.evolve_to_eq(stable_for=15, max_iter=300)

        nc = len(f.crystallized)
        m = f.order_param()

        bar = G + "█" * min(nc // 10, 40) + R
        print(f"  {label}: cryst={nc:6d}  ψ={m:.4f}  {bar}")

    print(f"\n  {D}If 4D >> 3D >> 2D → GL dynamics amplify dimensional effect{R}")


# ══════════════════════════════════════════════════════════

def main():
    print(f"\n{B}{C}╔══════════════════════════════════════════════════════════════╗{R}")
    print(f"{B}{C}║   TESSERACT — Ginzburg-Landau Formulation                    ║{R}")
    print(f"{B}{C}║   Genuine phase transitions, critical exponents              ║{R}")
    print(f"{B}{C}╚══════════════════════════════════════════════════════════════╝{R}")

    t0 = time.time()

    exp_a_phase_diagram()
    exp_b_finite_size()
    exp_c_critical_exponent()
    exp_d_correlation()
    exp_e_self_healing()
    exp_f_dimensions()

    elapsed = time.time() - t0
    print(f"\n{B}{'━' * 65}{R}")
    print(f"{B}  6 experiments completed in {elapsed:.1f}s{R}")
    print(f"{B}{'━' * 65}{R}\n")


if __name__ == "__main__":
    main()
