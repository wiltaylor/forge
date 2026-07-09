"""ForgeApp — the public entry point wrapping FastAPI.

.. code-block:: python

    from forge_server import ForgeApp

    app = ForgeApp("my-tool")
    app.auth_from_env()              # optional; open + anonymous claims otherwise
    app.with_docstore("data")
    app.with_events()
    app.with_components("components")
    app.serve_frontend("dist", spa=True)

    @app.action("echo")
    def echo(payload):
        return payload

    app.serve()                      # uvicorn on FORGE_HOST:FORGE_PORT
"""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Callable

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from . import actions as _actions
from . import auth as _auth
from . import components as _components
from . import config
from . import docstore as _docstore
from . import envelope
from . import events as _events
from . import static as _static
from .actions import ActionContext, ActionRegistry
from .docstore import DocStore
from .events import EventBus


class ForgeApp:
    """Lightweight Forge backend implementing the frozen API contract v1."""

    def __init__(self, name: str) -> None:
        config.load_env()
        self.name = name
        self.version = config.VERSION
        self._start = time.time()

        self.fastapi = FastAPI(title=name)  # ASGI escape hatch
        self.fastapi.state.forge_auth = None

        self.actions = ActionRegistry()
        self.events: EventBus | None = None
        self.docstore: DocStore | None = None

        # Routes that must stay at the end of the route table (SPA catch-all).
        self._tail_routes: list = []

        envelope.install_handlers(self.fastapi)
        self._install_cors()
        self._install_health()
        _auth.register_routes(self.fastapi)
        _actions.register_routes(
            self.fastapi, self.actions, self.require_auth, self._action_ctx
        )

    # -- auth -------------------------------------------------------------

    def auth(
        self,
        secret: str,
        users: dict[str, str] | str | None = None,
        ttl: int = config.DEFAULT_TTL_SECS,
        iss: str | None = None,
        verify_iss: bool | None = None,
    ) -> "ForgeApp":
        """Enable JWT auth explicitly. ``iss`` is validated only when passed
        (or when ``verify_iss=True``)."""
        if verify_iss is None:
            verify_iss = iss is not None
        self.fastapi.state.forge_auth = _auth.Authenticator(
            secret=secret,
            users=users,
            ttl=ttl,
            iss=iss if iss is not None else config.DEFAULT_ISS,
            verify_iss=verify_iss,
        )
        return self

    def auth_from_env(self) -> "ForgeApp":
        """Enable JWT auth from FORGE_* env vars; raises if the secret is unset."""
        secret = config.env_str("FORGE_JWT_SECRET")
        if not secret:
            raise RuntimeError(
                "auth_from_env(): FORGE_JWT_SECRET is not set — set it (>= 32 "
                "chars) or skip auth entirely for an open server"
            )
        return self.auth(
            secret=secret,
            users=config.parse_users(config.env_str("FORGE_AUTH_USERS", "") or ""),
            ttl=config.env_int("FORGE_JWT_TTL_SECS", config.DEFAULT_TTL_SECS),
            iss=config.env_str("FORGE_JWT_ISS"),
        )

    @property
    def require_auth(self) -> Callable:
        """Dependency for custom routes: ``claims = Depends(app.require_auth)``.

        Yields decoded claims, or anonymous claims when auth is disabled."""
        return _auth.require_claims

    @property
    def auth_enabled(self) -> bool:
        return self.fastapi.state.forge_auth is not None

    # -- features ----------------------------------------------------------

    def with_docstore(self, data_dir: str | Path | None = None) -> "ForgeApp":
        directory = data_dir or config.env_str("FORGE_DATA_DIR", config.DEFAULT_DATA_DIR)
        self.docstore = DocStore(directory)
        _docstore.register_routes(self.fastapi, self.docstore, self.require_auth)
        self._resort_routes()
        return self

    def with_events(self) -> "ForgeApp":
        self.events = EventBus()
        _events.register_routes(self.fastapi, self.events, self.require_auth)
        self._resort_routes()
        return self

    def with_components(self, components_dir: str | Path | None = None) -> "ForgeApp":
        directory = components_dir or config.env_str(
            "FORGE_COMPONENTS_DIR", config.DEFAULT_COMPONENTS_DIR
        )
        _components.register_routes(self.fastapi, directory, self.name, self.require_auth)
        self._resort_routes()
        return self

    def serve_frontend(self, dist_dir: str | Path, spa: bool = True) -> "ForgeApp":
        self._tail_routes.extend(_static.register_routes(self.fastapi, dist_dir, spa=spa))
        self._resort_routes()
        return self

    # -- actions -------------------------------------------------------------

    def action(self, name: str) -> Callable:
        """Decorator registering a (sync or async) action callable.

        The callable takes the JSON payload, and optionally a second ``ctx``
        argument (claims, events, app)."""

        def decorator(fn: Callable) -> Callable:
            self.actions.register(name, fn)
            return fn

        return decorator

    def _action_ctx(self, claims: dict[str, Any]) -> ActionContext:
        return ActionContext(claims=claims, app=self, events=self.events)

    # -- passthrough route decorators -----------------------------------------

    def get(self, path: str, **kwargs) -> Callable:
        return self._route(self.fastapi.get, path, **kwargs)

    def post(self, path: str, **kwargs) -> Callable:
        return self._route(self.fastapi.post, path, **kwargs)

    def put(self, path: str, **kwargs) -> Callable:
        return self._route(self.fastapi.put, path, **kwargs)

    def delete(self, path: str, **kwargs) -> Callable:
        return self._route(self.fastapi.delete, path, **kwargs)

    def _route(self, method: Callable, path: str, **kwargs) -> Callable:
        inner = method(path, **kwargs)

        def decorator(fn: Callable) -> Callable:
            result = inner(fn)
            self._resort_routes()
            return result

        return decorator

    # -- serving ---------------------------------------------------------------

    def serve(self, host: str | None = None, port: int | None = None) -> None:
        import uvicorn

        uvicorn.run(
            self.fastapi,
            host=host or config.env_str("FORGE_HOST", config.DEFAULT_HOST),
            port=port if port is not None else config.env_int("FORGE_PORT", config.DEFAULT_PORT),
        )

    # -- internals ---------------------------------------------------------------

    def _resort_routes(self) -> None:
        """Keep tail routes (SPA catch-all + static mounts) after everything
        else, so routes registered after serve_frontend() still win."""
        if not self._tail_routes:
            return
        routes = self.fastapi.router.routes
        for route in self._tail_routes:
            if route in routes:
                routes.remove(route)
                routes.append(route)

    def _install_cors(self) -> None:
        origins = config.cors_origins()
        self.fastapi.add_middleware(
            CORSMiddleware,
            allow_origins=origins,
            # Never wildcard-with-credentials: browsers reject it and it is
            # a credential-leak footgun.
            allow_credentials="*" not in origins,
            allow_methods=["*"],
            allow_headers=["Authorization", "Content-Type"],
        )

    def _install_health(self) -> None:
        @self.fastapi.get("/api/health")
        async def health():
            return envelope.ok(
                {
                    "uptime_s": round(time.time() - self._start, 1),
                    "version": self.version,
                    "app": self.name,
                    "auth_enabled": self.auth_enabled,
                    "actions": self.actions.names(),
                }
            )
