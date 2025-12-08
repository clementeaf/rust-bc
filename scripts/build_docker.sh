#!/bin/bash

# Script para construir la imagen Docker de rust-bc

set -e

echo "🐳 Construyendo imagen Docker de rust-bc..."

# Verificar que Docker esté instalado
if ! command -v docker &> /dev/null; then
    echo "❌ Docker no está instalado"
    echo "   Instala Docker Desktop desde: https://www.docker.com/products/docker-desktop"
    exit 1
fi

# Verificar que Docker daemon esté corriendo
if ! docker ps &> /dev/null; then
    echo "❌ Docker daemon no está corriendo"
    echo ""
    echo "Por favor:"
    echo "1. Abre Docker Desktop"
    echo "2. Espera a que inicie completamente"
    echo "3. Luego ejecuta este script nuevamente"
    echo ""
    # Intentar abrir Docker Desktop en macOS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "💡 Intentando abrir Docker Desktop..."
        open -a Docker 2>/dev/null || true
    fi
    exit 1
fi

echo "✅ Docker está corriendo"
echo ""

# Construir imagen
echo "📦 Construyendo imagen (esto puede tomar varios minutos)..."
echo ""

docker build -t rust-bc:latest . --progress=plain

echo ""
echo "✅ Build completado exitosamente!"
echo ""
echo "📊 Información de la imagen:"
docker images rust-bc:latest

echo ""
echo "🚀 Para ejecutar el nodo:"
echo "   docker run -d --name rust-bc-node -p 8080:8080 -p 8081:8081 -v blockchain-data:/app/data rust-bc:latest"
echo ""
echo "📚 Ver DOCKER.md para más información"

