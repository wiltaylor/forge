"""WebSocket contract tests — see test_contract.py for how to run."""
import json
import os

import httpx
import pytest
from websockets.sync.client import connect

BASE = os.environ.get("FORGE_TEST_BASE_URL", "http://127.0.0.1:8765")
USER = os.environ.get("FORGE_TEST_USER", "admin")
PASSWORD = os.environ.get("FORGE_TEST_PASSWORD", "admin")


@pytest.fixture(scope="module")
def token():
    r = httpx.post(f"{BASE}/api/auth/login", json={"username": USER, "password": PASSWORD})
    assert r.status_code == 200, r.text
    return r.json()["data"]["token"]


def ws_url(token: str) -> str:
    return BASE.replace("http", "ws", 1) + f"/api/ws?token={token}"


def test_ws_requires_token():
    with pytest.raises(Exception):
        with connect(BASE.replace("http", "ws", 1) + "/api/ws", open_timeout=5) as ws:
            ws.recv(timeout=2)


def test_ws_ping_pong(token):
    with connect(ws_url(token), open_timeout=5) as ws:
        ws.send(json.dumps({"type": "ping"}))
        frame = json.loads(ws.recv(timeout=5))
        assert frame["type"] == "pong"


def test_ws_subscribe_and_receive(token):
    with connect(ws_url(token), open_timeout=5) as ws:
        ws.send(json.dumps({"type": "subscribe", "topics": ["paritytest-ws"]}))
        # publish through the HTTP action while the socket is open
        r = httpx.post(
            f"{BASE}/api/actions/publish",
            json={"topic": "paritytest-ws", "data": {"hello": "ws"}},
            headers={"Authorization": f"Bearer {token}"},
        )
        assert r.status_code == 200
        # skip any non-event frames (e.g. pongs)
        for _ in range(5):
            frame = json.loads(ws.recv(timeout=8))
            if frame.get("type") == "event":
                assert frame["topic"] == "paritytest-ws"
                assert frame["data"] == {"hello": "ws"}
                return
        pytest.fail("no event frame received")
