"""HTTP routes for the JSON document store.

The doc-name rule and the file handling live in :mod:`forge_server.core.docstore`;
this module mounts them at ``/api/data`` and ``/api/data/{name}``.
"""

from __future__ import annotations

import json
from typing import Any, Callable

from fastapi import Depends, FastAPI, HTTPException, Request

from .core.docstore import DocStore
from .envelope import ok


async def _json_body(request: Request, default: Any) -> Any:
    raw = await request.body()
    if not raw:
        return default
    try:
        return json.loads(raw)
    except json.JSONDecodeError as e:
        raise HTTPException(400, f"body is not valid JSON: {e}") from e


def register_routes(app: FastAPI, store: DocStore, require_claims: Callable) -> None:
    @app.get("/api/data")
    async def list_docs(claims: dict = Depends(require_claims)):
        return ok(store.list())

    @app.get("/api/data/{name}")
    async def get_doc(name: str, claims: dict = Depends(require_claims)):
        return ok(store.read(name))

    @app.put("/api/data/{name}")
    async def put_doc(name: str, request: Request, claims: dict = Depends(require_claims)):
        body = await _json_body(request, default=None)
        store.write(name, body)
        return ok()

    @app.delete("/api/data/{name}")
    async def delete_doc(name: str, claims: dict = Depends(require_claims)):
        store.delete(name)
        return ok()
