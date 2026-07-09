"""Custom actions: named callables dispatched via ``POST /api/actions/{name}``.

An action is a sync or async callable taking the parsed JSON payload — and
optionally a second ``ctx`` argument (:class:`ActionContext`) — returning any
JSON-able value. Unknown actions 404 and the error lists registered names.
"""

from __future__ import annotations

import inspect
import json
from dataclasses import dataclass
from typing import Any, Callable

from fastapi import Depends, FastAPI, HTTPException, Request
from fastapi.concurrency import run_in_threadpool

from .envelope import ok


@dataclass
class ActionContext:
    """Second (optional) argument passed to actions that want it."""

    claims: dict[str, Any]
    app: Any  # the owning ForgeApp
    events: Any | None  # EventBus when .with_events() was called


class ActionRegistry:
    def __init__(self) -> None:
        self.actions: dict[str, Callable] = {}

    def register(self, name: str, fn: Callable) -> None:
        self.actions[name] = fn

    def names(self) -> list[str]:
        return sorted(self.actions)

    async def dispatch(self, name: str, payload: Any, ctx: ActionContext) -> Any:
        fn = self.actions.get(name)
        if fn is None:
            raise HTTPException(
                404, f"unknown action {name!r} (have: {self.names()})"
            )
        args: tuple = (payload,)
        if _wants_ctx(fn):
            args = (payload, ctx)
        if inspect.iscoroutinefunction(fn):
            return await fn(*args)
        result = await run_in_threadpool(fn, *args)
        if inspect.isawaitable(result):  # sync wrapper returning a coroutine
            result = await result
        return result


def _wants_ctx(fn: Callable) -> bool:
    try:
        sig = inspect.signature(fn)
    except (TypeError, ValueError):
        return False
    positional = [
        p
        for p in sig.parameters.values()
        if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD)
    ]
    if any(p.kind == p.VAR_POSITIONAL for p in sig.parameters.values()):
        return True
    return len(positional) >= 2


def register_routes(
    app: FastAPI,
    registry: ActionRegistry,
    require_claims: Callable,
    make_ctx: Callable[[dict[str, Any]], ActionContext],
) -> None:
    @app.post("/api/actions/{name}")
    async def run_action(
        name: str, request: Request, claims: dict = Depends(require_claims)
    ):
        raw = await request.body()
        try:
            payload = json.loads(raw) if raw else {}
        except json.JSONDecodeError as e:
            raise HTTPException(400, f"body is not valid JSON: {e}") from e
        result = await registry.dispatch(name, payload, make_ctx(claims))
        return ok(result)
