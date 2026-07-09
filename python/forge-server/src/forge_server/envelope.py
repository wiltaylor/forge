"""Response envelope helpers and exception handlers.

Every JSON response uses one envelope:

- success: ``{"ok": true, "data": <any>}`` — mutations may omit ``data``
- failure: ``{"ok": false, "error": "<message>"}`` with a meaningful status
"""

from __future__ import annotations

from typing import Any

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from starlette.exceptions import HTTPException as StarletteHTTPException

_UNSET = object()


def ok(data: Any = _UNSET) -> dict:
    """Success envelope. ``ok()`` (no argument) omits ``data`` for mutations."""
    if data is _UNSET:
        return {"ok": True}
    return {"ok": True, "data": data}


def fail(message: str, status: int = 400, headers: dict | None = None) -> JSONResponse:
    """Failure envelope as a JSONResponse."""
    return JSONResponse(
        {"ok": False, "error": message}, status_code=status, headers=headers
    )


def install_handlers(app: FastAPI) -> None:
    """Convert HTTPExceptions and request-validation errors to the envelope."""

    @app.exception_handler(StarletteHTTPException)
    async def _http_exception(request: Request, exc: StarletteHTTPException):
        detail = exc.detail if isinstance(exc.detail, str) else str(exc.detail)
        return fail(detail, status=exc.status_code, headers=getattr(exc, "headers", None))

    @app.exception_handler(RequestValidationError)
    async def _validation_error(request: Request, exc: RequestValidationError):
        parts = []
        for err in exc.errors():
            loc = ".".join(str(p) for p in err.get("loc", ()))
            parts.append(f"{loc}: {err.get('msg', 'invalid')}" if loc else err.get("msg", "invalid"))
        return fail("invalid request: " + "; ".join(parts) if parts else "invalid request", status=422)
