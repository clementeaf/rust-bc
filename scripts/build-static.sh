#!/usr/bin/env bash
# Build a fully static cerulean-node binary (musl-linked).
#
# Usage:
#   ./scripts/build-static.sh              # Build image + extract binary
#   ./scripts/build-static.sh --deploy     # Build + deploy to EC2 via SCP
#
# Output: ./dist/cerulean-node-linux-amd64
#
# The binary has ZERO runtime dependencies — runs on any Linux.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST="$REPO_ROOT/dist"
BINARY="cerulean-node-linux-amd64"
IMAGE="cerulean-node:static"

cd "$REPO_ROOT"

echo "=== Building static binary (musl) ==="
docker build -f Dockerfile.static -t "$IMAGE" .

echo "=== Extracting binary ==="
mkdir -p "$DIST"
CONTAINER_ID=$(docker create "$IMAGE")
docker cp "$CONTAINER_ID:/usr/local/bin/cerulean-node" "$DIST/$BINARY"
docker rm "$CONTAINER_ID" > /dev/null

chmod +x "$DIST/$BINARY"
SIZE=$(du -h "$DIST/$BINARY" | cut -f1)
echo "=== Built: $DIST/$BINARY ($SIZE) ==="

# Verify it's static
file "$DIST/$BINARY"

if [[ "${1:-}" == "--deploy" ]]; then
    # Load deploy config
    EC2_KEY="${EC2_KEY:-$HOME/.ssh/rust-bc-test.pem}"
    EC2_USER="${EC2_USER:-ec2-user}"
    EC2_HOST="${EC2_HOST:-52.91.18.180}"

    echo ""
    echo "=== Deploying to $EC2_HOST ==="
    scp -i "$EC2_KEY" -o StrictHostKeyChecking=no \
        "$DIST/$BINARY" "$EC2_USER@$EC2_HOST:~/cerulean-node"

    ssh -i "$EC2_KEY" -o StrictHostKeyChecking=no "$EC2_USER@$EC2_HOST" \
        "chmod +x ~/cerulean-node && echo 'Deployed: \$(~/cerulean-node --version 2>/dev/null || echo ok)'"

    echo "=== Deploy complete ==="
fi
