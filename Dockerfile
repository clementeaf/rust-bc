# Multi-stage build para optimizar tamaño de imagen
# Usar versión más reciente de Rust (latest) para soportar dependencias modernas
FROM rust:latest as builder

# Instalar dependencias del sistema
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Crear directorio de trabajo
WORKDIR /app

# Copiar todo el código fuente
COPY . .

# Compilar la aplicación
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

# Cambiar a usuario no-root
USER rustbc

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
    CMD curl -f http://localhost:${API_PORT}/api/v1/health || exit 1

# Entrypoint
ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["rust-bc"]

