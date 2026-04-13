# syntax=docker/dockerfile:1
# Multi-stage build with BuildKit cache for fast rebuilds.
# First build: ~8 min. Subsequent builds (code-only changes): ~30-60s.
#
# Build with:  docker compose build
# Fast option: ./scripts/build-fast.sh (cross-compile, ~5s Docker step)

FROM debian:bookworm AS builder

RUN apt-get update && apt-get install -y \
    curl pkg-config libssl-dev build-essential clang libclang-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly-2025-05-01

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Dummy src for dependency pre-build
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs

# Cache dependencies with BuildKit mount cache
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release 2>/dev/null || true

# Copy real source
COPY . .

# Build with cached deps — only recompiles project code
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release && cp target/release/rust-bc /usr/local/bin/rust-bc

# ── Runtime image ────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 rustbc && \
    mkdir -p /app/data && \
    chown -R rustbc:rustbc /app

WORKDIR /app

COPY --from=builder /usr/local/bin/rust-bc /usr/local/bin/rust-bc
COPY --chown=rustbc:rustbc scripts/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

USER root

ENV API_PORT=8080
ENV P2P_PORT=8081
ENV DB_NAME=blockchain
ENV DIFFICULTY=1
ENV RUST_LOG=info

EXPOSE 8080 8081
VOLUME ["/app/data"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -fk https://localhost:${API_PORT}/api/v1/health || exit 1

ENTRYPOINT ["/app/docker-entrypoint.sh"]
