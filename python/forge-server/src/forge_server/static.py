"""Static frontend serving with SPA fallback.

A plain ``StaticFiles(html=True)`` mount has no SPA fallback (unknown paths
404), so this registers an explicit catch-all GET that returns ``index.html``
for unknown non-``/api`` paths. ``/api/*`` misses stay JSON 404 envelopes.
"""

from __future__ import annotations

from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from starlette.routing import BaseRoute
from starlette.staticfiles import StaticFiles


def register_routes(
    app: FastAPI, dist_dir: str | Path, spa: bool = True
) -> list[BaseRoute]:
    """Register static-serving routes; returns the routes so the caller can
    keep them at the end of the route table (they include a catch-all)."""
    dist = Path(dist_dir).resolve()
    before = len(app.router.routes)

    assets = dist / "assets"
    if assets.is_dir():
        app.mount("/assets", StaticFiles(directory=assets), name="forge-assets")

    @app.get("/{full_path:path}", include_in_schema=False)
    async def spa_catch_all(full_path: str):
        # /api misses must stay JSON 404 envelopes, never index.html.
        if full_path == "api" or full_path.startswith("api/"):
            raise HTTPException(404, f"no such API route: /{full_path}")
        if full_path:
            candidate = (dist / full_path).resolve()
            if candidate.is_relative_to(dist) and candidate.is_file():
                return FileResponse(candidate)
        index = dist / "index.html"
        if (spa or not full_path) and index.is_file():
            return FileResponse(index)
        raise HTTPException(404, f"not found: /{full_path}")

    return list(app.router.routes[before:])
