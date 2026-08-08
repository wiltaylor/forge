"""HTTP routes for component federation.

The filename rule and the manifest live in :mod:`forge_server.core.components`;
this module mounts them at ``/api/components`` and ``/api/components/{file}``.
"""

from __future__ import annotations

from pathlib import Path
from typing import Callable

from fastapi import Depends, FastAPI
from fastapi.responses import FileResponse

from .core.components import (  # re-exported: one rule, one implementation
    ALLOWED_EXTENSIONS,
    FILE_PATTERN,
    Components,
    valid_component_file,
)
from .core.error import NotFound
from .envelope import ok

__all__ = [
    "ALLOWED_EXTENSIONS",
    "Components",
    "FILE_PATTERN",
    "register_routes",
    "valid_component_file",
]


def register_routes(
    app: FastAPI,
    components_dir: str | Path,
    app_name: str,
    require_claims: Callable,
) -> None:
    components = Components(components_dir)

    @app.get("/api/components")
    async def manifest(claims: dict = Depends(require_claims)):
        return ok(components.manifest(app_name))

    # `{file:path}` rather than `{file}`: the ASGI server percent-decodes the
    # request path before routing, so an encoded separator would otherwise
    # make the request miss this route entirely. The filename rule is what
    # must reject a traversal, and it cannot reject what it never sees.
    #
    # It follows that a plainly nested path — `/api/components/a/b.js` — also
    # reaches the rule, and is a 400 here where the Rust route misses and
    # returns 404. Both refuse it; the contract does not say which layer
    # answers. Raised on #40, which gives the filename rule one implementation
    # per language.
    @app.get("/api/components/{file:path}")
    async def bundle(file: str, claims: dict = Depends(require_claims)):
        path = components.file_path(file)
        if not path.is_file():
            raise NotFound(f"no component file {file!r}")
        return FileResponse(path)
