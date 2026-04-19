"""Tesseract HTTP client — async, zero dependencies."""
import asyncio
import json
import urllib.request
import urllib.error
from dataclasses import dataclass
from typing import Optional
from concurrent.futures import ThreadPoolExecutor

_pool = ThreadPoolExecutor(max_workers=32)


@dataclass
class Cell:
    probability: float
    crystallized: bool
    record: str
    support: int = 0


def _http_get(url: str) -> dict:
    try:
        req = urllib.request.Request(url)
        with urllib.request.urlopen(req, timeout=5) as r:
            return json.loads(r.read())
    except Exception:
        return {}


def _http_post(url: str, data: dict) -> dict:
    try:
        body = json.dumps(data).encode()
        req = urllib.request.Request(url, data=body, method="POST")
        req.add_header("Content-Type", "application/json")
        with urllib.request.urlopen(req, timeout=5) as r:
            return json.loads(r.read())
    except Exception:
        return {}


class TesseractClient:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip("/")

    async def seed(self, t: int, c: int, o: int, v: int, event_id: str) -> dict:
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            _pool,
            _http_post,
            f"{self.base_url}/seed",
            {"t": t, "c": c, "o": o, "v": v, "event_id": event_id},
        )

    async def get_cell(self, t: int, c: int, o: int, v: int) -> Cell:
        loop = asyncio.get_event_loop()
        d = await loop.run_in_executor(
            _pool, _http_get, f"{self.base_url}/cell/{t}/{c}/{o}/{v}"
        )
        return Cell(
            probability=d.get("probability", 0),
            crystallized=d.get("crystallized", False),
            record=d.get("record", ""),
            support=d.get("support", 0),
        )

    async def destroy(self, t: int, c: int, o: int, v: int) -> dict:
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            _pool,
            _http_post,
            f"{self.base_url}/destroy",
            {"t": t, "c": c, "o": o, "v": v},
        )

    async def status(self) -> dict:
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            _pool, _http_get, f"{self.base_url}/status"
        )

    async def close(self):
        pass
