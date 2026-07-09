import asyncio
import json

import pytest
from fastapi.testclient import TestClient

from forge_server import ForgeApp
from forge_server.events import QUEUE_SIZE, EventBus


async def test_bus_publish_reaches_queue():
    bus = EventBus()
    sub = bus.subscribe()
    bus.publish("tick", {"n": 1})
    topic, data = await asyncio.wait_for(sub.queue.get(), timeout=1)
    assert (topic, data) == ("tick", {"n": 1})
    bus.unsubscribe(sub)
    bus.publish("tick", {"n": 2})  # no subscribers — must not raise
    assert sub.queue.empty()


async def test_bus_drops_oldest_when_full():
    bus = EventBus()
    sub = bus.subscribe()
    for i in range(QUEUE_SIZE + 3):
        bus.publish("t", i)
    assert sub.lagged is True
    topic, data = sub.queue.get_nowait()
    assert data == 3  # 0..2 dropped
    assert sub.queue.qsize() == QUEUE_SIZE - 1


def test_ws_ping_subscribe_receive():
    app = ForgeApp("ev").with_events()
    client = TestClient(app.fastapi)
    with client.websocket_connect("/api/ws") as ws:
        # ping/pong round trip also guarantees the subscription is registered
        ws.send_json({"type": "ping"})
        assert ws.receive_json() == {"type": "pong"}

        ws.send_json({"type": "subscribe", "topics": ["metrics"]})
        ws.send_json({"type": "ping"})
        assert ws.receive_json() == {"type": "pong"}  # subscribe processed

        app.events.publish("other", {"skip": True})  # filtered out
        app.events.publish("metrics", {"cpu": 0.5})
        msg = ws.receive_json()
        assert msg == {"type": "event", "topic": "metrics", "data": {"cpu": 0.5}}


def test_ws_default_subscription_is_all_topics():
    app = ForgeApp("ev").with_events()
    client = TestClient(app.fastapi)
    with client.websocket_connect("/api/ws") as ws:
        ws.send_json({"type": "ping"})
        assert ws.receive_json() == {"type": "pong"}
        app.events.publish("anything", [1, 2, 3])
        assert ws.receive_json() == {
            "type": "event",
            "topic": "anything",
            "data": [1, 2, 3],
        }


def test_ws_rejected_without_token_when_auth_enabled():
    from conftest import SECRET

    app = ForgeApp("ev").auth(secret=SECRET, users={"admin": "admin"}).with_events()
    client = TestClient(app.fastapi)
    with pytest.raises(Exception):
        with client.websocket_connect("/api/ws") as ws:
            ws.receive_json()


def test_ws_accepts_query_token_when_auth_enabled():
    from conftest import SECRET, login

    app = ForgeApp("ev").auth(secret=SECRET, users={"admin": "admin"}).with_events()
    client = TestClient(app.fastapi)
    token = login(client)
    with client.websocket_connect(f"/api/ws?token={token}") as ws:
        ws.send_json({"type": "ping"})
        assert ws.receive_json() == {"type": "pong"}


async def test_sse_streams_published_event():
    """Drive the ASGI app directly: TestClient/ASGITransport buffer whole
    bodies, which never completes for an infinite SSE stream."""
    app = ForgeApp("ev").with_events()

    chunks: list[bytes] = []
    status: dict = {}
    got_event = asyncio.Event()
    disconnected = asyncio.Event()

    async def receive():
        await disconnected.wait()
        return {"type": "http.disconnect"}

    async def send(message):
        if message["type"] == "http.response.start":
            status["code"] = message["status"]
            status["headers"] = dict(message["headers"])
        elif message["type"] == "http.response.body":
            chunks.append(message.get("body", b""))
            if b"event: tick" in b"".join(chunks):
                got_event.set()

    scope = {
        "type": "http",
        "asgi": {"version": "3.0", "spec_version": "2.3"},
        "http_version": "1.1",
        "method": "GET",
        "scheme": "http",
        "path": "/api/events",
        "raw_path": b"/api/events",
        "query_string": b"topics=tick,other",
        "root_path": "",
        "headers": [(b"accept", b"text/event-stream")],
        "client": ("127.0.0.1", 12345),
        "server": ("127.0.0.1", 8765),
    }

    task = asyncio.create_task(app.fastapi(scope, receive, send))
    try:
        # Publish until the stream has picked it up (subscription-timing proof).
        async with asyncio.timeout(10):
            app.events.publish("skipped", {"filtered": True})  # not in ?topics=
            while not got_event.is_set():
                app.events.publish("tick", {"n": 7})
                await asyncio.sleep(0.02)
    finally:
        disconnected.set()
        try:
            await asyncio.wait_for(task, timeout=5)
        except (asyncio.TimeoutError, asyncio.CancelledError):
            task.cancel()

    assert status["code"] == 200
    assert status["headers"][b"content-type"].startswith(b"text/event-stream")
    body = b"".join(chunks).decode()
    assert "event: skipped" not in body  # ?topics= filter applied
    frame = next(f for f in body.split("\r\n\r\n") if "event: tick" in f)
    data_line = next(l for l in frame.splitlines() if l.startswith("data:"))
    assert json.loads(data_line.split(":", 1)[1].strip()) == {"n": 7}
