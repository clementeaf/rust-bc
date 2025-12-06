#!/bin/bash

# Script para iniciar el servidor con configuración optimizada para pruebas

cd "$(dirname "$0")/.."

echo "🚀 Iniciando servidor con configuración de pruebas..."
echo "📊 Dificultad: 1 (rápido para pruebas)"
echo ""

# Detener procesos anteriores
pkill -f "rust-bc|cargo run" 2>/dev/null
lsof -ti:8080,8081 | xargs kill -9 2>/dev/null
sleep 1

# Iniciar servidor
source ~/.cargo/env 2>/dev/null || true
DIFFICULTY=1 cargo run --release 8080 8081 blockchain > /tmp/rust-bc-server.log 2>&1 &

SERVER_PID=$!
echo "✅ Servidor iniciado (PID: $SERVER_PID)"
echo "📝 Logs: /tmp/rust-bc-server.log"
echo ""

# Esperar a que el servidor esté listo
echo "⏳ Esperando a que el servidor esté listo..."
for i in {1..10}; do
    if curl -s --max-time 1 http://localhost:8080/api/v1/health > /dev/null 2>&1; then
        echo "✅ Servidor listo!"
        echo ""
        echo "🌐 API: http://localhost:8080"
        echo "📡 P2P: 127.0.0.1:8081"
        echo ""
        echo "Para detener: pkill -f 'rust-bc'"
        exit 0
    fi
    sleep 1
    echo -n "."
done

echo ""
echo "❌ El servidor no respondió a tiempo"
echo "Revisa los logs: tail -f /tmp/rust-bc-server.log"
exit 1

