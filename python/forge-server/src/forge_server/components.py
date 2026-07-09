"""Component federation endpoints: manifest + bundle files."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Callable

from fastapi import Depends, FastAPI, HTTPException
from fastapi.responses import FileResponse

from .envelope import ok

FILE_RE = re.compile(r"^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$")
ALLOWED_EXTENSIONS = {".js", ".mjs", ".css", ".map"}


def validate_filename(file: str) -> None:
    if not FILE_RE.match(file) or ".." in file:
        raise HTTPException(400, f"invalid component filename: {file!r}")
    if Path(file).suffix not in ALLOWED_EXTENSIONS:
        raise HTTPException(
            400,
            f"invalid component filename: {file!r} (allowed extensions: "
            f"{' '.join(sorted(ALLOWED_EXTENSIONS))})",
        )


def register_routes(
    app: FastAPI,
    components_dir: str | Path,
    app_name: str,
    require_claims: Callable,
) -> None:
    directory = Path(components_dir)

    @app.get("/api/components")
    async def manifest(claims: dict = Depends(require_claims)):
        path = directory / "manifest.json"
        if path.is_file():
            try:
                data = json.loads(path.read_text())
            except json.JSONDecodeError as e:
                raise HTTPException(500, f"invalid manifest.json: {e}") from e
        else:
            data = {"components": []}
        data["app"] = app_name
        return ok(data)

    @app.get("/api/components/{file}")
    async def bundle(file: str, claims: dict = Depends(require_claims)):
        validate_filename(file)
        path = directory / file
        if not path.is_file():
            raise HTTPException(404, f"no component bundle {file!r}")
        return FileResponse(path)
