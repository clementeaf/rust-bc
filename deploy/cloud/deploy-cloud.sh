#!/usr/bin/env bash
# Cloud deployment script for rust-bc.
# Deploys a 3-node network across VMs via SSH.
#
# Usage: ./deploy-cloud.sh [setup|certs|start|stop|status|logs|bench|destroy]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Load inventory
if [[ ! -f "$SCRIPT_DIR/inventory.env" ]]; then
    echo "ERROR: inventory.env not found. Copy inventory.example.env and edit it."
    exit 1
fi
source "$SCRIPT_DIR/inventory.env"

SSH_OPTS="-o StrictHostKeyChecking=no -o ConnectTimeout=10"
[[ -n "${SSH_KEY:-}" ]] && SSH_OPTS="$SSH_OPTS -i $SSH_KEY"

NODES=("$VM1_IP" "$VM2_IP" "$VM3_IP")
USERS=("$VM1_USER" "$VM2_USER" "$VM3_USER")
NAMES=("node1" "node2" "orderer1")
ORGS=("org1" "org2" "orderer")

ssh_run() {
    local idx=$1; shift
    ssh $SSH_OPTS "${USERS[$idx]}@${NODES[$idx]}" "$@"
}

scp_to() {
    local idx=$1 src=$2 dst=$3
    scp $SSH_OPTS "$src" "${USERS[$idx]}@${NODES[$idx]}:$dst"
}

case "${1:-help}" in
    setup)
        echo "=== Installing Docker on all VMs ==="
        for i in 0 1 2; do
            echo "  [${NAMES[$i]}] ${NODES[$i]}..."
            ssh_run $i "command -v docker >/dev/null 2>&1 || (curl -fsSL https://get.docker.com | sh && sudo usermod -aG docker \$USER)" || true
            ssh_run $i "mkdir -p ~/rust-bc"
        done

        echo "=== Building Docker image ==="
        cd "$REPO_ROOT"
        docker build -t rust-bc:latest .
        docker save rust-bc:latest | gzip > /tmp/rust-bc-image.tar.gz
        echo "  Image saved ($(du -h /tmp/rust-bc-image.tar.gz | cut -f1))"

        echo "=== Distributing image to VMs ==="
        for i in 0 1 2; do
            echo "  [${NAMES[$i]}] uploading..."
            scp_to $i /tmp/rust-bc-image.tar.gz "~/rust-bc/image.tar.gz"
            ssh_run $i "docker load < ~/rust-bc/image.tar.gz"
        done
        rm -f /tmp/rust-bc-image.tar.gz
        echo "=== Setup complete ==="
        ;;

    certs)
        echo "=== Generating TLS certificates ==="
        cd "$REPO_ROOT/deploy"
        bash generate-tls.sh

        echo "=== Distributing certs ==="
        for i in 0 1 2; do
            echo "  [${NAMES[$i]}]..."
            ssh_run $i "mkdir -p ~/rust-bc/tls"
            scp_to $i "$REPO_ROOT/deploy/tls/ca.crt" "~/rust-bc/tls/ca.crt"
            scp_to $i "$REPO_ROOT/deploy/tls/${NAMES[$i]}.crt" "~/rust-bc/tls/node.crt"
            scp_to $i "$REPO_ROOT/deploy/tls/${NAMES[$i]}.key" "~/rust-bc/tls/node.key"
        done
        echo "=== Certs distributed ==="
        ;;

    start)
        echo "=== Starting nodes ==="
        for i in 0 1 2; do
            # Build bootstrap list (all other nodes)
            bootstrap=""
            for j in 0 1 2; do
                [[ $j -eq $i ]] && continue
                [[ -n "$bootstrap" ]] && bootstrap="$bootstrap,"
                bootstrap="${bootstrap}${NODES[$j]}:8081"
            done

            echo "  [${NAMES[$i]}] starting on ${NODES[$i]}..."
            ssh_run $i "docker rm -f rust-bc 2>/dev/null || true"
            ssh_run $i "docker run -d --name rust-bc --restart unless-stopped \
                -p 8080:8080 -p 8081:8081 \
                -v ~/rust-bc/tls:/tls:ro \
                -v ~/rust-bc/data:/app/data \
                -e BIND_ADDR=0.0.0.0 \
                -e API_PORT=8080 \
                -e P2P_PORT=8081 \
                -e STORAGE_BACKEND=rocksdb \
                -e STORAGE_PATH=/app/data/rocksdb \
                -e NETWORK_ID=${NETWORK_ID} \
                -e ORG_ID=${ORGS[$i]} \
                -e ACL_MODE=strict \
                -e BOOTSTRAP_NODES=$bootstrap \
                -e P2P_EXTERNAL_ADDRESS=${NODES[$i]}:8081 \
                -e SIGNING_ALGORITHM=${SIGNING_ALGORITHM} \
                -e TLS_CERT_PATH=/tls/node.crt \
                -e TLS_KEY_PATH=/tls/node.key \
                -e TLS_CA_CERT_PATH=/tls/ca.crt \
                -e RATE_LIMIT_PER_SECOND=100 \
                -e RATE_LIMIT_PER_MINUTE=3000 \
                -e CHECKPOINT_HMAC_SECRET=$(openssl rand -hex 16) \
                rust-bc:latest"
        done
        echo "=== Waiting for health checks... ==="
        sleep 15
        $0 status
        ;;

    stop)
        echo "=== Stopping all nodes ==="
        for i in 0 1 2; do
            echo "  [${NAMES[$i]}]..."
            ssh_run $i "docker stop rust-bc 2>/dev/null || true"
        done
        ;;

    status)
        echo "=== Node status ==="
        for i in 0 1 2; do
            health=$(curl -sk --max-time 5 "https://${NODES[$i]}:8080/api/v1/health" 2>/dev/null | jq -r '.data.status // "unreachable"')
            blocks=$(curl -sk --max-time 5 "https://${NODES[$i]}:8080/api/v1/stats" 2>/dev/null | jq -r '.data.blockchain.block_count // "?"')
            peers=$(curl -sk --max-time 5 "https://${NODES[$i]}:8080/api/v1/stats" 2>/dev/null | jq -r '.data.network.connected_peers // "?"')
            printf "  %-10s %-16s health=%-10s blocks=%-5s peers=%s\n" "${NAMES[$i]}" "${NODES[$i]}" "$health" "$blocks" "$peers"
        done
        ;;

    logs)
        echo "=== Recent logs (last 30 lines per node) ==="
        for i in 0 1 2; do
            echo ""
            echo "--- ${NAMES[$i]} (${NODES[$i]}) ---"
            ssh_run $i "docker logs rust-bc --tail 30 2>&1" || true
        done
        ;;

    bench)
        echo "=== Running load test against ${NODES[0]} ==="
        "$REPO_ROOT/scripts/load-test.sh" --node "https://${NODES[0]}:8080" --duration 120 --rate 200
        ;;

    destroy)
        echo "=== Destroying all nodes and data ==="
        for i in 0 1 2; do
            echo "  [${NAMES[$i]}]..."
            ssh_run $i "docker rm -f rust-bc 2>/dev/null || true; rm -rf ~/rust-bc/data"
        done
        echo "=== Destroyed ==="
        ;;

    *)
        echo "Usage: $0 {setup|certs|start|stop|status|logs|bench|destroy}"
        exit 1
        ;;
esac
