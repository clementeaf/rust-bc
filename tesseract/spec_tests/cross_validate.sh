#!/usr/bin/env bash
# Cross-validate Tesseract: Rust implementation vs Python reference implementation.
# Runs both, exports results JSON, compares key outputs.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$SCRIPT_DIR"

echo "=========================================="
echo " Tesseract Cross-Validation"
echo "=========================================="
echo ""

# --- Step 1: Run Python reference implementation ---
echo "[1/4] Running Python reference implementation..."
python3 reference_impl.py test_vectors.json
echo ""

# --- Step 2: Run Rust implementation ---
echo "[2/4] Running Rust implementation..."
cd "$PROJECT_DIR"
cargo test --test spec_vectors -- --nocapture 2>&1 | grep -E "OK|FAIL|passed|failed|exported"
cd "$SCRIPT_DIR"
echo ""

# --- Step 3: Compare results ---
echo "[3/4] Comparing results..."

if [ ! -f python_results.json ]; then
    echo "ERROR: python_results.json not found"
    exit 1
fi

if [ ! -f rust_results.json ]; then
    echo "ERROR: rust_results.json not found"
    exit 1
fi

# Compare using Python (both JSON files available)
python3 - <<'COMPARE_SCRIPT'
import json
import sys

with open("python_results.json") as f:
    py_results = {r["id"]: r["results"] for r in json.load(f)}

with open("rust_results.json") as f:
    rust_results = {r["id"]: r["results"] for r in json.load(f)}

# Keys to compare (skip implementation-specific details)
COMPARE_KEYS = [
    "sigma_at_center",
    "crystallized_at_center",
    "crystallized_count",
    "distance",
    "raw_sigma",
]

FLOAT_KEYS = ["probability_at_center", "sigma_eff", "distance"]
FLOAT_TOLERANCE = 0.01

mismatches = 0
compared = 0
for test_id in py_results:
    if test_id not in rust_results:
        print(f"  MISSING in Rust: {test_id}")
        mismatches += 1
        continue

    py = py_results[test_id]
    rs = rust_results[test_id]

    for key in COMPARE_KEYS:
        if key not in py or key not in rs:
            continue
        compared += 1

        py_val = py[key]
        rs_val = rs[key]

        if key in FLOAT_KEYS:
            if abs(float(py_val) - float(rs_val)) > FLOAT_TOLERANCE:
                print(f"  MISMATCH {test_id}.{key}: python={py_val}, rust={rs_val}")
                mismatches += 1
        else:
            if py_val != rs_val:
                print(f"  MISMATCH {test_id}.{key}: python={py_val}, rust={rs_val}")
                mismatches += 1

    # Compare float keys with tolerance
    for key in FLOAT_KEYS:
        if key not in py or key not in rs:
            continue
        if key in COMPARE_KEYS:
            continue  # already compared
        compared += 1
        if abs(float(py[key]) - float(rs[key])) > FLOAT_TOLERANCE:
            print(f"  MISMATCH {test_id}.{key}: python={py[key]}, rust={rs[key]}")
            mismatches += 1

if mismatches == 0:
    print(f"  ALL MATCH: {compared} values compared across {len(py_results)} tests")
else:
    print(f"  {mismatches} MISMATCHES in {compared} comparisons")
    sys.exit(1)
COMPARE_SCRIPT

echo ""

# --- Step 4: Summary ---
echo "[4/4] Summary"
echo "  Python results: python_results.json"
echo "  Rust results:   rust_results.json"
echo "  Test vectors:   test_vectors.json"
echo ""
echo "Cross-validation PASSED"
