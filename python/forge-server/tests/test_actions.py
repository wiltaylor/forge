"""The action registry called directly — no server.

The dispatch rules live in :mod:`forge_server.core.actions`. The client tests
at the bottom cover only what the route adds: body parsing, the claims-to-
context wiring, and the health listing.
"""

import pytest

from forge_server.core.actions import ActionContext, ActionRegistry
from forge_server.core.error import NotFound


def _ctx() -> ActionContext:
    return ActionContext(claims={"sub": "tester"}, app=None, events=None)


async def test_echo_roundtrip():
    """The core runs a sync action on the threadpool it no longer reaches
    through the framework, so this is the path the unweld changed."""
    registry = ActionRegistry()
    registry.register("echo", lambda payload: payload)
    payload = {"hello": "world", "n": [1, 2]}
    assert await registry.dispatch("echo", payload, _ctx()) == payload


async def test_unknown_action_is_a_404_listing_the_names():
    registry = ActionRegistry()
    registry.register("echo", lambda payload: payload)
    registry.register("boom", lambda payload: None)
    with pytest.raises(NotFound) as e:
        await registry.dispatch("nope", {}, _ctx())
    assert e.value.status == 404
    assert "nope" in e.value.message
    assert "['boom', 'echo']" in e.value.message  # the names, sorted


async def test_async_action():
    registry = ActionRegistry()

    async def double(payload):
        return {"n": payload["n"] * 2}

    registry.register("double", double)
    assert await registry.dispatch("double", {"n": 21}, _ctx()) == {"n": 42}


async def test_sync_wrapper_returning_a_coroutine_is_awaited():
    registry = ActionRegistry()

    async def inner(payload):
        return {"n": payload["n"]}

    registry.register("wrapped", lambda payload: inner(payload))
    assert await registry.dispatch("wrapped", {"n": 5}, _ctx()) == {"n": 5}


async def test_two_argument_action_receives_the_ctx():
    registry = ActionRegistry()
    seen = {}

    def whoami(payload, ctx):
        seen["ctx"] = ctx
        return {"sub": ctx.claims["sub"]}

    registry.register("whoami", whoami)
    ctx = _ctx()
    assert await registry.dispatch("whoami", {}, ctx) == {"sub": "tester"}
    assert seen["ctx"] is ctx


async def test_var_positional_action_receives_the_ctx():
    registry = ActionRegistry()

    def catchall(*args):
        return {"argc": len(args)}

    registry.register("catchall", catchall)
    assert await registry.dispatch("catchall", {}, _ctx()) == {"argc": 2}


# -- the route: what HTTP adds ---------------------------------------------


def forge_app():
    from forge_server import ForgeApp

    return ForgeApp("act")


def client_for(app):
    from fastapi.testclient import TestClient

    return TestClient(app.fastapi)


def echo_app():
    app = forge_app()

    @app.action("echo")
    def echo(payload):
        return payload

    return app


def test_route_parses_the_body_and_wraps_the_result():
    client = client_for(echo_app())
    r = client.post("/api/actions/echo", json={"hello": "world"})
    assert r.status_code == 200
    assert r.json() == {"ok": True, "data": {"hello": "world"}}
    # empty body → empty payload
    assert client.post("/api/actions/echo").json() == {"ok": True, "data": {}}


def test_invalid_json_body_400():
    r = client_for(echo_app()).post(
        "/api/actions/echo", content=b"{bad", headers={"content-type": "application/json"}
    )
    assert r.status_code == 400


def test_route_builds_the_ctx_from_the_app_and_the_claims():
    app = forge_app().with_events()

    @app.action("whoami")
    def whoami(payload, ctx):
        assert ctx.events is app.events
        assert ctx.app is app
        return {"sub": ctx.claims["sub"]}

    r = client_for(app).post("/api/actions/whoami", json={})
    assert r.json()["data"] == {"sub": "anonymous"}  # auth disabled → anonymous


def test_health_lists_actions():
    app = forge_app()

    @app.action("b")
    def b(payload):
        return None

    @app.action("a")
    def a(payload):
        return None

    assert client_for(app).get("/api/health").json()["data"]["actions"] == ["a", "b"]
