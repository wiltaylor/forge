"""JSON document store: one file per doc, atomic writes.

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
from typing import Any

from .error import BadRequest, Internal, NotFound

NAME_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")


class DocStore:
    def __init__(self, data_dir: str | Path) -> None:
        self.data_dir = Path(data_dir)

    def path(self, name: str) -> Path:
        """Path of a doc, once its name passes the doc-name rule.

        The rule is the path-traversal guard, so the returned path is always
        inside the data directory. Existence is the caller's business.
        """
        if not NAME_RE.match(name):
            raise BadRequest(
                f"invalid document name: {name!r} (must match {NAME_RE.pattern})"
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
            raise NotFound(f"no document {name!r}")
        try:
            return json.loads(p.read_text())
        except json.JSONDecodeError as e:
            # Only a doc written past this store can be corrupt, so this is the
            # server's fault, not the request's — a 500, as for a corrupt
            # manifest.json. Said in the envelope rather than as a stack trace,
            # and worded as the Rust core words it (forge-core/src/docstore.rs).
            raise Internal(f"document {name!r} is corrupt: {e}") from e

    def write(self, name: str, value: Any) -> None:
        p = self.path(name)
        self.data_dir.mkdir(parents=True, exist_ok=True)
        tmp = p.with_suffix(".json.tmp")
        tmp.write_text(json.dumps(value, indent=2))
        tmp.replace(p)  # atomic on POSIX

    def delete(self, name: str) -> None:
        self.path(name).unlink(missing_ok=True)
