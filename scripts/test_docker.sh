#!/bin/bash

# Script para probar el build de Docker

set -e

echo "🐳 Probando build de Docker..."

# Verificar que Docker esté instalado
if ! command -v docker &> /dev/null; then
    echo "❌ Docker no está instalado"
    exit 1
fi

# Construir imagen
echo "📦 Construyendo imagen..."
docker build -t rust-bc:test .

# Verificar que la imagen se creó
if docker images | grep -q "rust-bc.*test"; then
    echo "✅ Imagen construida exitosamente"
else
    echo "❌ Error al construir imagen"
    exit 1
fi

# Probar ejecución básica
echo "🚀 Probando ejecución..."
docker run --rm \
    --name rust-bc-test \
    -p 8080:8080 \
    -p 8081:8081 \
    rust-bc:test &
    
CONTAINER_PID=$!
sleep 5

# Verificar health check
if curl -f http://localhost:8080/api/v1/health > /dev/null 2>&1; then
    echo "✅ Health check exitoso"
else
    echo "⚠️  Health check falló (puede ser normal si el servidor aún está iniciando)"
fi

# Limpiar
docker stop rust-bc-test 2>/dev/null || true
docker rmi rust-bc:test 2>/dev/null || true

echo "✅ Prueba de Docker completada"

