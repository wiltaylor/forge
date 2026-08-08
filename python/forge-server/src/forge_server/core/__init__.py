"""forge_server.core — the transport-free core.

Holds the contract's rules that do not depend on how a request arrives, so
they can be called and tested without a server. Nothing here imports the web
framework: a rule that says no raises :class:`ForgeError`, and the routing
layer maps that to a status. Mirrors the Rust ``forge-core`` crate.
"""

from .actions import ActionContext, ActionRegistry
from .components import ALLOWED_EXTENSIONS, FILE_PATTERN, Components, valid_component_file
from .docstore import NAME_RE, DocStore
from .error import BadRequest, ForgeError, Internal, NotFound
from .events import QUEUE_SIZE, EventBus, Subscription

__all__ = [
    "ALLOWED_EXTENSIONS",
    "ActionContext",
    "ActionRegistry",
    "BadRequest",
    "Components",
    "DocStore",
    "EventBus",
    "FILE_PATTERN",
    "ForgeError",
    "Internal",
    "NAME_RE",
    "NotFound",
    "QUEUE_SIZE",
    "Subscription",
    "valid_component_file",
]
