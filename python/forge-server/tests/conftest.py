import pytest

from forge_server import ForgeApp

SECRET = "unit-test-secret-0123456789abcdef!"  # >= 32 chars
USERS = {"admin": "admin", "ops": "hunter2"}


@pytest.fixture
def open_app(tmp_path):
    """Auth-disabled app with a docstore — must work with zero env config."""
    app = ForgeApp("test-open")
    app.with_docstore(tmp_path / "data")
    return app


@pytest.fixture
def auth_app(tmp_path):
    """Auth-enabled app configured explicitly (not via process env)."""
    app = ForgeApp("test-auth")
    app.auth(secret=SECRET, users=dict(USERS))
    app.with_docstore(tmp_path / "data")
    return app


def login(client, username="admin", password="admin"):
    r = client.post(
        "/api/auth/login", json={"username": username, "password": password}
    )
    assert r.status_code == 200, r.text
    return r.json()["data"]["token"]
