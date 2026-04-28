#!/usr/bin/env bash
# CI Runtime Budget Report
# Runs the PR-critical test gates and reports timing.

set -euo pipefail

echo "=== CI Runtime Budget Report ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

TOTAL_START=$(date +%s)

run_timed() {
    local name="$1"
    shift
    local start=$(date +%s)
    echo -n "  $name ... "
    if "$@" > /dev/null 2>&1; then
        local end=$(date +%s)
        local elapsed=$((end - start))
        echo "${elapsed}s ✓"
    else
        local end=$(date +%s)
        local elapsed=$((end - start))
        echo "${elapsed}s ✗ FAILED"
        return 1
    fi
}

echo "PR Gates:"
run_timed "fmt check" cargo fmt --all -- --check
run_timed "clippy" cargo clippy --workspace -- -D warnings
run_timed "crypto module" cargo test -p pqc_crypto_module -- --test-threads=1
run_timed "crypto boundary" cargo test --test crypto_boundary
run_timed "fips readiness" cargo test -p pqc_crypto_module --test fips_readiness -- --test-threads=1
run_timed "pqc security audit" cargo test --test pqc_security_audit
run_timed "property invariants" cargo test --test property_invariants
run_timed "perf guardrails" cargo test --test performance_guardrails

TOTAL_END=$(date +%s)
TOTAL=$((TOTAL_END - TOTAL_START))

echo ""
echo "=== Summary ==="
echo "Total PR gate time: ${TOTAL}s"
echo "Budget: 900s (15 min)"
if [ "$TOTAL" -gt 900 ]; then
    echo "⚠ OVER BUDGET — consider splitting slow tests to nightly"
else
    echo "✓ Within budget"
fi
