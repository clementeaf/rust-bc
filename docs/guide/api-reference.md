# API Reference

All endpoints use the `/api/v1` prefix. Responses follow a standard envelope:

```json
{
  "status": "ok",
  "status_code": 200,
  "message": "...",
  "data": { ... },
  "timestamp": "2026-04-07T...",
  "trace_id": "uuid"
}
```

## Headers

| Header | Required | Description |
|--------|----------|-------------|
| `X-Org-Id` | For mutations | Organization ID of the caller |
| `X-Msp-Role` | Optional | MSP role: `admin`, `client`, `peer`, `orderer`, `member` |
| `X-Channel-Id` | Optional | Channel context (defaults to `"default"`) |
| `Content-Type` | For POST/PUT | `application/json` (or `application/octet-stream` for chaincode install) |

## Gateway

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/gateway/submit` | `peer/Propose` | Submit transaction (endorse + order + commit) |

**Body:**
```json
{
  "chaincode_id": "mycc",
  "channel_id": "mychannel",
  "transaction": {
    "id": "tx-001",
    "input_did": "did:bc:alice",
    "output_recipient": "did:bc:bob",
    "amount": 100
  }
}
```

**Response:** `{ "tx_id": "tx-001", "block_height": 1, "valid": true }`

## Organizations

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/store/organizations` | `peer/Admin` | Register an organization |
| GET | `/store/organizations` | — | List all organizations |
| GET | `/store/organizations/{org_id}` | — | Get organization by ID |

## Policies

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/store/policies` | `peer/Admin` | Set endorsement policy |
| GET | `/store/policies/{resource_id}` | — | Get policy |

**Policy types:** `AnyOf`, `AllOf`, `NOutOf`, `And`, `Or`, `OuBased`

## Channels

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/channels` | `peer/ChannelConfig` | Create channel |
| POST | `/channels/{id}/config` | `peer/ChannelConfig` | Update channel config |
| GET | `/channels/{id}/config` | — | Get current config |
| GET | `/channels/{id}/config/history` | — | Get config version history |
| GET | `/channels` | — | List channels |

## Chaincode

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/chaincode/install?chaincode_id=...&version=...` | `peer/ChaincodeToChaincode` | Install Wasm module |
| POST | `/chaincode/{id}/approve?version=...` | `peer/ChaincodeToChaincode` | Approve as org (requires `X-Org-Id`) |
| POST | `/chaincode/{id}/commit?version=...` | `peer/ChaincodeToChaincode` | Commit definition |
| POST | `/chaincode/{id}/simulate?version=...` | `peer/ChaincodeToChaincode` | Simulate execution |

## Private Data

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| PUT | `/private-data/{collection}/{key}` | `peer/PrivateData.Write` | Store private data (requires `X-Org-Id`) |
| GET | `/private-data/{collection}/{key}` | — | Retrieve private data (requires `X-Org-Id`) |
| POST | `/private-data/collections` | — | Register collection |

## Block Store

| Method | Path | Description |
|--------|------|-------------|
| GET | `/store/blocks?offset=0&limit=20` | List blocks (paginated) |
| GET | `/store/blocks/latest` | Get latest block height |
| GET | `/store/blocks/{height}` | Get block by height |
| GET | `/store/blocks/{height}/transactions` | Get transactions in block |

## Transaction Store

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/store/transactions` | `peer/Propose` | Write transaction |
| GET | `/store/transactions/{tx_id}` | — | Read transaction |

## Identity Store

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/store/identities` | `peer/Identity` | Write identity record |
| GET | `/store/identities/{did}` | — | Read identity |

## Credentials Store

| Method | Path | Description |
|--------|------|-------------|
| POST | `/store/credentials` | Write credential |
| GET | `/store/credentials/{id}` | Read credential |
| GET | `/store/credentials/by-subject/{did}` | Credentials by subject |

## Discovery

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| GET | `/discovery/endorsers?chaincode=...&channel=...` | — | Get endorser plan |
| GET | `/discovery/peers?channel=...` | — | Get channel peers |
| POST | `/discovery/register` | `peer/Discovery.Admin` | Register peer |

## MSP

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/msp/{msp_id}/revoke` | `peer/MSP.Admin` | Revoke certificate serial |
| GET | `/msp/{msp_id}` | — | Get MSP info + CRL size |

## ACL

| Method | Path | Description |
|--------|------|-------------|
| POST | `/acls` | Set ACL entry |
| GET | `/acls` | List all ACL entries |
| GET | `/acls/{resource}` | Get ACL for resource |

## Events

| Method | Path | Description |
|--------|------|-------------|
| GET | `/events/blocks?from_height=N` | Long-poll for blocks |
| GET | `/events/blocks` | WebSocket stream (upgrade) |

## Snapshots

| Method | Path | ACL | Description |
|--------|------|-----|-------------|
| POST | `/snapshots/{channel_id}` | `qscc/Snapshot.Admin` | Create snapshot |
| GET | `/snapshots/{channel_id}` | — | List snapshots |
| GET | `/snapshots/{channel_id}/{id}` | — | Download snapshot |

## Chain

| Method | Path | Description |
|--------|------|-------------|
| GET | `/chain/verify` | Verify chain integrity |
| GET | `/chain/info` | Get blockchain info |

## Utilities

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/version` | Node version |
| GET | `/openapi.json` | OpenAPI specification |
| GET | `/metrics` | Prometheus metrics |
