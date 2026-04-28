"""Pydantic models for rust-bc API request/response types."""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field


# ── Gateway ───────────────────────────────────────────────────────────────────


class TransactionInput(BaseModel):
    id: str
    input_did: str
    output_recipient: str
    amount: int


class GatewaySubmitRequest(BaseModel):
    chaincode_id: str
    channel_id: str = ""
    transaction: TransactionInput


class GatewaySubmitResponse(BaseModel):
    tx_id: str
    block_height: int
    valid: bool | None = None


# ── Chaincode ─────────────────────────────────────────────────────────────────


class KVRead(BaseModel):
    key: str
    version: int


class KVWrite(BaseModel):
    key: str
    value: str


class ReadWriteSet(BaseModel):
    reads: list[KVRead] = Field(default_factory=list)
    writes: list[KVWrite] = Field(default_factory=list)


class SimulateResponse(BaseModel):
    result: str = ""
    rwset: ReadWriteSet = Field(default_factory=ReadWriteSet)


# ── Organizations ─────────────────────────────────────────────────────────────


class Organization(BaseModel):
    org_id: str
    name: str = ""
    msp_id: str = ""


# ── Endorsement policies ─────────────────────────────────────────────────────


class EndorsementPolicy(BaseModel):
    """Flexible policy model — pass the variant directly as a dict.

    Examples:
        EndorsementPolicy(AnyOf=["org1", "org2"])
        EndorsementPolicy(NOutOf={"n": 2, "orgs": ["org1", "org2"]})
    """

    model_config = {"extra": "allow"}


# ── Channels ──────────────────────────────────────────────────────────────────


class ChannelInfo(BaseModel):
    channel_id: str


# ── Private data ──────────────────────────────────────────────────────────────


class PrivateDataWriteResponse(BaseModel):
    collection: str
    key: str
    hash: str


# ── Health ────────────────────────────────────────────────────────────────────


class HealthChecks(BaseModel):
    storage: str = ""
    peers: str = ""
    ordering: str = ""


class BlockchainHealth(BaseModel):
    height: int = 0
    last_block_hash: str = ""
    validators_count: int = 0


class HealthCheck(BaseModel):
    status: str
    uptime_seconds: int = 0
    blockchain: BlockchainHealth = Field(default_factory=BlockchainHealth)
    checks: HealthChecks | None = None


# ── API envelope ──────────────────────────────────────────────────────────────


class ApiResponse(BaseModel):
    status: str = ""
    status_code: int = 0
    message: str = ""
    data: Any = None
    error: Any = None
    timestamp: str = ""
    trace_id: str = ""
