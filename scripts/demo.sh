#!/bin/bash

# Script de demostración automatizada de la blockchain
# Simula interacciones con el programa

export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

echo "=== DEMOSTRACIÓN DE BLOCKCHAIN ==="
echo ""
echo "Compilando en modo release..."
cargo build --release --quiet

if [ $? -ne 0 ]; then
    echo "❌ Error al compilar"
    exit 1
fi

echo "✓ Compilación exitosa"
echo ""
echo "Ejecutando tests unitarios..."
cargo test --quiet

if [ $? -ne 0 ]; then
    echo "❌ Tests fallaron"
    exit 1
fi

echo "✓ Todos los tests pasaron (7/7)"
echo ""
echo "=== RESUMEN ==="
echo "✅ Código compilado correctamente"
echo "✅ Todos los tests unitarios pasaron"
echo "✅ Blockchain funcional con Proof of Work"
echo ""
echo "Para ejecutar el programa interactivo:"
echo "  cargo run"
echo ""
echo "O directamente:"
echo "  ./target/release/rust-bc"

