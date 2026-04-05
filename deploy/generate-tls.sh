#!/bin/bash
# Generate self-signed TLS certificates for local testing.
# Creates a CA + per-node certs in deploy/tls/

set -euo pipefail
DIR="$(cd "$(dirname "$0")/tls" && pwd)"
mkdir -p "$DIR"

echo "==> Generating CA..."
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
  -days 365 -nodes \
  -keyout "$DIR/ca-key.pem" \
  -out "$DIR/ca-cert.pem" \
  -subj "/CN=rust-bc-ca/O=rust-bc"

for NODE in node1 node2 node3 orderer1; do
  echo "==> Generating cert for $NODE..."

  # CSR
  openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
    -nodes \
    -keyout "$DIR/$NODE-key.pem" \
    -out "$DIR/$NODE.csr" \
    -subj "/CN=$NODE/O=rust-bc"

  # SAN config
  cat > "$DIR/$NODE-ext.cnf" <<EOF
[v3_req]
subjectAltName = DNS:$NODE,DNS:localhost,IP:127.0.0.1
EOF

  # Sign with CA
  openssl x509 -req \
    -in "$DIR/$NODE.csr" \
    -CA "$DIR/ca-cert.pem" \
    -CAkey "$DIR/ca-key.pem" \
    -CAcreateserial \
    -days 365 \
    -extfile "$DIR/$NODE-ext.cnf" \
    -extensions v3_req \
    -out "$DIR/$NODE-cert.pem"

  rm -f "$DIR/$NODE.csr" "$DIR/$NODE-ext.cnf"
done

echo "==> TLS certificates generated in $DIR"
ls -la "$DIR"/*.pem
