"""Black-box Forge API contract tests (docs/api-contract.md).

Run against a LIVE server (either backend — that's the point):

    FORGE_TEST_BASE_URL=http://127.0.0.1:8765 pytest examples/parity -q

Assumes the demo configuration: auth enabled, FORGE_AUTH_USERS contains
admin:admin, doc store + events + a `publish` and `echo` action registered
(both example apps do this). Tests are additive-only: they create and delete
docs under the `paritytest-` prefix.
"""
import json
import os
import time

import httpx
import pytest

BASE = os.environ.get("FORGE_TEST_BASE_URL", "http://127.0.0.1:8765")
USER = os.environ.get("FORGE_TEST_USER", "admin")
PASSWORD = os.environ.get("FORGE_TEST_PASSWORD", "admin")
DOC = "paritytest-doc"


@pytest.fixture(scope="session")
def client():
    with httpx.Client(base_url=BASE, timeout=10.0) as c:
        yield c


@pytest.fixture(scope="session")
def token(client):
    r = client.post("/api/auth/login", json={"username": USER, "password": PASSWORD})
    assert r.status_code == 200, r.text
    body = r.json()
    assert body["ok"] is True
    data = body["data"]
    assert isinstance(data["token"], str) and data["token"]
    assert isinstance(data["expires_at"], (int, float))
    assert data["user"]["name"] == USER
    return data["token"]


@pytest.fixture(scope="session")
def auth(token):
    return {"Authorization": f"Bearer {token}"}


# ---------------- health ----------------

def test_health_open(client):
    r = client.get("/api/health")
    assert r.status_code == 200
    body = r.json()
    assert body["ok"] is True
    data = body["data"]
    assert data["auth_enabled"] is True
    assert isinstance(data["uptime_s"], (int, float))
    assert isinstance(data["actions"], list)
    assert "echo" in data["actions"]


# ---------------- auth ----------------

def test_login_bad_credentials(client):
    r = client.post("/api/auth/login", json={"username": USER, "password": "definitely-wrong"})
    assert r.status_code == 401
    body = r.json()
    assert body["ok"] is False
    assert isinstance(body["error"], str)


def test_me_requires_token(client):
    r = client.get("/api/auth/me")
    assert r.status_code == 401
    assert r.json()["ok"] is False


def test_me_with_bearer(client, auth):
    r = client.get("/api/auth/me", headers=auth)
    assert r.status_code == 200
    data = r.json()["data"]
    assert data["sub"] == USER
    assert isinstance(data["roles"], list)
    assert isinstance(data["exp"], int)


def test_query_param_token_accepted(client, token):
    r = client.get("/api/auth/me", params={"token": token})
    assert r.status_code == 200
    assert r.json()["data"]["sub"] == USER


def test_garbage_token_rejected(client):
    r = client.get("/api/auth/me", headers={"Authorization": "Bearer not.a.jwt"})
    assert r.status_code == 401


# ---------------- doc store ----------------

def test_data_requires_auth(client):
    assert client.get("/api/data").status_code == 401


def test_doc_roundtrip(client, auth):
    doc = {"n": 1, "nested": {"ok": True}, "s": "x"}
    r = client.put(f"/api/data/{DOC}", json=doc, headers=auth)
    assert r.status_code == 200
    assert r.json()["ok"] is True

    r = client.get(f"/api/data/{DOC}", headers=auth)
    assert r.status_code == 200
    assert r.json()["data"] == doc

    r = client.get("/api/data", headers=auth)
    names = [d["name"] for d in r.json()["data"]]
    assert DOC in names
    meta = next(d for d in r.json()["data"] if d["name"] == DOC)
    assert meta["bytes"] > 0
    assert isinstance(meta["modified"], (int, float))

    r = client.delete(f"/api/data/{DOC}", headers=auth)
    assert r.status_code == 200
    # idempotent
    r = client.delete(f"/api/data/{DOC}", headers=auth)
    assert r.status_code == 200
    r = client.get(f"/api/data/{DOC}", headers=auth)
    assert r.status_code == 404
    assert r.json()["ok"] is False


@pytest.mark.parametrize("bad", ["UPPER", "-lead", ".dot", "a" * 70, "sl/ash", "..", "a b"])
def test_doc_bad_names(client, auth, bad):
    r = client.put(f"/api/data/{bad}", json={}, headers=auth)
    # 400 = validation reject; 404/405 = names with path separators never
    # match the route at the router layer (equally safe).
    assert r.status_code in (400, 404, 405), f"{bad!r} -> {r.status_code}"
    if r.headers.get("content-type", "").startswith("application/json"):
        body = r.json()
        if "ok" in body:
            assert body["ok"] is False


# ---------------- actions ----------------

def test_action_echo(client, auth):
    payload = {"ping": "pong", "n": 3}
    r = client.post("/api/actions/echo", json=payload, headers=auth)
    assert r.status_code == 200
    assert r.json()["data"] == payload


def test_action_unknown_404(client, auth):
    r = client.post("/api/actions/definitely-not-registered", json={}, headers=auth)
    assert r.status_code == 404
    body = r.json()
    assert body["ok"] is False
    assert "echo" in body["error"]


# ---------------- events (SSE) ----------------

def test_sse_receives_published_event(client, token, auth):
    with client.stream(
        "GET", "/api/events", params={"token": token, "topics": "paritytest"}, timeout=10.0,
    ) as stream:
        assert stream.status_code == 200
        assert stream.headers["content-type"].startswith("text/event-stream")
        client.post(
            "/api/actions/publish",
            json={"topic": "paritytest", "data": {"hello": "sse"}},
            headers=auth,
        )
        deadline = time.time() + 8
        event_name = None
        payload = None
        for line in stream.iter_lines():
            if time.time() > deadline:
                break
            if line.startswith("event:"):
                event_name = line.split(":", 1)[1].strip()
            elif line.startswith("data:") and event_name == "paritytest":
                payload = json.loads(line.split(":", 1)[1].strip())
                break
        assert payload == {"hello": "sse"}, f"no event received (last name={event_name})"


# ---------------- frontend / fallback ----------------

def test_api_miss_is_json_404(client):
    r = client.get("/api/definitely-not-a-route")
    assert r.status_code == 404
    assert r.headers["content-type"].startswith("application/json")
    assert r.json()["ok"] is False


def test_spa_fallback_serves_html(client):
    r = client.get("/some/client/route")
    assert r.status_code == 200
    assert "text/html" in r.headers["content-type"]


# ---------------- components (federation) ----------------

def test_components_manifest(client, auth):
    r = client.get("/api/components", headers=auth)
    if r.status_code == 404:
        pytest.skip("components dir not configured on this server")
    assert r.status_code == 200
    data = r.json()["data"]
    assert "components" in data and isinstance(data["components"], list)
    if data["components"]:
        comp = data["components"][0]
        file = comp["file"]
        rf = client.get(f"/api/components/{file}", headers=auth)
        assert rf.status_code == 200
        # traversal guard
        assert client.get("/api/components/../secret.js", headers=auth).status_code in (400, 404)


def test_components_require_auth(client):
    r = client.get("/api/components")
    assert r.status_code in (401, 404)
