# Multi-stage build with cargo cache for fast rebuilds
FROM debian:bookworm AS builder

# Install Rust nightly (latest) via rustup
RUN apt-get update && apt-get install -y \
    curl pkg-config libssl-dev build-essential clang libclang-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly-2024-12-18

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Layer 1: cache dependencies — only re-run when Cargo.toml/lock change
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# Layer 2: build real code — only re-run when src/ changes
COPY . .
RUN cargo build --release

# Imagen final minimalista
FROM debian:bookworm-slim

# Instalar solo runtime dependencies (incluyendo curl para health check)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Crear usuario no-root para seguridad
RUN useradd -m -u 1000 rustbc && \
    mkdir -p /app/data && \
    chown -R rustbc:rustbc /app

WORKDIR /app

# Copiar binario compilado
COPY --from=builder /app/target/release/rust-bc /usr/local/bin/rust-bc

# Copiar scripts de inicio (si existen)
COPY --chown=rustbc:rustbc scripts/docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Entrypoint fixes ownership of /app/data then drops to rustbc (see docker-entrypoint.sh).
USER root

# Variables de entorno por defecto
ENV API_PORT=8080
ENV P2P_PORT=8081
ENV DB_NAME=blockchain
ENV DIFFICULTY=1
ENV RUST_LOG=info

# Exponer puertos
EXPOSE 8080 8081

# Volumen para datos persistentes
VOLUME ["/app/data"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -fk https://localhost:${API_PORT}/api/v1/health || exit 1

# Entrypoint — env vars (API_PORT, P2P_PORT, etc.) configure the node.
# Do NOT pass positional args via CMD; the entrypoint treats $1 as API_PORT.
ENTRYPOINT ["/app/docker-entrypoint.sh"]
