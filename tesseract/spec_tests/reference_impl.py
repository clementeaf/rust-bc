"""
Tesseract reference implementation (Python).

Minimal implementation of the core protocol from TESSERACT_PROTOCOL.md.
Used for cross-validation against the Rust implementation.
No optimizations — clarity over performance.
"""

import json
import math
import sys
from collections import defaultdict
from typing import Optional

# --- Constants (must match Rust) ---

CRYSTALLIZATION_THRESHOLD = 0.85
INFLUENCE_FACTOR = 0.15
SEED_RADIUS = 3
CASCADE_STRENGTH = 0.08
EPSILON = 0.05
MIN_CAUSAL_DEPTH = 3
CORRELATION_THRESHOLD = 0.5
ZERO_COST_DISCOUNT = 0.25

DIMENSIONS = ["Temporal", "Context", "Origin", "Verification"]


# --- Core types ---

class Coord:
    __slots__ = ("t", "c", "o", "v")

    def __init__(self, t: int, c: int, o: int, v: int):
        self.t, self.c, self.o, self.v = t, c, o, v

    def __eq__(self, other):
        return (self.t, self.c, self.o, self.v) == (other.t, other.c, other.o, other.v)

    def __hash__(self):
        return hash((self.t, self.c, self.o, self.v))

    def __repr__(self):
        return f"({self.t},{self.c},{self.o},{self.v})"


class Attestation:
    def __init__(self, dimension: str, validator_id: str, event_id: str, weight: float):
        self.dimension = dimension
        self.validator_id = validator_id
        self.event_id = event_id
        self.weight = weight


class Cell:
    def __init__(self):
        self.probability = 0.0
        self.crystallized = False
        self.attestations: dict[str, list[Attestation]] = defaultdict(list)

    def attested_dimensions(self) -> int:
        return sum(1 for atts in self.attestations.values() if atts)

    def sigma_independence(self) -> int:
        """Count dimensions with at least one exclusive validator."""
        # Map each validator to set of dimensions it appears on
        validator_dims: dict[str, set] = defaultdict(set)
        for dim, atts in self.attestations.items():
            for att in atts:
                validator_dims[att.validator_id].add(dim)

        independent = 0
        for dim in DIMENSIONS:
            atts = self.attestations.get(dim, [])
            has_exclusive = any(
                len(validator_dims.get(att.validator_id, set())) == 1
                for att in atts
            )
            if has_exclusive:
                independent += 1
        return independent


class Field:
    def __init__(self, size: int):
        self.size = size
        self.cells: dict[Coord, Cell] = {}

    def get(self, coord: Coord) -> Cell:
        if coord not in self.cells:
            return Cell()  # empty cell (not stored)
        return self.cells[coord]

    def get_mut(self, coord: Coord) -> Cell:
        if coord not in self.cells:
            self.cells[coord] = Cell()
        return self.cells[coord]

    def active_cells(self) -> int:
        return len(self.cells)

    def crystallized_count(self) -> int:
        return sum(1 for c in self.cells.values() if c.crystallized)


# --- Distance ---

def wrapping_dist(a: int, b: int, size: int) -> int:
    a, b = a % size, b % size
    d = abs(a - b)
    return min(d, size - d)


def distance(a: Coord, b: Coord, size: int) -> float:
    dt = wrapping_dist(a.t, b.t, size)
    dc = wrapping_dist(a.c, b.c, size)
    do = wrapping_dist(a.o, b.o, size)
    dv = wrapping_dist(a.v, b.v, size)
    return math.sqrt(dt * dt + dc * dc + do * do + dv * dv)


# --- Attestation seeding ---

def attest(field: Field, center: Coord, event_id: str, dimension: str, validator_id: str):
    """Seed an attestation into the field (matches Rust attest())."""
    s = field.size
    axis_max = min(SEED_RADIUS, s // 2)

    for dt in range(-axis_max, axis_max + 1):
        t = (center.t + dt) % s
        for dc in range(-axis_max, axis_max + 1):
            c = (center.c + dc) % s
            for do_ in range(-axis_max, axis_max + 1):
                o = (center.o + do_) % s
                for dv in range(-axis_max, axis_max + 1):
                    v = (center.v + dv) % s
                    coord = Coord(t, c, o, v)

                    dist = distance(center, coord, s)
                    p = 1.0 / (1.0 + dist)

                    if p < EPSILON:
                        continue

                    cell = field.get_mut(coord)
                    cell.probability = min(cell.probability + p, 1.0)

                    # Record attestation (with dedup)
                    if p >= EPSILON:
                        atts = cell.attestations[dimension]
                        already = any(
                            a.validator_id == validator_id and a.event_id == event_id
                            for a in atts
                        )
                        if not already:
                            atts.append(Attestation(dimension, validator_id, event_id, p))

                    # Crystallization check
                    if (not cell.crystallized
                            and cell.probability >= CRYSTALLIZATION_THRESHOLD
                            and cell.attestations  # has attestations
                            and cell.sigma_independence() >= 4):
                        cell.crystallized = True
                        cell.probability = 1.0


# --- Sigma_eff ---

def compute_sigma_eff(cell: Cell, causal_graph=None) -> float:
    """Compute effective sigma (matches Rust adversarial::effective_sigma)."""
    sigma_eff = 0.0

    # Build validator -> dims map
    validator_dims: dict[str, set] = defaultdict(set)
    for dim, atts in cell.attestations.items():
        for att in atts:
            validator_dims[att.validator_id].add(dim)

    for dim in DIMENSIONS:
        atts = cell.attestations.get(dim, [])
        if not atts:
            continue

        # Independence
        has_exclusive = any(
            len(validator_dims.get(att.validator_id, set())) == 1
            for att in atts
        )
        independence = 1.0 if has_exclusive else 0.0

        # Diversity (simplified without graph: assume independent)
        diversity = 1.0

        # Cost
        cost = ZERO_COST_DISCOUNT if causal_graph is None else 1.0

        sigma_eff += min(1.0, independence * diversity * cost)

    return sigma_eff


# --- Test runner ---

def run_test(test: dict) -> dict:
    """Execute a single test vector and return results."""
    inp = test["input"]
    field_size = inp["field_size"]
    field = Field(field_size)
    results = {}

    # Apply attestations
    if "attestations" in inp:
        for att in inp["attestations"]:
            coord = Coord(*att["coord"])
            attest(field, coord, att["event_id"], att["dimension"], att["validator_id"])

    # Distance tests
    if "coord_a" in inp and "coord_b" in inp:
        a = Coord(*inp["coord_a"])
        b = Coord(*inp["coord_b"])
        results["distance"] = distance(a, b, field_size)

    # Center checks
    if "attestations" in inp and inp["attestations"]:
        center_coord = Coord(*inp["attestations"][0]["coord"])
        center_cell = field.get(center_coord)
        results["sigma_at_center"] = center_cell.sigma_independence()
        results["crystallized_at_center"] = center_cell.crystallized
        results["probability_at_center"] = center_cell.probability

        # sigma_eff
        causal_graph = inp.get("causal_graph")
        results["raw_sigma"] = center_cell.sigma_independence()
        results["sigma_eff"] = compute_sigma_eff(center_cell, causal_graph)

    results["active_cells"] = field.active_cells()
    results["crystallized_count"] = field.crystallized_count()

    return results


def check_expected(test_id: str, results: dict, expected: dict) -> list:
    """Compare results against expected values. Returns list of failures."""
    failures = []
    for key, exp_val in expected.items():
        if key.endswith("_gt"):
            actual_key = key[:-3]
            actual = results.get(actual_key)
            if actual is not None and actual <= exp_val:
                failures.append(f"  {key}: expected > {exp_val}, got {actual}")
        elif key.endswith("_lte"):
            actual_key = key[:-4]
            actual = results.get(actual_key)
            if actual is not None and actual > exp_val:
                failures.append(f"  {key}: expected <= {exp_val}, got {actual}")
        elif key.endswith("_tolerance"):
            continue  # used by companion key
        else:
            actual = results.get(key)
            if actual is None:
                continue
            tolerance = expected.get(f"{key}_tolerance", 0)
            if isinstance(exp_val, float):
                if abs(actual - exp_val) > tolerance + 1e-10:
                    failures.append(f"  {key}: expected {exp_val} (+/-{tolerance}), got {actual}")
            elif isinstance(exp_val, bool):
                if actual != exp_val:
                    failures.append(f"  {key}: expected {exp_val}, got {actual}")
            elif isinstance(exp_val, int):
                if actual != exp_val:
                    failures.append(f"  {key}: expected {exp_val}, got {actual}")
    return failures


def main():
    vectors_path = sys.argv[1] if len(sys.argv) > 1 else "test_vectors.json"
    with open(vectors_path) as f:
        data = json.load(f)

    print(f"Tesseract Reference Implementation — Python")
    print(f"Test vectors version: {data['version']}")
    print(f"Running {len(data['tests'])} tests...\n")

    passed = 0
    failed = 0
    all_results = []

    for test in data["tests"]:
        results = run_test(test)
        failures = check_expected(test["id"], results, test["expected"])
        all_results.append({"id": test["id"], "results": results, "pass": len(failures) == 0})

        if failures:
            print(f"FAIL {test['id']}: {test['name']}")
            for f in failures:
                print(f)
            failed += 1
        else:
            print(f"  OK {test['id']}: {test['name']}")
            passed += 1

    print(f"\n{passed} passed, {failed} failed, {passed + failed} total")

    # Export results for cross-validation
    with open("python_results.json", "w") as f:
        json.dump(all_results, f, indent=2, default=str)
    print(f"Results exported to python_results.json")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
