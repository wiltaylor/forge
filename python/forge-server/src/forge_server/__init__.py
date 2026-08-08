"""forge-server — lightweight Python backend implementing the Forge API contract v1.

``ForgeApp`` and ``__version__`` are resolved on first use rather than on
import. Importing any submodule runs this file first, so an eager import here
would drag the web framework into ``forge_server.core``, which exists to be
callable without it.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:  # pragma: no cover - for type checkers and IDEs only
    from .app import ForgeApp

__all__ = ["ForgeApp", "__version__"]


def __getattr__(name: str) -> Any:
    if name == "ForgeApp":
        from .app import ForgeApp

        return ForgeApp
    if name == "__version__":
        from .config import VERSION

        return VERSION
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__() -> list[str]:
    return sorted(__all__)
