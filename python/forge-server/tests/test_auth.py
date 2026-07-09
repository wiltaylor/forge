import time

import jwt
import pytest
from fastapi.testclient import TestClient

from forge_server import ForgeApp
from conftest import SECRET, login


def test_login_ok(auth_app):
    client = TestClient(auth_app.fastapi)
    r = client.post("/api/auth/login", json={"username": "admin", "password": "admin"})
    assert r.status_code == 200
    body = r.json()
    assert body["ok"] is True
    data = body["data"]
    assert data["user"] == {"name": "admin", "roles": []}
    assert isinstance(data["token"], str) and data["token"]
    assert data["expires_at"] > time.time()
    claims = jwt.decode(data["token"], SECRET, algorithms=["HS256"])
    assert claims["sub"] == "admin"
    assert claims["roles"] == []
    assert claims["iss"] == "forge"
    assert claims["exp"] == data["expires_at"]


@pytest.mark.parametrize(
    "creds",
    [
        {"username": "admin", "password": "wrong"},
        {"username": "nobody", "password": "admin"},
    ],
)
def test_login_bad_credentials(auth_app, creds):
    client = TestClient(auth_app.fastapi)
    r = client.post("/api/auth/login", json=creds)
    assert r.status_code == 401
    assert r.json()["ok"] is False


def test_login_404_when_auth_disabled(open_app):
    client = TestClient(open_app.fastapi)
    r = client.post("/api/auth/login", json={"username": "a", "password": "b"})
    assert r.status_code == 404
    assert r.json()["ok"] is False


def test_me_with_bearer_token(auth_app):
    client = TestClient(auth_app.fastapi)
    token = login(client)
    r = client.get("/api/auth/me", headers={"Authorization": f"Bearer {token}"})
    assert r.status_code == 200
    data = r.json()["data"]
    assert data["sub"] == "admin"
    assert data["roles"] == []
    assert data["iss"] == "forge"
    assert isinstance(data["exp"], int)


def test_me_with_query_param_token(auth_app):
    client = TestClient(auth_app.fastapi)
    token = login(client)
    r = client.get(f"/api/auth/me?token={token}")
    assert r.status_code == 200
    assert r.json()["data"]["sub"] == "admin"


def test_me_without_token_401(auth_app):
    client = TestClient(auth_app.fastapi)
    r = client.get("/api/auth/me")
    assert r.status_code == 401
    assert r.json()["ok"] is False


def test_expired_token_401(auth_app):
    now = int(time.time())
    expired = jwt.encode(
        {"sub": "admin", "roles": [], "iat": now - 7200, "exp": now - 3600, "iss": "forge"},
        SECRET,
        algorithm="HS256",
    )
    client = TestClient(auth_app.fastapi)
    r = client.get("/api/auth/me", headers={"Authorization": f"Bearer {expired}"})
    assert r.status_code == 401
    assert "token" in r.json()["error"]


def test_anonymous_claims_when_disabled(open_app):
    client = TestClient(open_app.fastapi)
    r = client.get("/api/auth/me")
    assert r.status_code == 200
    data = r.json()["data"]
    assert data["sub"] == "anonymous"
    assert data["roles"] == []
    # protected-when-enabled endpoints are open too
    assert client.get("/api/data").status_code == 200


def test_docstore_requires_token_when_auth_enabled(auth_app):
    client = TestClient(auth_app.fastapi)
    assert client.get("/api/data").status_code == 401
    token = login(client)
    assert (
        client.get("/api/data", headers={"Authorization": f"Bearer {token}"}).status_code
        == 200
    )


def test_short_secret_raises():
    with pytest.raises(ValueError):
        ForgeApp("bad").auth(secret="too-short")


def test_auth_from_env_requires_secret(monkeypatch):
    monkeypatch.delenv("FORGE_JWT_SECRET", raising=False)
    with pytest.raises(RuntimeError):
        ForgeApp("no-secret").auth_from_env()


def test_login_with_argon2_hash(tmp_path):
    from forge_server.hash import hash_password

    app = ForgeApp("argon").auth(
        secret=SECRET, users={"ops": hash_password("hunter2")}
    )
    client = TestClient(app.fastapi)
    assert (
        client.post(
            "/api/auth/login", json={"username": "ops", "password": "hunter2"}
        ).status_code
        == 200
    )
    assert (
        client.post(
            "/api/auth/login", json={"username": "ops", "password": "wrong"}
        ).status_code
        == 401
    )


def test_users_string_parsing_first_colon_wins():
    app = ForgeApp("parse").auth(secret=SECRET, users="admin:pa:ss,ops:hunter2")
    auth = app.fastapi.state.forge_auth
    assert auth.users == {"admin": "pa:ss", "ops": "hunter2"}


def test_health_reports_auth_state(auth_app, open_app):
    assert TestClient(auth_app.fastapi).get("/api/health").json()["data"]["auth_enabled"]
    body = TestClient(open_app.fastapi).get("/api/health").json()
    assert body["ok"] is True
    data = body["data"]
    assert data["auth_enabled"] is False
    assert data["app"] == "test-open"
    assert "uptime_s" in data and "version" in data and data["actions"] == []
