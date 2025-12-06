#!/bin/bash

# Script para ejecutar un nodo con puertos personalizados
# Uso: ./run_node.sh <api_port> <p2p_port> <db_name>

API_PORT=${1:-8080}
P2P_PORT=${2:-8081}
DB_NAME=${3:-blockchain.db}

echo "🚀 Iniciando nodo..."
echo "   API Port: $API_PORT"
echo "   P2P Port: $P2P_PORT"
echo "   Database: $DB_NAME"

export RUST_BACKTRACE=1
export API_PORT=$API_PORT
export P2P_PORT=$P2P_PORT
export DB_NAME=$DB_NAME

# Modificar temporalmente el código para usar variables de entorno
# O mejor, crear un binario separado

cd /Users/clementefalcone/Desktop/personal/rust-bc
cargo run --release

