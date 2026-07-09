"""In-process event bus fanned out over SSE (``/api/events``) and WS (``/api/ws``).

Live-telemetry semantics, not a durable queue: each subscriber has a bounded
``asyncio.Queue`` (64); on overflow the oldest message is dropped — WS clients
get ``{"type": "lagged"}``, SSE just drops.
"""

from __future__ import annotations

import asyncio
import json
from typing import Any, Callable

from fastapi import Depends, FastAPI, HTTPException, Request, WebSocket, WebSocketDisconnect
from sse_starlette.sse import EventSourceResponse

QUEUE_SIZE = 64
SSE_PING_SECS = 15


class Subscription:
    __slots__ = ("queue", "lagged")

    def __init__(self) -> None:
        self.queue: asyncio.Queue[tuple[str, Any]] = asyncio.Queue(maxsize=QUEUE_SIZE)
        self.lagged = False


class EventBus:
    def __init__(self) -> None:
        self.subscribers: set[Subscription] = set()
        self._loop: asyncio.AbstractEventLoop | None = None

    def subscribe(self) -> Subscription:
        self._loop = asyncio.get_running_loop()
        sub = Subscription()
        self.subscribers.add(sub)
        return sub

    def unsubscribe(self, sub: Subscription) -> None:
        self.subscribers.discard(sub)

    def publish(self, topic: str, data: Any = None) -> None:
        """Fan ``(topic, data)`` out to every subscriber. Thread-safe: when
        called off the loop that owns the subscribers, delivery is scheduled
        with ``call_soon_threadsafe``."""
        try:
            running = asyncio.get_running_loop()
        except RuntimeError:
            running = None
        if self._loop is not None and running is not self._loop:
            self._loop.call_soon_threadsafe(self._deliver, topic, data)
        else:
            self._deliver(topic, data)

    def _deliver(self, topic: str, data: Any) -> None:
        for sub in list(self.subscribers):
            try:
                sub.queue.put_nowait((topic, data))
            except asyncio.QueueFull:
                try:
                    sub.queue.get_nowait()  # drop oldest
                except asyncio.QueueEmpty:
                    pass
                sub.lagged = True
                try:
                    sub.queue.put_nowait((topic, data))
                except asyncio.QueueFull:
                    pass


def _parse_topics(raw: str | None) -> set[str] | None:
    """``None``/empty = all topics."""
    if not raw:
        return None
    topics = {t.strip() for t in raw.split(",") if t.strip()}
    return topics or None


def register_routes(app: FastAPI, bus: EventBus, require_claims: Callable) -> None:
    from . import auth as _auth

    @app.get("/api/events")
    async def sse_events(
        request: Request,
        topics: str | None = None,
        claims: dict = Depends(require_claims),
    ):
        wanted = _parse_topics(topics)

        async def generator():
            sub = bus.subscribe()
            try:
                while True:
                    topic, data = await sub.queue.get()
                    if wanted is not None and topic not in wanted:
                        continue
                    yield {"event": topic, "data": json.dumps(data)}
            finally:
                bus.unsubscribe(sub)

        # sse-starlette sends a `: ping` comment heartbeat every `ping` seconds.
        return EventSourceResponse(generator(), ping=SSE_PING_SECS)

    @app.websocket("/api/ws")
    async def ws_events(ws: WebSocket):
        try:
            _auth.websocket_claims(ws)
        except HTTPException:
            await ws.close(code=1008)  # policy violation (bad/missing token)
            return

        await ws.accept()
        sub = bus.subscribe()
        wanted: set[str] | None = None  # None = all topics

        async def pump() -> None:
            while True:
                if sub.lagged:
                    sub.lagged = False
                    await ws.send_json({"type": "lagged"})
                topic, data = await sub.queue.get()
                if wanted is not None and topic not in wanted:
                    continue
                await ws.send_json({"type": "event", "topic": topic, "data": data})

        sender = asyncio.create_task(pump())
        try:
            while True:
                try:
                    msg = await ws.receive_json()
                except (json.JSONDecodeError, ValueError):
                    continue  # ignore non-JSON frames
                if not isinstance(msg, dict):
                    continue
                kind = msg.get("type")
                if kind == "subscribe":
                    wanted = _parse_topics_list(msg.get("topics"))
                elif kind == "ping":
                    await ws.send_json({"type": "pong"})
        except WebSocketDisconnect:
            pass
        finally:
            sender.cancel()
            bus.unsubscribe(sub)


def _parse_topics_list(topics: Any) -> set[str] | None:
    """Empty/omitted topics list = all topics."""
    if not topics or not isinstance(topics, list):
        return None
    return {str(t) for t in topics} or None
