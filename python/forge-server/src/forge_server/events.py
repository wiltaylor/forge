"""The event bus fanned out over SSE (``/api/events``) and WS (``/api/ws``).

The bus itself lives in :mod:`forge_server.core.events`; this module streams it
to clients. Who is told about a dropped message is a transport decision: a WS
client gets ``{"type": "lagged"}``, an SSE client just drops.
"""

from __future__ import annotations

import asyncio
import json
from typing import Any, Callable

from fastapi import Depends, FastAPI, HTTPException, Request, WebSocket, WebSocketDisconnect
from sse_starlette.sse import EventSourceResponse

from .config import log
from .core.events import EventBus
from .envelope import fail

SSE_PING_SECS = 15


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
        # Subscribe here, not inside the generator: the response headers reach
        # the client before the generator takes its first step, so a client
        # that publishes as soon as the stream is open would lose the event.
        sub = bus.subscribe()

        async def generator():
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
        except HTTPException as e:
            await _refuse(ws, e)
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


async def _refuse(ws: WebSocket, exc: HTTPException) -> None:
    """Refuse a handshake with the contract's status and envelope.

    Closing before accepting is what every ASGI server offers, but it reaches
    the client as a bare 403 with no body. A server that carries the
    ``websocket.http.response`` extension can answer the upgrade with the real
    response instead, so an unauthorised socket reads as 401 here exactly as
    it does on every other endpoint.
    """
    if "websocket.http.response" not in (ws.scope.get("extensions") or {}):
        # Said out loud rather than passed over: on this host a refused upgrade
        # does not carry the status the contract states, and a silent
        # difference is the thing the contract corpus exists to stop.
        log.warning(
            "this ASGI server has no `websocket.http.response` extension, so a "
            "refused upgrade closes with 1008 instead of the contract's %d",
            exc.status_code,
        )
        await ws.close(code=1008)  # policy violation (bad/missing token)
        return
    response = fail(str(exc.detail), status=exc.status_code)
    # The server frames this refusal itself and adds our headers to its own, so
    # a Content-Length here arrives twice and the client rejects the response
    # as malformed. The server's framing is right; ours is redundant.
    response.raw_headers = [
        (name, value) for name, value in response.raw_headers if name != b"content-length"
    ]
    await ws.send_denial_response(response)


def _parse_topics_list(topics: Any) -> set[str] | None:
    """Empty/omitted topics list = all topics."""
    if not topics or not isinstance(topics, list):
        return None
    return {str(t) for t in topics} or None
