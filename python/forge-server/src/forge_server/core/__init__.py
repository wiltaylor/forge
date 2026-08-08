"""forge_server.core — the transport-free core.

Holds the contract's rules that do not depend on how a request arrives, so
they can be called and tested without a server. Nothing here imports the web
framework: a rule that says no raises :class:`ForgeError`, and the routing
layer maps that to a status. Mirrors the Rust ``forge-core`` crate.
"""

from .components import ALLOWED_EXTENSIONS, FILE_PATTERN, Components, valid_component_file
from .error import BadRequest, ForgeError, Internal, NotFound

__all__ = [
    "ALLOWED_EXTENSIONS",
    "BadRequest",
    "Components",
    "FILE_PATTERN",
    "ForgeError",
    "Internal",
    "NotFound",
    "valid_component_file",
]
