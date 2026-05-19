#!/usr/bin/env bash
# Install Cerulean Ledger node — single binary, zero dependencies.
#
# Usage:
#   curl -sL https://ceruleanledger-releases.s3.amazonaws.com/install.sh | bash
#   # or
#   ./install-cerulean.sh
#
# What it does:
#   1. Downloads the binary from S3
#   2. Creates a systemd service
#   3. Starts the node
#
# Requirements: Linux x86_64, curl, systemd

set -euo pipefail

S3_URL="https://ceruleanledger-releases.s3.amazonaws.com/releases/latest/cerulean-node-linux-amd64"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/cerulean"
SERVICE_USER="cerulean"
BINARY="cerulean-node"

echo "=== Cerulean Ledger Installer ==="

# Check arch
ARCH=$(uname -m)
if [[ "$ARCH" != "x86_64" ]]; then
    echo "ERROR: Only x86_64 supported. Got: $ARCH"
    exit 1
fi

# Check root
if [[ $EUID -ne 0 ]]; then
    echo "Run as root: sudo $0"
    exit 1
fi

# Download
echo "Downloading binary..."
curl -fSL "$S3_URL" -o "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"
echo "  Installed: $INSTALL_DIR/$BINARY"

# Create user + data dir
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false -d "$DATA_DIR" "$SERVICE_USER"
fi
mkdir -p "$DATA_DIR"
chown "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"

# Systemd service
cat > /etc/systemd/system/cerulean-node.service <<EOF
[Unit]
Description=Cerulean Ledger Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
ExecStart=$INSTALL_DIR/$BINARY
WorkingDirectory=$DATA_DIR
Restart=on-failure
RestartSec=5

# Environment — customize in /etc/cerulean/node.env
EnvironmentFile=-/etc/cerulean/node.env

# Defaults
Environment=BIND_ADDR=0.0.0.0
Environment=API_PORT=8080
Environment=P2P_PORT=8081
Environment=STORAGE_BACKEND=rocksdb
Environment=STORAGE_PATH=/var/lib/cerulean/rocksdb
Environment=ACL_MODE=permissive
Environment=SIGNING_ALGORITHM=ml-dsa-65
Environment=RUST_LOG=info

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR

[Install]
WantedBy=multi-user.target
EOF

# Config directory
mkdir -p /etc/cerulean
if [[ ! -f /etc/cerulean/node.env ]]; then
    cat > /etc/cerulean/node.env <<'ENVEOF'
# Cerulean Ledger configuration
# Uncomment and edit as needed.
#
# NETWORK_ID=mainnet
# ORG_ID=my-org
# ACL_MODE=strict
# BOOTSTRAP_NODES=10.0.1.10:8081,10.0.1.11:8081
# VAULT_RECOVERY_SECRET=<generate with: openssl rand -hex 32>
ENVEOF
    echo "  Config: /etc/cerulean/node.env"
fi

# Enable + start
systemctl daemon-reload
systemctl enable cerulean-node
systemctl start cerulean-node

echo ""
echo "=== Cerulean Ledger installed ==="
echo "  Status:  systemctl status cerulean-node"
echo "  Logs:    journalctl -u cerulean-node -f"
echo "  Config:  /etc/cerulean/node.env"
echo "  Data:    $DATA_DIR"
echo "  API:     http://localhost:8080/api/v1/health"
