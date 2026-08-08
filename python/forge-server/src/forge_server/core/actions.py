"""Custom actions: named callables dispatched by name.

An action is a sync or async callable taking the parsed JSON payload — and
optionally a second ``ctx`` argument (:class:`ActionContext`) — returning any
JSON-able value. An unknown action raises :class:`NotFound` (404) and the
error lists the registered names.
"""

from __future__ import annotations

import functools
import inspect
from dataclasses import dataclass
from typing import Any, Callable

import anyio.to_thread

from .error import NotFound


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
            raise NotFound(f"unknown action {name!r} (have: {self.names()})")
        args: tuple = (payload,)
        if _wants_ctx(fn):
            args = (payload, ctx)
        if inspect.iscoroutinefunction(fn):
            return await fn(*args)
        # A sync action must not block the loop. This is what the framework's
        # `run_in_threadpool` does, called directly: same worker pool, same
        # limiter, so a sync action queues exactly as it did before — but anyio
        # is a concurrency library, not a web framework, so the core stays
        # callable without one. `asyncio.to_thread` would be a second, smaller
        # pool with its own backpressure, and would tie the core to asyncio.
        result = await anyio.to_thread.run_sync(functools.partial(fn, *args))
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
