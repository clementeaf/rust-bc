#!/usr/bin/env bash
# Clean-room reproducible build verification for pqc_crypto_module.
# Designed to run in a fresh environment (Docker/VM or CI).

set -euo pipefail

echo "=== Clean-Room Build Verification ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# 1. Print toolchain info
echo "--- Toolchain ---"
rustc --version
cargo --version
echo "Target: $(rustc -vV | grep host | awk '{print $2}')"
echo ""

# 2. Fetch dependencies (network required here only)
echo "--- Fetching dependencies ---"
cargo fetch --locked
echo ""

# 3. Build in release mode (no network needed after fetch)
echo "--- Building pqc_crypto_module (release) ---"
cargo build --locked --release -p pqc_crypto_module 2>&1
echo ""

# 4. Hash the produced artifact
echo "--- Artifact hash ---"
ARTIFACT=$(find target/release -name "libpqc_crypto_module*" -type f | head -1)
if [ -n "$ARTIFACT" ]; then
    echo "Artifact: $ARTIFACT"
    shasum -a 256 "$ARTIFACT"
else
    echo "Artifact: (library crate — no binary output, build verified via exit code)"
fi
echo ""

# 5. Run self-tests
echo "--- Self-tests ---"
cargo test -p pqc_crypto_module --release -- --test-threads=1 2>&1 | tail -5
echo ""

echo "=== Build verification complete ==="
