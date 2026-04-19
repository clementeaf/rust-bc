#!/usr/bin/env python3
"""
Tesseract Field v2 — Fixed dynamics.

Changes from v1:
1. Source-aware orthogonal support: σ counts axes where neighbors
   have influences from DIFFERENT events than this cell.
   Single seed → σ=0 everywhere. Two orthogonal seeds → σ=1-2 at overlap.

2. Crystallization cascade: when a cell crystallizes, it pushes
   a small probability boost to its immediate neighbors. This creates
   dynamic correlation beyond the seed radius.

3. Same resonance table, but now it actually discriminates because
   σ reflects real diversity, not just proximity.
"""
import math
from typing import Optional


class FieldV2:
    def __init__(self, size: int, theta: float = 0.85, alpha: float = 0.15,
                 seed_radius: int = 4, cascade_strength: float = 0.08):
        self.size = size
        self.theta = theta
        self.alpha = alpha
        self.seed_radius = min(seed_radius, size // 2)
        self.cascade = cascade_strength
        self.cells: dict[tuple, float] = {}
        self.crystallized: set[tuple] = set()
        # Track source events per cell: coord → set of event_ids
        self.sources: dict[tuple, set[str]] = {}
        self.EPSILON = 0.05
        self.RES = {0: (1.0, 0.0), 1: (1.0, 0.0), 2: (1.5, 0.02),
                    3: (2.5, 0.05), 4: (4.0, 0.10)}

    def _dist(self, a, b):
        s = self.size
        return math.sqrt(sum(min(abs(a[i]-b[i]), s-abs(a[i]-b[i]))**2 for i in range(4)))

    def seed(self, center: tuple, event_id: str = ""):
        s, r = self.size, self.seed_radius
        for dt in range(-r, r+1):
            for dc in range(-r, r+1):
                for do_ in range(-r, r+1):
                    for dv in range(-r, r+1):
                        coord = ((center[0]+dt)%s, (center[1]+dc)%s,
                                 (center[2]+do_)%s, (center[3]+dv)%s)
                        d = self._dist(center, coord)
                        p = 1.0 / (1.0 + d)
                        if p < self.EPSILON:
                            continue
                        old = self.cells.get(coord, 0.0)
                        new = min(old + p, 1.0)
                        self.cells[coord] = new
                        if event_id:
                            self.sources.setdefault(coord, set()).add(event_id)
                        if coord not in self.crystallized and new >= self.theta:
                            self.crystallized.add(coord)
                            self.cells[coord] = 1.0
                            self._cascade_from(coord)

    def _cascade_from(self, coord):
        """When a cell crystallizes, push a small boost to neighbors."""
        for n in self.neighbors(coord):
            old = self.cells.get(n, 0.0)
            new = min(old + self.cascade, 1.0)
            if new >= self.EPSILON:
                self.cells[n] = new
                # Cascade inherits the parent's sources
                parent_sources = self.sources.get(coord, set())
                self.sources.setdefault(n, set()).update(parent_sources)
                if n not in self.crystallized and new >= self.theta:
                    self.crystallized.add(n)
                    self.cells[n] = 1.0

    def neighbors(self, coord):
        s = self.size
        result = []
        for axis in range(4):
            for delta in (-1, 1):
                n = list(coord)
                n[axis] = (n[axis] + delta) % s
                result.append(tuple(n))
        return result

    def orthogonal_support(self, coord):
        """Count axes where at least one neighbor has sources DIFFERENT
        from this cell's sources. Measures diversity, not proximity."""
        s = self.size
        my_sources = self.sources.get(coord, set())
        if not my_sources:
            # No sources at all — check basic neighbor presence (fallback)
            axes = 0
            for axis in range(4):
                for delta in (-1, 1):
                    n = list(coord)
                    n[axis] = (n[axis] + delta) % s
                    if self.cells.get(tuple(n), 0.0) > 0.5:
                        axes += 1
                        break
            return axes

        axes = 0
        for axis in range(4):
            for delta in (-1, 1):
                n = list(coord)
                n[axis] = (n[axis] + delta) % s
                neighbor_sources = self.sources.get(tuple(n), set())
                # Is there a source in the neighbor that's NOT in my sources?
                if neighbor_sources - my_sources:
                    axes += 1
                    break
        return axes

    def evolve(self):
        to_process = set()
        for coord in list(self.cells.keys()):
            to_process.add(coord)
            for n in self.neighbors(coord):
                to_process.add(n)

        updates = []
        for coord in to_process:
            if coord in self.crystallized:
                continue
            p = self.cells.get(coord, 0.0)
            avg = sum(self.cells.get(n, 0.0) for n in self.neighbors(coord)) / 8.0
            delta = (avg - p) * self.alpha
            sigma = self.orthogonal_support(coord)
            amp, res = self.RES.get(sigma, (1.0, 0.0))
            new_p = max(0.0, min(1.0, p + delta * amp + res))
            updates.append((coord, new_p))

        nc = 0
        for coord, new_p in updates:
            if new_p < self.EPSILON:
                if coord in self.cells and not self.sources.get(coord):
                    del self.cells[coord]
                continue
            self.cells[coord] = new_p
            if coord not in self.crystallized and new_p >= self.theta:
                self.crystallized.add(coord)
                self.cells[coord] = 1.0
                self._cascade_from(coord)
                nc += 1
        return nc

    def evolve_to_eq(self, stable_for=5, max_iter=200):
        stable = 0
        for i in range(max_iter):
            if self.evolve() == 0:
                stable += 1
            else:
                stable = 0
            if stable >= stable_for:
                return i + 1
        return max_iter

    def destroy(self, coord):
        if coord in self.cells:
            self.cells[coord] = 0.0
            self.crystallized.discard(coord)

    def order_param(self):
        total = self.size ** 4
        return len(self.crystallized) / total

    def active_count(self):
        return len(self.cells)

    def cryst_count(self):
        return len(self.crystallized)
