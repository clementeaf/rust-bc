"""Tests for BlockchainClient using httpx mock transport."""

import pytest
import httpx
from rust_bc import BlockchainClient
from rust_bc.types import TransactionInput, Organization
from rust_bc.exceptions import NotFoundError, ForbiddenError, ValidationError


def _envelope(data, status_code=200):
    """Build a gateway API envelope."""
    return {
        "status": "Success",
        "status_code": status_code,
        "message": "OK",
        "data": data,
        "error": None,
        "timestamp": "2026-04-07T00:00:00Z",
        "trace_id": "test-trace",
    }


class MockTransport(httpx.BaseTransport):
    """Simple mock transport that returns canned responses by path."""

    def __init__(self, routes: dict[str, tuple[int, dict]]) -> None:
        self._routes = routes

    def handle_request(self, request: httpx.Request) -> httpx.Response:
        path = request.url.path
        # Strip base path prefix
        for suffix, (status, body) in self._routes.items():
            if path.endswith(suffix):
                return httpx.Response(status, json=body)
        return httpx.Response(404, json=_envelope(None, 404))


def _make_client(routes: dict[str, tuple[int, dict]]) -> BlockchainClient:
    client = BlockchainClient.__new__(BlockchainClient)
    client._client = httpx.Client(
        base_url="http://test/api/v1",
        transport=MockTransport(routes),
    )
    return client


class TestHealth:
    def test_health_returns_healthy(self):
        client = _make_client({
            "/health": (200, _envelope({
                "status": "healthy",
                "uptime_seconds": 60,
                "blockchain": {"height": 5, "last_block_hash": "abc", "validators_count": 0},
                "checks": {"storage": "ok", "peers": "ok (3 connected)", "ordering": "ok"},
            })),
        })
        health = client.health()
        assert health.status == "healthy"
        assert health.uptime_seconds == 60
        assert health.blockchain.height == 5
        assert health.checks is not None
        assert health.checks.storage == "ok"

    def test_health_degraded(self):
        client = _make_client({
            "/health": (200, _envelope({
                "status": "degraded",
                "uptime_seconds": 10,
                "blockchain": {"height": 0, "last_block_hash": "", "validators_count": 0},
                "checks": {"storage": "unavailable", "peers": "none", "ordering": "ok"},
            })),
        })
        health = client.health()
        assert health.status == "degraded"
        assert health.checks is not None
        assert health.checks.storage == "unavailable"


class TestGateway:
    def test_submit_transaction(self):
        client = _make_client({
            "/gateway/submit": (200, _envelope({
                "tx_id": "tx-001",
                "block_height": 3,
                "valid": True,
            })),
        })
        tx = TransactionInput(
            id="tx-001",
            input_did="did:bc:alice",
            output_recipient="did:bc:bob",
            amount=100,
        )
        result = client.submit_transaction("mycc", "mychannel", tx)
        assert result.tx_id == "tx-001"
        assert result.block_height == 3
        assert result.valid is True


class TestOrganizations:
    def test_register_org(self):
        client = _make_client({
            "/store/organizations": (201, _envelope({
                "org_id": "org1",
                "name": "Organization 1",
                "msp_id": "Org1MSP",
            }, 201)),
        })
        org = Organization(org_id="org1", name="Organization 1", msp_id="Org1MSP")
        result = client.register_org(org)
        assert result.org_id == "org1"
        assert result.msp_id == "Org1MSP"

    def test_list_orgs(self):
        client = _make_client({
            "/store/organizations": (200, _envelope([
                {"org_id": "org1", "name": "Org 1", "msp_id": "Org1MSP"},
                {"org_id": "org2", "name": "Org 2", "msp_id": "Org2MSP"},
            ])),
        })
        orgs = client.list_orgs()
        assert len(orgs) == 2
        assert orgs[0].org_id == "org1"


class TestChannels:
    def test_create_channel(self):
        client = _make_client({
            "/channels": (201, _envelope({"channel_id": "mychannel"}, 201)),
        })
        ch = client.create_channel("mychannel")
        assert ch.channel_id == "mychannel"

    def test_list_channels(self):
        client = _make_client({
            "/channels": (200, _envelope([{"channel_id": "ch1"}, {"channel_id": "ch2"}])),
        })
        channels = client.list_channels()
        assert len(channels) == 2


class TestPrivateData:
    def test_put_private_data(self):
        client = _make_client({
            "/private-data/secret/key1": (200, _envelope({
                "collection": "secret",
                "key": "key1",
                "hash": "abc123",
            })),
        })
        result = client.put_private_data("secret", "key1", "value1", "org1")
        assert result.hash == "abc123"

    def test_get_private_data(self):
        client = _make_client({
            "/private-data/secret/key1": (200, _envelope({"value": "secret-value"})),
        })
        value = client.get_private_data("secret", "key1", "org1")
        assert value == "secret-value"


class TestErrors:
    def test_not_found_raises(self):
        client = _make_client({})
        with pytest.raises(NotFoundError):
            client.chain_info()

    def test_forbidden_raises(self):
        client = _make_client({
            "/chain/info": (403, {"status_code": 403, "message": "access denied"}),
        })
        with pytest.raises(ForbiddenError):
            client.chain_info()

    def test_validation_error_raises(self):
        client = _make_client({
            "/gateway/submit": (400, {"status_code": 400, "message": "chaincode_id empty"}),
        })
        with pytest.raises(ValidationError):
            tx = TransactionInput(id="x", input_did="a", output_recipient="b", amount=1)
            client.submit_transaction("", "", tx)


class TestLegacyEndpoints:
    def test_create_wallet(self):
        client = _make_client({
            "/wallets/create": (200, {
                "success": True,
                "data": {"address": "0xabc", "balance": 0, "public_key": "pk"},
            }),
        })
        wallet = client.create_wallet()
        assert wallet["address"] == "0xabc"

    def test_mine_block(self):
        client = _make_client({
            "/mine": (200, _envelope({
                "hash": "blockhash",
                "reward": 50,
                "transactions_count": 1,
            })),
        })
        result = client.mine_block("miner1")
        assert result["hash"] == "blockhash"
