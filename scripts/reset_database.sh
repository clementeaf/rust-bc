#!/bin/bash

/**
 * Script para resetear la base de datos cuando hay forks o corrupción
 * Elimina la base de datos y permite empezar con una cadena limpia
 */

DB_NAME="${1:-blockchain}"
DB_PATH="${DB_NAME}.db"

echo "🔄 RESET DE BASE DE DATOS"
echo "=========================="
echo ""

if [ ! -f "$DB_PATH" ]; then
    echo "ℹ️  La base de datos $DB_PATH no existe. No hay nada que resetear."
    exit 0
fi

echo "⚠️  ADVERTENCIA: Esto eliminará completamente la base de datos $DB_PATH"
echo "   Todos los bloques y datos serán perdidos."
echo ""
read -p "¿Estás seguro? (escribe 'yes' para confirmar): " confirmation

if [ "$confirmation" != "yes" ]; then
    echo "❌ Operación cancelada."
    exit 1
fi

echo ""
echo "🛑 Deteniendo servidor si está corriendo..."
pkill -f "rust-bc.*8080" 2>/dev/null
sleep 2

echo "🗑️  Eliminando base de datos..."
rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✅ Base de datos eliminada exitosamente."
    echo ""
    echo "💡 Para iniciar el servidor con una cadena limpia:"
    echo "   DIFFICULTY=1 cargo run --release 8080 8081 $DB_NAME"
else
    echo "❌ Error al eliminar la base de datos."
    exit 1
fi

