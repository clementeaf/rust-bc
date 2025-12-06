#!/bin/bash

set +e

echo "🔥 EJECUTANDO SUITE COMPLETA DE PRUEBAS DE ESTRÉS"
echo "================================================="
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

if ! pgrep -f "rust-bc" > /dev/null; then
    echo "⚠️  El servidor no está corriendo, iniciando..."
    cd "$SCRIPT_DIR/.."
    source ~/.cargo/env 2>/dev/null || true
    DIFFICULTY=1 cargo run --release 8080 8081 blockchain > /tmp/rust-bc-server.log 2>&1 &
    sleep 3
    if ! curl -s http://localhost:8080/api/v1/health > /dev/null 2>&1; then
        echo "❌ No se pudo iniciar el servidor"
        exit 1
    fi
    echo "✅ Servidor iniciado con dificultad 1 (rápido para pruebas)"
fi

echo "✅ Servidor detectado"
echo ""

echo "📊 FASE 1: Pruebas Críticas (Casos Límite)"
echo "==========================================="
if bash scripts/test_critical.sh; then
    echo "✅ Pruebas críticas completadas"
else
    echo "❌ Pruebas críticas fallaron"
fi
echo ""

sleep 2

echo "📊 FASE 2: Pruebas de Estrés (Carga Puntual)"
echo "=============================================="
if bash scripts/test_stress.sh; then
    echo "✅ Pruebas de estrés completadas"
else
    echo "❌ Pruebas de estrés fallaron"
fi
echo ""

sleep 2

echo "📊 FASE 3: Pruebas de Carga Prolongada (60 segundos)"
echo "====================================================="
echo "⚠️  Esta prueba tomará 60 segundos..."
if bash scripts/test_load.sh; then
    echo "✅ Pruebas de carga completadas"
else
    echo "❌ Pruebas de carga fallaron"
fi
echo ""

echo "===================================="
echo "🎯 SUITE COMPLETA FINALIZADA"
echo "===================================="
echo ""
echo "📁 Resultados guardados en:"
echo "   - test_results_critical/"
echo "   - test_results_stress/"
echo "   - load_test_results_*.txt"
echo ""

