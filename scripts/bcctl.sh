#!/usr/bin/env bash
# bcctl — operator CLI for rust-bc blockchain network.
#
# Usage:
#   bcctl status                  Show all nodes health and peer count
#   bcctl peers                   Show P2P connectivity matrix
#   bcctl blocks [node]           Show latest block info (default: node1)
#   bcctl mine [address] [node]   Mine a block
#   bcctl wallet create [node]    Create a new wallet
#   bcctl channels [node]         List channels
#   bcctl channel create <id>     Create a new channel
#   bcctl orgs [node]             List organizations
#   bcctl logs <node> [lines]     Show node logs (default: 50 lines)
#   bcctl restart [node]          Restart a node (or all)
#   bcctl metrics <node>          Show Prometheus metrics
#   bcctl verify [node]           Verify chain integrity
#   bcctl consistency             Compare chain state across all peers
#   bcctl env                     Show network configuration

set -uo pipefail

CURL="curl -sk --max-time 10"
COMPOSE="docker compose"

PEERS=(node1 node2 node3)
ALL_NODES=(node1 node2 node3 orderer1)

port_for() {
    case "$1" in
        node1)    echo 8080 ;;
        node2)    echo 8082 ;;
        node3)    echo 8084 ;;
        orderer1) echo 8086 ;;
        *)        echo 8080 ;;
    esac
}

p2p_port_for() {
    case "$1" in
        node1)    echo 8081 ;;
        node2)    echo 8083 ;;
        node3)    echo 8085 ;;
        orderer1) echo 8087 ;;
        *)        echo 8081 ;;
    esac
}

url() { echo "https://localhost:$(port_for "$1")"; }

api() {
    local node="$1" path="$2"
    shift 2
    $CURL "$(url "$node")/api/v1/$path" "$@" 2>/dev/null
}

json() { python3 -m json.tool 2>/dev/null || cat; }

# ── Commands ─────────────────────────────────────────────────────────────────

cmd_status() {
    printf "%-12s %-10s %-8s %-8s %-20s\n" "NODE" "STATUS" "BLOCKS" "PEERS" "LATEST HASH"
    printf "%-12s %-10s %-8s %-8s %-20s\n" "────" "──────" "──────" "─────" "───────────"
    for node in "${ALL_NODES[@]}"; do
        local health blocks peers hash
        health=$(api "$node" "health" | jq -r '.data.status // "down"' 2>/dev/null || echo "down")
        local stats
        stats=$(api "$node" "stats" 2>/dev/null)
        blocks=$(echo "$stats" | jq -r '.data.blockchain.block_count // "-"' 2>/dev/null)
        peers=$(echo "$stats" | jq -r '.data.network.connected_peers // "-"' 2>/dev/null)
        hash=$(echo "$stats" | jq -r '.data.blockchain.latest_block_hash // "-"' 2>/dev/null)
        hash="${hash:0:16}..."

        local status_color
        if [[ "$health" == "healthy" ]]; then
            status_color="\033[32m$health\033[0m"
        else
            status_color="\033[31m$health\033[0m"
        fi
        printf "%-12s %-20b %-8s %-8s %-20s\n" "$node" "$status_color" "$blocks" "$peers" "$hash"
    done
}

cmd_peers() {
    echo "P2P Connectivity:"
    echo ""
    for node in "${ALL_NODES[@]}"; do
        local peers
        peers=$(api "$node" "peers" 2>/dev/null | jq -r '.data[]? // empty' 2>/dev/null)
        if [[ -z "$peers" ]]; then
            peers=$(api "$node" "stats" | jq -r '.data.network.connected_peers // 0' 2>/dev/null)
            echo "  $node: $peers peers (details not available via legacy endpoint)"
        else
            echo "  $node:"
            echo "$peers" | while read -r p; do echo "    -> $p"; done
        fi
    done
}

cmd_blocks() {
    local node="${1:-node1}"
    echo "Latest blocks on $node:"
    api "$node" "stats" | jq '.data.blockchain' 2>/dev/null | json
}

cmd_mine() {
    local address="${1:-}"
    local node="${2:-node1}"

    if [[ -z "$address" ]]; then
        echo "Creating wallet..."
        address=$(api "$node" "wallets/create" -X POST -d '{}' | jq -r '.data.address' 2>/dev/null)
        echo "  Wallet: $address"
    fi

    echo "Mining on $node..."
    local resp
    resp=$(api "$node" "mine" -X POST -H 'Content-Type: application/json' \
        -d "{\"miner_address\":\"$address\"}")
    echo "$resp" | jq '{hash: .data.hash, reward: .data.reward, txs: .data.transactions_count}' 2>/dev/null
}

cmd_wallet_create() {
    local node="${1:-node1}"
    api "$node" "wallets/create" -X POST -H 'Content-Type: application/json' -d '{}' | jq '.data' 2>/dev/null | json
}

cmd_channels() {
    local node="${1:-node1}"
    api "$node" "channels" | jq '.data' 2>/dev/null | json
}

cmd_channel_create() {
    local id="$1"
    local node="${2:-node1}"
    api "$node" "channels" -X POST -H 'Content-Type: application/json' \
        -d "{\"channel_id\":\"$id\"}" | json
}

cmd_orgs() {
    local node="${1:-node1}"
    api "$node" "store/organizations" | jq '.data' 2>/dev/null | json
}

cmd_logs() {
    local node="$1"
    local lines="${2:-50}"
    $COMPOSE logs --tail "$lines" "$node" 2>&1 | grep -v "level=warning"
}

cmd_restart() {
    local node="${1:-}"
    if [[ -z "$node" ]]; then
        echo "Restarting all nodes..."
        $COMPOSE restart node1 node2 node3 orderer1
    else
        echo "Restarting $node..."
        $COMPOSE restart "$node"
    fi
}

cmd_metrics() {
    local node="${1:-node1}"
    $CURL "$(url "$node")/metrics" 2>/dev/null | head -50
}

cmd_verify() {
    local node="${1:-node1}"
    api "$node" "chain/verify" | jq '.data' 2>/dev/null | json
}

cmd_consistency() {
    echo "Chain consistency check:"
    echo ""
    local hashes=()
    local counts=()

    for node in "${PEERS[@]}"; do
        local stats
        stats=$(api "$node" "stats" 2>/dev/null)
        local hash count
        hash=$(echo "$stats" | jq -r '.data.blockchain.latest_block_hash // "?"' 2>/dev/null)
        count=$(echo "$stats" | jq -r '.data.blockchain.block_count // "?"' 2>/dev/null)
        hashes+=("$hash")
        counts+=("$count")
        printf "  %-10s blocks=%-6s hash=%s\n" "$node" "$count" "${hash:0:20}..."
    done

    echo ""
    if [[ "${hashes[0]}" == "${hashes[1]}" && "${hashes[1]}" == "${hashes[2]}" ]]; then
        echo "  \033[32mCONSISTENT\033[0m — all peers agree on latest block"
    else
        echo "  \033[31mINCONSISTENT\033[0m — peers have different chain tips!"
        echo "  This may indicate a fork or propagation delay."
    fi
}

cmd_env() {
    echo "Network Configuration:"
    echo ""
    for node in "${ALL_NODES[@]}"; do
        local port
        port=$(port_for "$node")
        local p2p
        p2p=$(p2p_port_for "$node")
        printf "  %-12s API=https://localhost:%-6s P2P=:%s\n" "$node" "$port" "$p2p"
    done
    echo ""
    echo "  Prometheus:  http://localhost:9090"
    echo "  Grafana:     http://localhost:3000 (admin/admin)"
    echo ""
    echo "  Network:     local-test"
    echo "  Storage:     RocksDB"
    echo "  TLS:         Enabled (self-signed)"
}

# ── Dispatch ─────────────────────────────────────────────────────────────────

cmd="${1:-status}"
shift || true

case "$cmd" in
    status)       cmd_status ;;
    peers)        cmd_peers ;;
    blocks)       cmd_blocks "$@" ;;
    mine)         cmd_mine "$@" ;;
    wallet)
        subcmd="${1:-}"
        shift || true
        case "$subcmd" in
            create) cmd_wallet_create "$@" ;;
            *)      echo "Usage: bcctl wallet create [node]"; exit 1 ;;
        esac
        ;;
    channels)     cmd_channels "$@" ;;
    channel)
        subcmd="${1:-}"
        shift || true
        case "$subcmd" in
            create) cmd_channel_create "$@" ;;
            *)      echo "Usage: bcctl channel create <id>"; exit 1 ;;
        esac
        ;;
    orgs)         cmd_orgs "$@" ;;
    logs)         cmd_logs "$@" ;;
    restart)      cmd_restart "$@" ;;
    metrics)      cmd_metrics "$@" ;;
    verify)       cmd_verify "$@" ;;
    consistency)  cmd_consistency ;;
    env)          cmd_env ;;
    help|--help|-h)
        head -15 "$0" | grep '^#' | sed 's/^# \?//'
        ;;
    *)
        echo "Unknown command: $cmd"
        echo "Run: bcctl help"
        exit 1
        ;;
esac
