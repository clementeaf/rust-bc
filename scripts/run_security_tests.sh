#!/bin/bash

# Script para ejecutar pruebas de seguridad
# Verifica que el servidor esté corriendo antes de ejecutar las pruebas

API_URL="http://127.0.0.1:8080/api/v1"

echo "🛡️  PREPARANDO PRUEBAS DE SEGURIDAD"
echo "====================================="
echo ""

# Verificar que el servidor esté corriendo
echo "⏳ Verificando servidor..."
for i in {1..10}; do
    if curl -s --max-time 2 "$API_URL/health" >/dev/null 2>&1; then
        echo "✅ Servidor detectado y respondiendo"
        echo ""
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ ERROR: El servidor no está respondiendo"
        echo ""
        echo "💡 Para iniciar el servidor, ejecuta en otra terminal:"
        echo "   DIFFICULTY=1 cargo run --release 8080 8081 blockchain"
        echo ""
        echo "   Luego ejecuta este script nuevamente."
        exit 1
    fi
    echo -n "."
    sleep 1
done

echo "🚀 Ejecutando pruebas de seguridad..."
echo ""

# Ejecutar las pruebas
./scripts/test_security_attacks.sh

