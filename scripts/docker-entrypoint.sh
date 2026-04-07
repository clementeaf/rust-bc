#!/bin/bash
set -e

# Función para mostrar ayuda
show_help() {
    cat << EOF
Rust Blockchain Node

Uso:
    docker run [OPTIONS] rust-bc [API_PORT] [P2P_PORT] [DB_NAME]

Variables de entorno:
    API_PORT      - Puerto para API REST (default: 8080)
    P2P_PORT      - Puerto para red P2P (default: 8081)
    DB_NAME       - Nombre de la base de datos (default: blockchain)
    DIFFICULTY    - Dificultad de minería (default: 1)
    RUST_LOG      - Nivel de logging (default: info)

Ejemplos:
    # Uso básico
    docker run -p 8080:8080 -p 8081:8081 rust-bc

    # Con puertos personalizados
    docker run -p 3000:3000 -p 4000:4000 rust-bc 3000 4000

    # Con base de datos persistente
    docker run -v blockchain-data:/app/data rust-bc

    # Con variables de entorno
    docker run -e API_PORT=3000 -e DIFFICULTY=2 rust-bc
EOF
}

# Mostrar ayuda si se solicita
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    show_help
    exit 0
fi

# Configurar rutas
DB_PATH="/app/data/${DB_NAME:-blockchain}.db"
mkdir -p /app/data

# Si se pasan argumentos numéricos, usarlos como puertos
if [ $# -ge 1 ] && [[ "$1" =~ ^[0-9]+$ ]]; then
    API_PORT=${1:-${API_PORT:-8080}}
    P2P_PORT=${2:-${P2P_PORT:-8081}}
    DB_NAME=${3:-${DB_NAME:-blockchain}}
    DB_PATH="/app/data/${DB_NAME}.db"
fi

# Log de inicio
echo "🚀 Iniciando Rust Blockchain Node..."
echo "📊 Configuración:"
echo "   - API Port: ${API_PORT:-8080}"
echo "   - P2P Port: ${P2P_PORT:-8081}"
echo "   - Database: ${DB_PATH}"
echo "   - Difficulty: ${DIFFICULTY:-1}"
echo "   - Log Level: ${RUST_LOG:-info}"

# Ejecutar la aplicación
# Los argumentos se pasan directamente: API_PORT P2P_PORT DB_NAME
exec rust-bc \
    "${API_PORT:-8080}" \
    "${P2P_PORT:-8081}" \
    "${DB_NAME:-blockchain}"

