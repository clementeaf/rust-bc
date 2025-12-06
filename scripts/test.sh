#!/bin/bash

# Script de prueba para la blockchain
# Requiere Rust y Cargo instalados

echo "=== PRUEBAS DE BLOCKCHAIN ==="
echo ""

# Verificar que cargo está instalado
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo no está instalado"
    echo "Instala Rust desde: https://rustup.rs/"
    exit 1
fi

echo "✓ Cargo encontrado"
echo ""

# Compilar el proyecto
echo "Compilando proyecto..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "❌ Error al compilar"
    exit 1
fi
echo "✓ Compilación exitosa"
echo ""

# Ejecutar tests unitarios
echo "Ejecutando tests unitarios..."
cargo test
if [ $? -ne 0 ]; then
    echo "❌ Tests fallaron"
    exit 1
fi
echo "✓ Todos los tests pasaron"
echo ""

# Verificar el tamaño del código
echo "Verificando tamaño del código..."
LINES=$(wc -l < src/main.rs)
echo "Líneas de código: $LINES"
if [ $LINES -gt 300 ]; then
    echo "⚠️  Advertencia: El código excede 300 líneas"
else
    echo "✓ Código dentro del límite de 300 líneas"
fi
echo ""

echo "=== PRUEBAS COMPLETADAS ==="
echo ""
echo "Para ejecutar el programa interactivo:"
echo "  cargo run"
echo ""
echo "Para ejecutar solo los tests:"
echo "  cargo test"

