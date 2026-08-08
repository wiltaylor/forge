"""Static frontend serving with SPA fallback.

A plain ``StaticFiles(html=True)`` mount has no SPA fallback (unknown paths
404), so this registers an explicit catch-all that returns ``index.html``
for unknown non-``/api`` paths. ``/api/*`` misses stay JSON 404 envelopes.

The catch-all takes every method, not only GET, because the contract says an
``/api`` miss is a 404 envelope whatever the method — a ``PUT`` to
``/api/data/a/b`` is a route that does not exist, not a method that is not
allowed. A method that *is* wrong for a route that does exist still gets its
405: see :func:`_method_mismatch`.
"""

from __future__ import annotations

from pathlib import Path

from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import FileResponse
from starlette.routing import BaseRoute, Match
from starlette.staticfiles import StaticFiles

# Everything a client can send. The catch-all answers them all, so that a miss
# is a miss whatever the method.
METHODS = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]

# Methods the static frontend serves. Anything else on a non-/api path is a
# 404, which is what the Rust backend's fallback returns.
SERVED = {"GET", "HEAD"}


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

    @app.api_route("/{full_path:path}", methods=METHODS, include_in_schema=False)
    async def spa_catch_all(request: Request, full_path: str):
        # A path another route serves under a different method is a 405, and
        # this catch-all must not turn it into a 404.
        if _method_mismatch(request):
            raise HTTPException(405, "Method Not Allowed")
        # /api misses must stay JSON 404 envelopes, never index.html.
        if full_path == "api" or full_path.startswith("api/"):
            raise HTTPException(404, f"no such API route: /{full_path}")
        if request.method not in SERVED:
            raise HTTPException(404, f"not found: /{full_path}")
        if full_path:
            candidate = (dist / full_path).resolve()
            if candidate.is_relative_to(dist) and candidate.is_file():
                return FileResponse(candidate)
        index = dist / "index.html"
        if (spa or not full_path) and index.is_file():
            return FileResponse(index)
        raise HTTPException(404, f"not found: /{full_path}")

    return list(app.router.routes[before:])


def _method_mismatch(request: Request) -> bool:
    """Does another route serve this path under a different method?

    Starlette answers a partial match — right path, wrong method — with 405,
    but only when no route matches fully. The catch-all matches every path
    fully, so it has to ask the question itself.
    """
    for route in request.app.router.routes:
        match, _ = route.matches(request.scope)
        if match is Match.PARTIAL:
            return True
    return False
