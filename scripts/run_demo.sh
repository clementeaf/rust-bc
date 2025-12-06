#!/bin/bash

export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

echo "=== EJECUTANDO BLOCKCHAIN ==="
echo ""
echo "Iniciando programa..."
echo ""

# Ejecutar el programa con entrada automatizada
{
    sleep 1
    echo "2"  # Ver cadena completa
    sleep 1
    echo "1"  # Minar nuevo bloque
    sleep 1
    echo "Transacción de prueba #1"
    sleep 2
    echo "2"  # Ver cadena completa de nuevo
    sleep 1
    echo "3"  # Verificar cadena
    sleep 1
    echo "4"  # Salir
} | cargo run --release

