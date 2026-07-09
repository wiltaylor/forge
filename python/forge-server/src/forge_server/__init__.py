"""forge-server — lightweight Python backend implementing the Forge API contract v1."""

from .config import VERSION as __version__
from .app import ForgeApp

__all__ = ["ForgeApp", "__version__"]
