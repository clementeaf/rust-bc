#!/usr/bin/env bash
# Cerulean Ledger — Sandbox Launcher
#
# Levanta el nodo + frontends via Docker Compose y los expone
# al internet via Cloudflare Tunnel (Quick Tunnel — sin cuenta ni config).
#
# Requisitos:
#   - Docker y Docker Compose instalados
#   - cloudflared instalado (brew install cloudflare/cloudflare/cloudflared)
#
# Uso:
#   ./scripts/sandbox.sh           # Quick tunnel (URL temporal de Cloudflare)
#   ./scripts/sandbox.sh stop      # Apaga todo
#
# Para dominio propio, ver SANDBOX.md.

set -euo pipefail
cd "$(dirname "$0")/.."

COMPOSE_FILE="docker-compose.sandbox.yml"

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ── Stop ──────────────────────────────────────────────────────────────────────

if [[ "${1:-}" == "stop" ]]; then
    echo -e "${CYAN}Stopping sandbox...${NC}"
    docker compose -f "$COMPOSE_FILE" down
    pkill -f "cloudflared.*tunnel" 2>/dev/null || true
    echo -e "${GREEN}Sandbox stopped.${NC}"
    exit 0
fi

# ── Preflight ─────────────────────────────────────────────────────────────────

if ! command -v docker &>/dev/null; then
    echo -e "${RED}Docker not found. Install Docker first.${NC}"
    exit 1
fi

if ! command -v cloudflared &>/dev/null; then
    echo -e "${YELLOW}cloudflared not found. Installing via brew...${NC}"
    brew install cloudflare/cloudflare/cloudflared
fi

# ── Build & Start ─────────────────────────────────────────────────────────────

echo -e "${CYAN}Building sandbox containers...${NC}"
docker compose -f "$COMPOSE_FILE" build

echo -e "${CYAN}Starting sandbox...${NC}"
docker compose -f "$COMPOSE_FILE" up -d

echo ""
echo -e "${CYAN}Waiting for node health...${NC}"
for i in $(seq 1 30); do
    if curl -sf http://localhost:9600/api/v1/health > /dev/null 2>&1; then
        echo -e "${GREEN}Node healthy.${NC}"
        break
    fi
    if [[ $i -eq 30 ]]; then
        echo -e "${RED}Node failed to start. Check: docker compose -f $COMPOSE_FILE logs${NC}"
        exit 1
    fi
    sleep 2
done

# ── Seed Data ────────────────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}Seeding demo data...${NC}"
./scripts/seed-sandbox.sh http://localhost:9600 || echo -e "${YELLOW}Seed script failed (non-fatal).${NC}"

# ── Quick Tunnels ─────────────────────────────────────────────────────────────

echo ""
echo -e "${CYAN}Starting Cloudflare Quick Tunnels...${NC}"
echo -e "${YELLOW}(URLs are temporary — valid while this script runs)${NC}"
echo ""

TUNNEL_DIR=$(mktemp -d)
trap "rm -rf $TUNNEL_DIR" EXIT

# Tunnel for explorer (main entry point)
cloudflared tunnel --url http://localhost:5173 \
    --logfile "$TUNNEL_DIR/explorer.log" \
    --pidfile "$TUNNEL_DIR/explorer.pid" &
EXPLORER_PID=$!

# Tunnel for voto
cloudflared tunnel --url http://localhost:5174 \
    --logfile "$TUNNEL_DIR/voto.log" \
    --pidfile "$TUNNEL_DIR/voto.pid" &
VOTO_PID=$!

# Tunnel for raw API
cloudflared tunnel --url http://localhost:9600 \
    --logfile "$TUNNEL_DIR/api.log" \
    --pidfile "$TUNNEL_DIR/api.pid" &
API_PID=$!

# Wait for tunnels to establish and extract URLs
sleep 5

extract_url() {
    local logfile="$1"
    grep -oE 'https://[a-z0-9-]+\.trycloudflare\.com' "$logfile" 2>/dev/null | head -1
}

EXPLORER_URL=$(extract_url "$TUNNEL_DIR/explorer.log")
VOTO_URL=$(extract_url "$TUNNEL_DIR/voto.log")
API_URL=$(extract_url "$TUNNEL_DIR/api.log")

# Retry if URLs not yet available
if [[ -z "$EXPLORER_URL" || -z "$VOTO_URL" || -z "$API_URL" ]]; then
    sleep 5
    EXPLORER_URL=$(extract_url "$TUNNEL_DIR/explorer.log")
    VOTO_URL=$(extract_url "$TUNNEL_DIR/voto.log")
    API_URL=$(extract_url "$TUNNEL_DIR/api.log")
fi

echo -e "${GREEN}============================================${NC}"
echo -e "${GREEN} Cerulean Sandbox Live${NC}"
echo -e "${GREEN}============================================${NC}"
echo ""
echo -e "  Block Explorer:  ${CYAN}${EXPLORER_URL:-pending...}${NC}"
echo -e "  Cerulean Voto:   ${CYAN}${VOTO_URL:-pending...}${NC}"
echo -e "  API (raw):       ${CYAN}${API_URL:-pending...}${NC}"
echo ""
echo -e "  Local ports:     explorer :5173  |  voto :5174  |  api :9600"
echo ""
echo -e "${YELLOW}  Press Ctrl+C to stop all tunnels.${NC}"
echo -e "${YELLOW}  Run ./scripts/sandbox.sh stop to clean up containers.${NC}"
echo ""

# Keep alive — wait for any tunnel to exit
wait $EXPLORER_PID $VOTO_PID $API_PID 2>/dev/null
