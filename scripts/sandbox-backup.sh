#!/usr/bin/env bash
# Cerulean Ledger — Sandbox Backup/Export
#
# Creates a timestamped tarball of the sandbox RocksDB volume.
#
# Usage:
#   ./scripts/sandbox-backup.sh              # Backup to ./backups/
#   ./scripts/sandbox-backup.sh /path/to     # Backup to custom directory
#   ./scripts/sandbox-backup.sh restore <tarball>  # Restore from backup

set -euo pipefail
cd "$(dirname "$0")/.."

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

COMPOSE_FILE="docker-compose.sandbox.yml"
VOLUME_NAME="rust-bc_sandbox-data"
BACKUP_DIR="${1:-./backups}"

# ── Restore mode ─────────────────────────────────────────────────────────────

if [[ "${1:-}" == "restore" ]]; then
    TARBALL="${2:-}"
    if [[ -z "$TARBALL" || ! -f "$TARBALL" ]]; then
        echo -e "${RED}Usage: $0 restore <tarball-path>${NC}"
        exit 1
    fi

    echo -e "${CYAN}Stopping sandbox...${NC}"
    docker compose -f "$COMPOSE_FILE" down 2>/dev/null || true

    echo -e "${CYAN}Restoring from: $TARBALL${NC}"
    docker volume rm "$VOLUME_NAME" 2>/dev/null || true
    docker volume create "$VOLUME_NAME"
    docker run --rm -v "$VOLUME_NAME":/data -v "$(realpath "$TARBALL")":/backup.tar.gz alpine \
        sh -c "cd /data && tar xzf /backup.tar.gz"

    echo -e "${GREEN}Restore complete. Run: ./scripts/sandbox.sh${NC}"
    exit 0
fi

# ── Backup mode ──────────────────────────────────────────────────────────────

mkdir -p "$BACKUP_DIR"

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
FILENAME="cerulean-sandbox-${TIMESTAMP}.tar.gz"
FILEPATH="$BACKUP_DIR/$FILENAME"

echo -e "${CYAN}Backing up sandbox data...${NC}"
echo -e "  Volume: $VOLUME_NAME"
echo -e "  Output: $FILEPATH"

# Check volume exists
if ! docker volume inspect "$VOLUME_NAME" >/dev/null 2>&1; then
    echo -e "${RED}Volume $VOLUME_NAME not found. Is the sandbox running?${NC}"
    exit 1
fi

# Create backup via temporary container
docker run --rm \
    -v "$VOLUME_NAME":/data:ro \
    -v "$(realpath "$BACKUP_DIR")":/backup \
    alpine \
    tar czf "/backup/$FILENAME" -C /data .

SIZE=$(du -h "$FILEPATH" | cut -f1)
echo -e "${GREEN}Backup complete: $FILEPATH ($SIZE)${NC}"
echo ""
echo -e "  Restore with: ./scripts/sandbox-backup.sh restore $FILEPATH"
