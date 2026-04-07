#!/usr/bin/env bash
# Rolling upgrade — updates nodes one at a time with zero downtime.
#
# Usage: ./scripts/rolling-upgrade.sh [--build]
#
# With --build: rebuilds Docker images before upgrading.
# Without --build: uses existing images (for pre-built releases).
#
# Order: orderer first (to ensure ordering continuity), then peers.
# Each node is health-checked before moving to the next.

set -euo pipefail

CURL="curl -sk --max-time 10"
COMPOSE="docker compose"
UPGRADE_ORDER=(orderer1 node3 node2 node1)
HEALTH_RETRIES=30
HEALTH_INTERVAL=2

red()   { printf "\033[31m%s\033[0m\n" "$1"; }
green() { printf "\033[32m%s\033[0m\n" "$1"; }
bold()  { printf "\033[1m%s\033[0m\n" "$1"; }

port_for() {
    case "$1" in
        node1)    echo 8080 ;;
        node2)    echo 8082 ;;
        node3)    echo 8084 ;;
        orderer1) echo 8086 ;;
        *)        echo 8080 ;;
    esac
}

wait_healthy() {
    local node="$1"
    local port
    port=$(port_for "$node")
    echo -n "  Waiting for $node to be healthy..."
    for i in $(seq 1 $HEALTH_RETRIES); do
        if $CURL "https://localhost:$port/api/v1/health" 2>/dev/null | grep -q healthy; then
            green " OK (${i}s)"
            return 0
        fi
        sleep $HEALTH_INTERVAL
    done
    red " FAILED after $((HEALTH_RETRIES * HEALTH_INTERVAL))s"
    return 1
}

check_consistency() {
    echo -n "  Checking chain consistency..."
    local heights=()
    for node in node1 node2 node3; do
        local port
        port=$(port_for "$node")
        local h
        h=$($CURL "https://localhost:$port/api/v1/stats" 2>/dev/null | \
            python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('blockchain',{}).get('block_count',0))" 2>/dev/null || echo "0")
        heights+=("$h")
    done

    if [[ "${heights[0]}" == "${heights[1]}" && "${heights[1]}" == "${heights[2]}" ]]; then
        green " consistent (height=${heights[0]})"
        return 0
    else
        red " inconsistent: ${heights[*]}"
        return 1
    fi
}

# ── Main ─────────────────────────────────────────────────────────────────────

bold "═══ rust-bc Rolling Upgrade ═══"
echo ""

if [[ "${1:-}" == "--build" ]]; then
    bold "Step 0: Building new images..."
    $COMPOSE build
    echo ""
fi

for node in "${UPGRADE_ORDER[@]}"; do
    bold "Upgrading $node..."

    echo "  Stopping $node..."
    $COMPOSE stop "$node"

    echo "  Starting $node with new image..."
    $COMPOSE up -d "$node"

    if ! wait_healthy "$node"; then
        red "ABORT: $node failed to become healthy after upgrade."
        red "Rolling back: $COMPOSE up -d $node"
        exit 1
    fi

    # Brief pause to let gossip stabilize
    sleep 3
    echo ""
done

bold "Post-upgrade verification..."
check_consistency

echo ""
green "Rolling upgrade complete. All nodes healthy and consistent."
