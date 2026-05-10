#!/usr/bin/env bash
# Cerulean Ledger — Sandbox Seed Data (Enterprise Edition)
#
# Pre-loads the sandbox node with rich demo data:
# - Multiple organizations and channels
# - Wallets with inter-wallet transfers
# - Identities and verifiable credentials
# - Oracle feeds (activated via ORACLE_DEMO)
# - Governance proposals with votes
# - Channel-isolated state
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
H=(-H "Content-Type: application/json" -H "X-Org-Id: org1" -H "X-Msp-Role: admin")
H2=(-H "Content-Type: application/json" -H "X-Org-Id: org2" -H "X-Msp-Role: admin")

echo -e "${CYAN}╔═══════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║   Cerulean Sandbox Seeder (Enterprise)    ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════╝${NC}"
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

# ── Organizations ──────────────────────────────────────────────────────────────

echo -e "${CYAN}2. Registering organizations${NC}"

for org in '{"org_id":"org1","name":"Universidad de Chile","msp_id":"Org1MSP"}' \
           '{"org_id":"org2","name":"Banco Central","msp_id":"Org2MSP"}'; do
    org_id=$(echo "$org" | grep -o '"org_id":"[^"]*"' | cut -d'"' -f4)
    resp=$(curl -s -X POST "$API/organizations" "${H[@]}" -d "$org" 2>/dev/null || echo "")
    if echo "$resp" | grep -q "success.*true\|already"; then
        ok "Organization: $org_id"
    else
        skip "Organization: $org_id (may exist)"
    fi
done

# ── Channels ──────────────────────────────────────────────────────────────────

echo -e "${CYAN}3. Creating channels${NC}"

for ch in "academic" "financial"; do
    resp=$(curl -s -X POST "$API/channels" "${H[@]}" \
        -d "{\"channel_id\":\"$ch\",\"org_ids\":[\"org1\",\"org2\"]}" 2>/dev/null || echo "")
    if echo "$resp" | grep -q "success\|created\|exists"; then
        ok "Channel: $ch"
    else
        skip "Channel: $ch"
    fi
done

# ── Wallets ──────────────────────────────────────────────────────────────────

echo -e "${CYAN}4. Creating wallets${NC}"

declare -A WALLET_ADDRS
WALLETS=("alice" "bob" "carlos" "diana" "sandbox-miner" "banco-central" "tesoreria")
for name in "${WALLETS[@]}"; do
    resp=$(curl -s -X POST "$API/wallets/create" "${H[@]}" \
        -d "{\"owner\": \"$name\"}" 2>/dev/null || echo "")
    if echo "$resp" | grep -q '"success":true'; then
        addr=$(echo "$resp" | grep -o '"address":"[^"]*"' | head -1 | cut -d'"' -f4)
        ok "Wallet: $name ($addr)"
        WALLET_ADDRS[$name]="$addr"
    else
        skip "Wallet: $name (may already exist)"
        # Try to extract from list
        if [[ "$name" == "sandbox-miner" ]]; then
            wallets=$(curl -s "$API/wallets" "${H[@]}" 2>/dev/null || echo "")
            WALLET_ADDRS[$name]=$(echo "$wallets" | grep -o '"address":"[^"]*"' | head -1 | cut -d'"' -f4)
        fi
    fi
done

MINER_ADDR="${WALLET_ADDRS[sandbox-miner]:-}"

# ── Mine blocks ──────────────────────────────────────────────────────────────

echo -e "${CYAN}5. Mining demo blocks${NC}"

if [[ -z "$MINER_ADDR" ]]; then
    skip "No miner address available — skipping mining"
else
    for i in $(seq 1 8); do
        resp=$(curl -s -X POST "$API/mine" "${H[@]}" \
            -d "{\"miner_address\": \"$MINER_ADDR\"}" 2>/dev/null || echo "")
        if echo "$resp" | grep -q '"success":true'; then
            ok "Block $i mined"
        else
            fail "Block $i"
        fi
    done
fi

# ── Transfers ─────────────────────────────────────────────────────────────────

echo -e "${CYAN}6. Wallet transfers${NC}"

ALICE="${WALLET_ADDRS[alice]:-}"
BOB="${WALLET_ADDRS[bob]:-}"
CARLOS="${WALLET_ADDRS[carlos]:-}"

if [[ -n "$ALICE" && -n "$BOB" ]]; then
    for i in 1 2 3; do
        resp=$(curl -s -X POST "$API/transactions" "${H[@]}" \
            -d "{\"from\":\"$ALICE\",\"to\":\"$BOB\",\"amount\":$((i * 10)),\"fee\":1}" 2>/dev/null || echo "")
        if echo "$resp" | grep -q "success\|accepted\|queued"; then
            ok "Transfer: alice → bob ($((i*10)) NOTA)"
        else
            skip "Transfer $i"
        fi
    done
fi

if [[ -n "$BOB" && -n "$CARLOS" ]]; then
    resp=$(curl -s -X POST "$API/transactions" "${H[@]}" \
        -d "{\"from\":\"$BOB\",\"to\":\"$CARLOS\",\"amount\":5,\"fee\":1}" 2>/dev/null || echo "")
    if echo "$resp" | grep -q "success\|accepted\|queued"; then
        ok "Transfer: bob → carlos (5 NOTA)"
    else
        skip "Transfer bob→carlos"
    fi
fi

# ── Identities (DIDs) ───────────────────────────────────────────────────────

echo -e "${CYAN}7. Creating identities${NC}"

IDENTITIES=("universidad-chile" "registro-civil" "servicio-salud" "banco-central-did" "alice-persona" "bob-empresa" "carlos-auditor")
for name in "${IDENTITIES[@]}"; do
    resp=$(curl -sf -X POST "$API/identity/create" "${H[@]}" \
        -d "{\"name\": \"$name\"}" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Identity: $name"
    else
        skip "Identity: $name"
    fi
done

# ── Credentials ──────────────────────────────────────────────────────────────

echo -e "${CYAN}8. Issuing credentials${NC}"

CREDS=(
    '{"id":"cred-titulo-001","issuer_did":"did:cerulean:universidad-chile","subject_did":"did:cerulean:alice-persona","cred_type":"TituloProfesional","issued_at":1715200000,"expires_at":1746736000,"claims":{"carrera":"Ingenieria Civil Informatica","universidad":"Universidad de Chile","anno":"2024"},"signature":"demo-sig-001"}'
    '{"id":"cred-salud-001","issuer_did":"did:cerulean:servicio-salud","subject_did":"did:cerulean:alice-persona","cred_type":"CertificadoVacunacion","issued_at":1715200000,"expires_at":1746736000,"claims":{"vacuna":"COVID-19","dosis":"3","fecha":"2024-03-15"},"signature":"demo-sig-002"}'
    '{"id":"cred-empresa-001","issuer_did":"did:cerulean:registro-civil","subject_did":"did:cerulean:bob-empresa","cred_type":"RegistroEmpresa","issued_at":1715200000,"expires_at":1746736000,"claims":{"rut":"76.123.456-7","razon_social":"Innovaciones Blockchain SpA","tipo":"SpA"},"signature":"demo-sig-003"}'
    '{"id":"cred-audit-001","issuer_did":"did:cerulean:banco-central-did","subject_did":"did:cerulean:carlos-auditor","cred_type":"LicenciaAuditoria","issued_at":1715200000,"expires_at":1746736000,"claims":{"tipo":"auditor_externo","nivel":"senior","registro":"AUD-2024-789"},"signature":"demo-sig-004"}'
    '{"id":"cred-kyc-001","issuer_did":"did:cerulean:banco-central-did","subject_did":"did:cerulean:bob-empresa","cred_type":"KYC","issued_at":1715200000,"expires_at":1746736000,"claims":{"nivel":"enhanced","pep":false,"aml_score":"low"},"signature":"demo-sig-005"}'
)

for cred in "${CREDS[@]}"; do
    cred_id=$(echo "$cred" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    resp=$(curl -sf -X POST "$API/store/credentials" "${H[@]}" \
        -d "$cred" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Credential: $cred_id"
    else
        skip "Credential: $cred_id"
    fi
done

# ── Governance proposals + votes ─────────────────────────────────────────────

echo -e "${CYAN}9. Governance demo (proposals + votes)${NC}"

# Proposal 1: block time change
resp=$(curl -sf -X POST "$API/governance/proposals" "${H[@]}" \
    -d '{
        "proposer": "alice-persona",
        "action": {"ParamChange": {"changes": [["block_time_ms", {"Integer": 5000}]]}},
        "description": "Reducir tiempo de bloque a 5 segundos para mejorar UX",
        "deposit": 0
    }' 2>/dev/null || echo "")
if [[ -n "$resp" ]]; then
    ok "Proposal 1: Block time reduction"
else
    skip "Proposal 1"
fi

# Proposal 2: PQC enforcement
resp=$(curl -sf -X POST "$API/governance/proposals" "${H[@]}" \
    -d '{
        "proposer": "carlos-auditor",
        "action": {"ParamChange": {"changes": [["require_pqc_signatures", {"Bool": true}]]}},
        "description": "Activar firma post-cuantica obligatoria para todos los validadores",
        "deposit": 0
    }' 2>/dev/null || echo "")
if [[ -n "$resp" ]]; then
    ok "Proposal 2: PQC enforcement"
else
    skip "Proposal 2"
fi

# Vote on proposal 1
for voter in "alice-persona" "bob-empresa" "carlos-auditor"; do
    resp=$(curl -sf -X POST "$API/governance/proposals/1/vote" "${H[@]}" \
        -d "{\"voter\":\"$voter\",\"option\":\"Yes\",\"power\":1000}" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then
        ok "Vote: $voter → Yes on proposal 1"
    else
        skip "Vote: $voter"
    fi
done

# ── Channel state (isolated world state demo) ────────────────────────────────

echo -e "${CYAN}10. Channel world state${NC}"

# Write to academic channel
resp=$(curl -sf -X POST "$API/channels/academic/state" "${H[@]}" \
    -d '{"key":"enrollment:2024:alice","value":"active"}' 2>/dev/null || echo "")
if [[ -n "$resp" ]]; then ok "Channel state: academic/enrollment:alice"; else skip "academic state"; fi

resp=$(curl -sf -X POST "$API/channels/financial/state" "${H2[@]}" \
    -d '{"key":"balance:org2:reserve","value":"50000000"}' 2>/dev/null || echo "")
if [[ -n "$resp" ]]; then ok "Channel state: financial/balance:org2"; else skip "financial state"; fi

# ── Contact form entry ───────────────────────────────────────────────────────

echo -e "${CYAN}11. Sample contact entries${NC}"

CONTACTS=(
    '{"name":"Demo Visitor","email":"demo@cerulean.cl","org":"Cerulean Labs","message":"Interesados en piloto para trazabilidad de documentos academicos."}'
    '{"name":"Maria Fernandez","email":"mfernandez@bancocentral.cl","org":"Banco Central","message":"Consulta sobre integracion con sistema RTGS via ISO 20022."}'
)

for contact in "${CONTACTS[@]}"; do
    name=$(echo "$contact" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
    resp=$(curl -sf -X POST "$API/contact" "${H[@]}" -d "$contact" 2>/dev/null || echo "")
    if [[ -n "$resp" ]]; then ok "Contact: $name"; else skip "Contact: $name"; fi
done

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║         Seed Complete                     ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════╝${NC}"
echo ""

# Show final stats
HEALTH=$(curl -sf "$API/health" 2>/dev/null || echo "{}")
BLOCKS=$(echo "$HEALTH" | grep -o '"block_height":[0-9]*' | cut -d: -f2 || echo "?")
VERSION=$(echo "$HEALTH" | grep -o '"version":"[^"]*"' | cut -d'"' -f4 || echo "?")
echo -e "  Version:      ${CYAN}${VERSION:-?}${NC}"
echo -e "  Blocks:       ${CYAN}${BLOCKS:-?}${NC}"
echo -e "  Wallets:      ${CYAN}${#WALLETS[@]}${NC}"
echo -e "  Identities:   ${CYAN}${#IDENTITIES[@]}${NC}"
echo -e "  Credentials:  ${CYAN}${#CREDS[@]}${NC}"
echo -e "  Orgs:         ${CYAN}2${NC} (org1, org2)"
echo -e "  Channels:     ${CYAN}2${NC} (academic, financial)"
echo ""
echo -e "  Explorer:     ${CYAN}http://localhost:5173${NC}"
echo -e "  Voto:         ${CYAN}http://localhost:5174${NC}"
echo -e "  API:          ${CYAN}$API${NC}"
echo -e "  Prometheus:   ${CYAN}http://localhost:9090${NC}"
echo -e "  Grafana:      ${CYAN}http://localhost:3000${NC} (admin/admin)"
