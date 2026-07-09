"""JSON document store (playpen lineage): one file per doc, atomic writes.

- Doc name regex ``^[a-z0-9][a-z0-9_-]{0,63}$`` doubles as the
  path-traversal guard (violations → 400).
- One file per doc: ``<data-dir>/<name>.json``.
- Writes are atomic: write ``<name>.json.tmp`` then rename over the target.
- DELETE of a missing doc succeeds (idempotent).
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Callable

from fastapi import Depends, FastAPI, HTTPException, Request

from .envelope import ok

NAME_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")


class DocStore:
    def __init__(self, data_dir: str | Path) -> None:
        self.data_dir = Path(data_dir)

    def path(self, name: str) -> Path:
        if not NAME_RE.match(name):
            raise HTTPException(
                400,
                f"invalid document name: {name!r} (must match {NAME_RE.pattern})",
            )
        return self.data_dir / f"{name}.json"

    def list(self) -> list[dict[str, Any]]:
        docs: list[dict[str, Any]] = []
        if self.data_dir.exists():
            for p in sorted(self.data_dir.glob("*.json")):
                st = p.stat()
                docs.append(
                    {"name": p.stem, "bytes": st.st_size, "modified": st.st_mtime}
                )
        return docs

    def read(self, name: str) -> Any:
        p = self.path(name)
        if not p.exists():
            raise HTTPException(404, f"no document {name!r}")
        return json.loads(p.read_text())

    def write(self, name: str, value: Any) -> None:
        p = self.path(name)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        tmp = p.with_suffix(".json.tmp")
        tmp.write_text(json.dumps(value, indent=2))
        tmp.replace(p)  # atomic on POSIX

    def delete(self, name: str) -> None:
        self.path(name).unlink(missing_ok=True)


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
