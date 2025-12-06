#!/bin/bash

# Script simple para iniciar un nodo
# Uso: ./start_node.sh <api_port> <p2p_port> <db_name>

API_PORT=${1:-8080}
P2P_PORT=${2:-8081}
DB_NAME=${3:-blockchain}

cd /Users/clementefalcone/Desktop/personal/rust-bc
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

echo "🚀 Iniciando nodo: API=$API_PORT, P2P=$P2P_PORT, DB=$DB_NAME"
cargo run --release $API_PORT $P2P_PORT $DB_NAME

