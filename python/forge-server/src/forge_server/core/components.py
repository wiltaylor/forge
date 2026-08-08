"""Component federation: the bundle-filename rule and the manifest.

The manifest is ``manifest.json`` in the components directory, served with the
application name injected. Bundle filenames must match
``^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$``, hold no ``..``, and end in one of
``.js .mjs .css .map`` — the rule doubles as the path-traversal guard.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from .error import BadRequest, Internal

#: The bundle-filename pattern shared by every error message and validator.
FILE_PATTERN = r"^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$"
#: Extensions a bundle file may carry.
ALLOWED_EXTENSIONS = (".js", ".mjs", ".css", ".map")

_FILE_RE = re.compile(FILE_PATTERN)


def valid_component_file(name: str) -> bool:
    """Validate a bundle filename per the contract."""
    if not _FILE_RE.fullmatch(name) or ".." in name:
        return False
    return name.endswith(ALLOWED_EXTENSIONS)


class Components:
    """Filesystem-backed component federation directory."""

    def __init__(self, directory: str | Path) -> None:
        self.directory = Path(directory)

    def manifest(self, app: str) -> Any:
        """The federation manifest with ``app`` injected.

        No ``manifest.json`` is an empty catalogue — ``{app, components: []}``
        — not a 404: the contract states one response shape for this endpoint
        and names no error status for it, unlike the endpoints where a miss is
        a 404 (``/api/data/{name}``, ``/api/actions/{name}``).
        """
        path = self.directory / "manifest.json"
        if not path.is_file():
            return {"app": app, "components": []}
        try:
            data = json.loads(path.read_text())
        except json.JSONDecodeError as e:
            raise Internal(f"manifest.json is not valid JSON: {e}") from e
        if isinstance(data, dict):
            return {**data, "app": app}
        if isinstance(data, list):
            # An array manifest is treated as the components list.
            return {"app": app, "components": data}
        return data

    def file_path(self, name: str) -> Path:
        """Path of a bundle file, once its name passes the filename rule.

        The rule is the path-traversal guard, so the returned path is always
        inside the components directory. Existence is the caller's business.
        """
        if not valid_component_file(name):
            raise BadRequest(
                f"invalid component file name: {name!r} (must match "
                f"{FILE_PATTERN}, extensions {' '.join(ALLOWED_EXTENSIONS)})"
            )
        return self.directory / name
