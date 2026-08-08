"""Domain error raised by the transport-free core.

The core states *what* went wrong and which HTTP-shaped status says so; the
routing layer builds the ``{"ok": false, "error": ...}`` envelope. Mirrors
``ForgeError`` in the Rust ``forge-core`` crate.
"""

from __future__ import annotations


class ForgeError(Exception):
    """A contract rule refused. ``status`` is the status the rule maps to."""

    status = 500

    def __init__(self, message: str) -> None:
        super().__init__(message)
        self.message = message


class BadRequest(ForgeError):
    """400 — malformed input (bad doc name, bad component filename, ...)."""

    status = 400


class NotFound(ForgeError):
    """404 — unknown resource, action or file."""

    status = 404


class Internal(ForgeError):
    """500 — anything else (corrupt file on disk, ...)."""

    status = 500
