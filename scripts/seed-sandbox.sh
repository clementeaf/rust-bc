#!/usr/bin/env bash
# Cerulean Ledger — Sandbox Seed Data
#
# Pre-loads the sandbox node with demo data so the explorer
# and voting apps have content on first visit.
#
# Usage:
#   ./scripts/seed-sandbox.sh              # Uses localhost:9600
#   ./scripts/seed-sandbox.sh <base-url>   # Custom API base URL
#
# Idempotent: safe to run multiple times.

set -euo pipefail

BASE="${1:-http://localhost:9600}"
API="$BASE/api/v1"

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

ok()   { echo -e "  ${GREEN}[OK]${NC} $1"; }
fail() { echo -e "  ${RED}[FAIL]${NC} $1"; }
skip() { echo -e "  ${YELLOW}[SKIP]${NC} $1"; }

# Headers for permissive ACL mode
HEADERS=(-H "Content-Type: application/json" -H "X-Org-Id: sandbox" -H "X-Msp-Role: Admin")

echo -e "${CYAN}=== Cerulean Sandbox Seeder ===${NC}"
echo -e "  Target: ${CYAN}$API${NC}"
echo ""

# ── Health check ─────────────────────────────────────────────────────────────

echo -e "${CYAN}1. Health check${NC}"
if curl -sf "$API/health" > /dev/null 2>&1; then
    ok "Node is healthy"
else
    fail "Node not reachable at $API/health"
    exit 1
fi

# ── Wallets ──────────────────────────────────────────────────────────────────

echo -e "${CYAN}2. Creating wallets${NC}"

WALLETS=("alice" "bob" "carlos" "diana" "sandbox-miner")
for name in "${WALLETS[@]}"; do
    resp=$(curl -sf -X POST "$API/wallets/create" "${HEADERS[@]}" \
        -d "{\"owner\": \"$name\"}" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Wallet: $name"
    else
        skip "Wallet: $name (may already exist)"
    fi
done

# ── Mine blocks ──────────────────────────────────────────────────────────────

echo -e "${CYAN}3. Mining demo blocks${NC}"

for i in $(seq 1 5); do
    resp=$(curl -sf -X POST "$API/blocks/mine" "${HEADERS[@]}" \
        -d '{"miner_address": "sandbox-miner"}' 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Block $i mined"
    else
        fail "Block $i failed"
    fi
done

# ── Identities (DIDs) ───────────────────────────────────────────────────────

echo -e "${CYAN}4. Creating identities${NC}"

IDENTITIES=("universidad-chile" "registro-civil" "servicio-salud" "alice-persona" "bob-empresa")
for name in "${IDENTITIES[@]}"; do
    resp=$(curl -sf -X POST "$API/identity/create" "${HEADERS[@]}" \
        -d "{\"name\": \"$name\"}" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Identity: $name"
    else
        skip "Identity: $name (may already exist)"
    fi
done

# ── Credentials ──────────────────────────────────────────────────────────────

echo -e "${CYAN}5. Issuing credentials${NC}"

# Store credentials directly
CREDS=(
    '{"id":"cred-titulo-001","issuer_did":"did:cerulean:universidad-chile","subject_did":"did:cerulean:alice-persona","cred_type":"TituloProfesional","issued_at":1715200000,"expires_at":1746736000,"claims":{"carrera":"Ingenieria Civil Informatica","universidad":"Universidad de Chile","anno":"2024"},"signature":"demo-signature-hex"}'
    '{"id":"cred-salud-001","issuer_did":"did:cerulean:servicio-salud","subject_did":"did:cerulean:alice-persona","cred_type":"CertificadoVacunacion","issued_at":1715200000,"expires_at":1746736000,"claims":{"vacuna":"COVID-19","dosis":"3","fecha":"2024-03-15"},"signature":"demo-signature-hex"}'
    '{"id":"cred-empresa-001","issuer_did":"did:cerulean:registro-civil","subject_did":"did:cerulean:bob-empresa","cred_type":"RegistroEmpresa","issued_at":1715200000,"expires_at":1746736000,"claims":{"rut":"76.123.456-7","razon_social":"Innovaciones Blockchain SpA","tipo":"SpA"},"signature":"demo-signature-hex"}'
)

for cred in "${CREDS[@]}"; do
    cred_id=$(echo "$cred" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    resp=$(curl -sf -X POST "$API/store/credentials" "${HEADERS[@]}" \
        -d "$cred" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Credential: $cred_id"
    else
        skip "Credential: $cred_id (may already exist)"
    fi
done

# ── Governance proposal ──────────────────────────────────────────────────────

echo -e "${CYAN}6. Creating governance demo${NC}"

resp=$(curl -sf -X POST "$API/governance/proposals" "${HEADERS[@]}" \
    -d '{
        "proposer": "alice-persona",
        "action": {"ParamChange": {"changes": [["block_time_ms", {"Integer": 5000}]]}},
        "description": "Reducir tiempo de bloque a 5 segundos para mejorar UX",
        "deposit": 0
    }' 2>/dev/null || echo "")
if [[ -n "$resp" ]]; then
    ok "Governance proposal created"
else
    skip "Governance proposal (may already exist or rate-limited)"
fi

# ── Contact form entry ───────────────────────────────────────────────────────

echo -e "${CYAN}7. Sample contact entry${NC}"

resp=$(curl -sf -X POST "$API/contact" "${HEADERS[@]}" \
    -d '{
        "name": "Demo Visitor",
        "email": "demo@cerulean.cl",
        "org": "Cerulean Labs",
        "message": "Interesados en piloto para trazabilidad de documentos academicos."
    }' 2>/dev/null || echo "")
if [[ -n "$resp" ]]; then
    ok "Contact entry"
else
    skip "Contact entry"
fi

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo -e "${GREEN}=== Seed complete ===${NC}"
echo ""

# Show final stats
HEALTH=$(curl -sf "$API/health" 2>/dev/null || echo "{}")
BLOCKS=$(echo "$HEALTH" | grep -o '"block_height":[0-9]*' | cut -d: -f2 || echo "?")
echo -e "  Blocks: ${CYAN}${BLOCKS:-?}${NC}"
echo -e "  Wallets: ${CYAN}${#WALLETS[@]}${NC}"
echo -e "  Identities: ${CYAN}${#IDENTITIES[@]}${NC}"
echo -e "  Credentials: ${CYAN}${#CREDS[@]}${NC}"
echo ""
echo -e "  Explorer: ${CYAN}http://localhost:5173${NC}"
echo -e "  Voto: ${CYAN}http://localhost:5174${NC}"
echo -e "  API: ${CYAN}$API${NC}"
