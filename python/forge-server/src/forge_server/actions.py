"""HTTP route for custom actions.

The registry and the dispatch rules live in :mod:`forge_server.core.actions`;
this module mounts them at ``POST /api/actions/{name}``.
"""

from __future__ import annotations

import json
from typing import Any, Callable

from fastapi import Depends, FastAPI, HTTPException, Request

from .core.actions import ActionContext, ActionRegistry
from .envelope import ok


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
