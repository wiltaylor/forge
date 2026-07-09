from fastapi.testclient import TestClient

from forge_server import ForgeApp


def test_echo_roundtrip():
    app = ForgeApp("act")

    @app.action("echo")
    def echo(payload):
        return payload

    client = TestClient(app.fastapi)
    r = client.post("/api/actions/echo", json={"hello": "world", "n": [1, 2]})
    assert r.status_code == 200
    assert r.json() == {"ok": True, "data": {"hello": "world", "n": [1, 2]}}
    # empty body → empty payload
    assert client.post("/api/actions/echo").json() == {"ok": True, "data": {}}


def test_unknown_action_404_lists_names():
    app = ForgeApp("act")

    @app.action("echo")
    def echo(payload):
        return payload

    @app.action("boom")
    def boom(payload):
        return None

    client = TestClient(app.fastapi)
    r = client.post("/api/actions/nope", json={})
    assert r.status_code == 404
    err = r.json()["error"]
    assert "nope" in err and "boom" in err and "echo" in err


def test_async_action():
    app = ForgeApp("act")

    @app.action("double")
    async def double(payload):
        return {"n": payload["n"] * 2}

    client = TestClient(app.fastapi)
    assert client.post("/api/actions/double", json={"n": 21}).json()["data"] == {"n": 42}


def test_action_with_ctx():
    app = ForgeApp("act").with_events()

    @app.action("whoami")
    def whoami(payload, ctx):
        assert ctx.events is app.events
        assert ctx.app is app
        return {"sub": ctx.claims["sub"]}

    client = TestClient(app.fastapi)
    r = client.post("/api/actions/whoami", json={})
    assert r.json()["data"] == {"sub": "anonymous"}  # auth disabled → anonymous


def test_invalid_json_body_400():
    app = ForgeApp("act")

    @app.action("echo")
    def echo(payload):
        return payload

    client = TestClient(app.fastapi)
    r = client.post(
        "/api/actions/echo", content=b"{bad", headers={"content-type": "application/json"}
    )
    assert r.status_code == 400


def test_health_lists_actions():
    app = ForgeApp("act")

    @app.action("b")
    def b(payload):
        return None

    @app.action("a")
    def a(payload):
        return None

    client = TestClient(app.fastapi)
    assert client.get("/api/health").json()["data"]["actions"] == ["a", "b"]
