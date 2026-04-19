FROM rust:1.87-slim AS builder
WORKDIR /build
COPY Cargo.toml ./
COPY src/ ./src/
RUN cargo build --release --bin node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/node /usr/local/bin/tesseract-node
RUN mkdir -p /data
ENV DATA_DIR=/data
EXPOSE 7700
CMD ["tesseract-node"]
