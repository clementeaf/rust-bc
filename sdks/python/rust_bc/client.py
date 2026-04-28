"""Core HTTP client for the rust-bc blockchain network."""

from __future__ import annotations

from typing import Any

import httpx

from rust_bc.exceptions import (
    BlockchainError,
    ConnectionError,
    ForbiddenError,
    NotFoundError,
    ValidationError,
)
from rust_bc.types import (
    ApiResponse,
    ChannelInfo,
    GatewaySubmitResponse,
    HealthCheck,
    Organization,
    PrivateDataWriteResponse,
    SimulateResponse,
    TransactionInput,
)


class BlockchainClient:
    """Synchronous client for the rust-bc blockchain REST API.

    Usage::

        client = BlockchainClient("https://localhost:8080/api/v1")
        health = client.health()
        print(health.status)  # "healthy"
    """

    def __init__(
        self,
        base_url: str = "http://127.0.0.1:8080/api/v1",
        *,
        timeout: float = 30.0,
        verify_ssl: bool = True,
    ) -> None:
        self._client = httpx.Client(
            base_url=base_url,
            timeout=timeout,
            verify=verify_ssl,
            headers={"Content-Type": "application/json"},
        )

    def close(self) -> None:
        """Close the underlying HTTP connection pool."""
        self._client.close()

    def __enter__(self) -> "BlockchainClient":
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    # ── Helpers ───────────────────────────────────────────────────────────────

    def _unwrap(self, response: httpx.Response) -> Any:
        """Parse the API envelope and return the `data` field, or raise."""
        self._check_status(response)
        body = response.json()

        # Gateway envelope
        if "status_code" in body:
            envelope = ApiResponse.model_validate(body)
            if envelope.status_code >= 400:
                self._raise_for_code(envelope.status_code, envelope.message)
            return envelope.data

        # Legacy envelope
        if "success" in body:
            if not body.get("success"):
                msg = body.get("message", "request failed")
                raise BlockchainError(str(msg))
            return body.get("data")

        return body

    def _check_status(self, response: httpx.Response) -> None:
        if response.status_code == 404:
            raise NotFoundError("resource not found", 404)
        if response.status_code == 403:
            raise ForbiddenError("access denied", 403)
        if response.status_code == 400:
            msg = ""
            try:
                msg = response.json().get("message", "")
            except Exception:
                pass
            raise ValidationError(str(msg) or "bad request", 400)
        if response.status_code >= 500:
            raise BlockchainError(f"server error: {response.status_code}", response.status_code)

    @staticmethod
    def _raise_for_code(code: int, message: str) -> None:
        if code == 404:
            raise NotFoundError(message, 404)
        if code == 403:
            raise ForbiddenError(message, 403)
        if code == 400:
            raise ValidationError(message, 400)
        if code >= 500:
            raise BlockchainError(message, code)

    # ── Health ────────────────────────────────────────────────────────────────

    def health(self) -> HealthCheck:
        """GET /health — node health with dependency checks."""
        resp = self._client.get("/health")
        data = self._unwrap(resp)
        return HealthCheck.model_validate(data)

    # ── Gateway ───────────────────────────────────────────────────────────────

    def submit_transaction(
        self,
        chaincode_id: str,
        channel_id: str,
        tx: TransactionInput,
    ) -> GatewaySubmitResponse:
        """POST /gateway/submit — full endorse → order → commit pipeline."""
        body = {
            "chaincode_id": chaincode_id,
            "channel_id": channel_id,
            "transaction": tx.model_dump(),
        }
        resp = self._client.post("/gateway/submit", json=body)
        data = self._unwrap(resp)
        return GatewaySubmitResponse.model_validate(data)

    # ── Chaincode ─────────────────────────────────────────────────────────────

    def evaluate(
        self,
        chaincode_id: str,
        function: str,
        version: str | None = None,
    ) -> SimulateResponse:
        """POST /chaincode/{id}/simulate — read-only chaincode query."""
        url = f"/chaincode/{chaincode_id}/simulate"
        if version:
            url += f"?version={version}"
        body = {"function": function}
        resp = self._client.post(url, json=body)
        data = self._unwrap(resp)
        return SimulateResponse.model_validate(data)

    # ── Organizations ─────────────────────────────────────────────────────────

    def register_org(self, org: Organization) -> Organization:
        """POST /store/organizations — register an organization."""
        resp = self._client.post("/store/organizations", json=org.model_dump())
        data = self._unwrap(resp)
        return Organization.model_validate(data)

    def list_orgs(self) -> list[Organization]:
        """GET /store/organizations — list all organizations."""
        resp = self._client.get("/store/organizations")
        data = self._unwrap(resp)
        if isinstance(data, list):
            return [Organization.model_validate(o) for o in data]
        return []

    # ── Policies ──────────────────────────────────────────────────────────────

    def set_policy(self, resource_id: str, policy: dict[str, Any]) -> None:
        """POST /store/policies — set endorsement policy for a resource."""
        body = {"resource_id": resource_id, "policy": policy}
        resp = self._client.post("/store/policies", json=body)
        self._unwrap(resp)

    # ── Channels ──────────────────────────────────────────────────────────────

    def create_channel(self, channel_id: str) -> ChannelInfo:
        """POST /channels — create a new channel."""
        resp = self._client.post("/channels", json={"channel_id": channel_id})
        data = self._unwrap(resp)
        return ChannelInfo.model_validate(data)

    def list_channels(self) -> list[ChannelInfo]:
        """GET /channels — list all channels."""
        resp = self._client.get("/channels")
        data = self._unwrap(resp)
        if isinstance(data, list):
            return [ChannelInfo.model_validate(c) for c in data]
        return []

    # ── Private data ──────────────────────────────────────────────────────────

    def put_private_data(
        self,
        collection: str,
        key: str,
        value: str,
        org_id: str,
    ) -> PrivateDataWriteResponse:
        """PUT /private-data/{collection}/{key} — write private data."""
        resp = self._client.put(
            f"/private-data/{collection}/{key}",
            json={"value": value},
            headers={"X-Org-Id": org_id},
        )
        data = self._unwrap(resp)
        return PrivateDataWriteResponse.model_validate(data)

    def get_private_data(self, collection: str, key: str, org_id: str) -> str:
        """GET /private-data/{collection}/{key} — read private data."""
        resp = self._client.get(
            f"/private-data/{collection}/{key}",
            headers={"X-Org-Id": org_id},
        )
        data = self._unwrap(resp)
        if isinstance(data, dict):
            return str(data.get("value", ""))
        return str(data)

    # ── Blocks ────────────────────────────────────────────────────────────────

    def get_blocks(self) -> list[dict[str, Any]]:
        """GET /blocks — full blockchain."""
        resp = self._client.get("/blocks")
        data = self._unwrap(resp)
        return data if isinstance(data, list) else []

    def get_block_by_index(self, index: int) -> dict[str, Any]:
        """GET /blocks/index/{index}."""
        resp = self._client.get(f"/blocks/index/{index}")
        return self._unwrap(resp)

    # ── Wallets ───────────────────────────────────────────────────────────────

    def create_wallet(self) -> dict[str, Any]:
        """POST /wallets/create — create a new wallet."""
        resp = self._client.post("/wallets/create")
        return self._unwrap(resp)

    def get_wallet(self, address: str) -> dict[str, Any]:
        """GET /wallets/{address} — get wallet balance."""
        resp = self._client.get(f"/wallets/{address}")
        return self._unwrap(resp)

    # ── Mining ────────────────────────────────────────────────────────────────

    def mine_block(self, miner_address: str) -> dict[str, Any]:
        """POST /mine — mine a block."""
        resp = self._client.post("/mine", json={"miner_address": miner_address})
        return self._unwrap(resp)

    # ── Chain info ────────────────────────────────────────────────────────────

    def chain_info(self) -> dict[str, Any]:
        """GET /chain/info — blockchain metadata."""
        resp = self._client.get("/chain/info")
        return self._unwrap(resp)

    def verify_chain(self) -> dict[str, Any]:
        """GET /chain/verify — integrity verification."""
        resp = self._client.get("/chain/verify")
        return self._unwrap(resp)
