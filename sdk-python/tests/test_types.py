"""Tests for Pydantic models."""

from rust_bc.types import (
    ApiResponse,
    GatewaySubmitResponse,
    HealthCheck,
    Organization,
    TransactionInput,
)


class TestTransactionInput:
    def test_roundtrip(self):
        tx = TransactionInput(
            id="tx-1", input_did="did:bc:alice", output_recipient="did:bc:bob", amount=100
        )
        d = tx.model_dump()
        assert d["id"] == "tx-1"
        assert d["amount"] == 100
        rebuilt = TransactionInput.model_validate(d)
        assert rebuilt == tx


class TestGatewaySubmitResponse:
    def test_parse_with_valid(self):
        r = GatewaySubmitResponse.model_validate(
            {"tx_id": "tx-1", "block_height": 5, "valid": True}
        )
        assert r.valid is True

    def test_parse_without_valid(self):
        r = GatewaySubmitResponse.model_validate({"tx_id": "tx-1", "block_height": 5})
        assert r.valid is None


class TestOrganization:
    def test_minimal(self):
        org = Organization(org_id="org1")
        assert org.name == ""
        assert org.msp_id == ""

    def test_full(self):
        org = Organization(org_id="org1", name="Org 1", msp_id="Org1MSP")
        d = org.model_dump()
        assert d["msp_id"] == "Org1MSP"


class TestHealthCheck:
    def test_parse_with_checks(self):
        h = HealthCheck.model_validate({
            "status": "healthy",
            "uptime_seconds": 60,
            "blockchain": {"height": 5, "last_block_hash": "abc", "validators_count": 0},
            "checks": {"storage": "ok", "peers": "ok", "ordering": "ok"},
        })
        assert h.status == "healthy"
        assert h.checks is not None
        assert h.checks.storage == "ok"

    def test_parse_without_checks(self):
        h = HealthCheck.model_validate({"status": "healthy"})
        assert h.checks is None


class TestApiResponse:
    def test_parse_envelope(self):
        r = ApiResponse.model_validate({
            "status": "Success",
            "status_code": 200,
            "message": "OK",
            "data": {"key": "value"},
            "timestamp": "2026-04-07T00:00:00Z",
            "trace_id": "uuid",
        })
        assert r.status_code == 200
        assert r.data == {"key": "value"}
