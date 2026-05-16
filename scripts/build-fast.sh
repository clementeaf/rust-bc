#!/usr/bin/env bash
# Fast build: cross-compile on host, Docker image in seconds.
#
# Prerequisites (one-time):
#   rustup target add x86_64-unknown-linux-gnu
#   brew install filosottile/musl-cross/musl-cross   # macOS cross-linker
#   # OR: apt install gcc-x86-64-linux-gnu           # Linux
#
# Usage:
#   ./scripts/build-fast.sh          # Build + docker image
#   ./scripts/build-fast.sh --up     # Build + image + docker compose up

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

TARGET="x86_64-unknown-linux-gnu"

echo "━━━ Fast Build Pipeline ━━━"
echo ""

# ── Step 1: Cross-compile ────────────────────────────────────────────────
echo "[1/3] Cross-compiling for $TARGET..."

# Detect cross-linker
if command -v x86_64-linux-gnu-gcc &>/dev/null; then
    export CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
elif command -v x86_64-linux-musl-gcc &>/dev/null; then
    export CC_x86_64_unknown_linux_gnu=x86_64-linux-musl-gcc
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-musl-gcc
else
    echo "No cross-linker found. Falling back to cargo-cross or Docker build."
    echo ""
    echo "Install one of:"
    echo "  macOS:  brew install filosottile/musl-cross/musl-cross"
    echo "  Linux:  apt install gcc-x86-64-linux-gnu"
    echo ""
    echo "Or use: docker compose build (slow but works everywhere)"
    exit 1
fi

time cargo build --release --target "$TARGET" 2>&1
echo ""

BINARY="target/$TARGET/release/rust-bc"
if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi
echo "  Binary: $(du -h "$BINARY" | cut -f1)"

# ── Step 2: Docker image ────────────────────────────────────────────────
echo "[2/3] Building Docker image..."
# Inline minimal Dockerfile — copies the pre-compiled binary, no build stage needed.
time docker build -t rust-bc:latest -f - . <<'DOCKERFILE'
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -m -u 1000 rustbc && mkdir -p /app/data && chown -R rustbc:rustbc /app
WORKDIR /app
COPY target/x86_64-unknown-linux-gnu/release/rust-bc /usr/local/bin/rust-bc
ENV API_PORT=8080 P2P_PORT=8081 BIND_ADDR=0.0.0.0 DIFFICULTY=1 RUST_LOG=info
EXPOSE 8080 8081
VOLUME ["/app/data"]
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -fk https://localhost:${API_PORT}/api/v1/health || exit 1
CMD ["rust-bc"]
DOCKERFILE
echo ""

# ── Step 3: Optionally start ────────────────────────────────────────────
if [[ "${1:-}" == "--up" ]]; then
    echo "[3/3] Starting network..."
    docker compose down -v 2>/dev/null || true
    docker compose up -d
    echo ""
    echo "Waiting for health..."
    sleep 10
    docker compose ps --format "{{.Name}}: {{.Status}}" | grep -v "level=warning"
else
    echo "[3/3] Done. Run: docker compose up -d"
fi

echo ""
echo "━━━ Build complete ━━━"
