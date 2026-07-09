"""JWT (HS256) authentication per the Forge API contract.

Claims: ``{sub, roles, iat, exp, iss}``. Tokens travel as
``Authorization: Bearer <jwt>`` with a ``?token=`` query-parameter fallback
(header wins) for EventSource / browser WebSocket.

Auth-disabled mode is first-class: with no authenticator configured every
endpoint is open and handlers see anonymous claims.
"""

from __future__ import annotations

import time
from typing import Any

import jwt
from fastapi import FastAPI, HTTPException, Request, WebSocket
from pydantic import BaseModel

from . import config
from .envelope import ok

MIN_SECRET_LEN = 32

ANONYMOUS_CLAIMS: dict[str, Any] = {"sub": "anonymous", "roles": []}


def anonymous_claims() -> dict[str, Any]:
    return {"sub": "anonymous", "roles": [], "iss": None, "exp": None}


class Authenticator:
    """Issues and validates HS256 JWTs against a static user table."""

    def __init__(
        self,
        secret: str,
        users: dict[str, str] | str | None = None,
        ttl: int = config.DEFAULT_TTL_SECS,
        iss: str = config.DEFAULT_ISS,
        verify_iss: bool = False,
    ) -> None:
        if not secret or len(secret) < MIN_SECRET_LEN:
            raise ValueError(
                f"FORGE_JWT_SECRET must be at least {MIN_SECRET_LEN} characters "
                f"(got {len(secret or '')})"
            )
        if isinstance(users, str):
            users = config.parse_users(users)
        self.secret = secret
        self.users = dict(users or {})
        self.ttl = int(ttl)
        self.iss = iss
        # The issuer claim is always emitted but only validated when
        # explicitly configured (contract: "validated only if set explicitly").
        self.verify_iss = verify_iss

    # -- tokens ---------------------------------------------------------

    def issue(self, username: str, roles: list[str] | None = None) -> tuple[str, int]:
        """Return ``(token, expires_at)`` for a user."""
        iat = int(time.time())
        exp = iat + self.ttl
        claims = {
            "sub": username,
            "roles": list(roles or []),
            "iat": iat,
            "exp": exp,
            "iss": self.iss,
        }
        return jwt.encode(claims, self.secret, algorithm="HS256"), exp

    def decode(self, token: str) -> dict[str, Any]:
        """Decode + validate a token; raises HTTPException(401) on failure."""
        kwargs: dict[str, Any] = {}
        if self.verify_iss:
            kwargs["issuer"] = self.iss
        try:
            claims = jwt.decode(
                token,
                self.secret,
                algorithms=["HS256"],
                options={"verify_iss": self.verify_iss},
                **kwargs,
            )
        except jwt.PyJWTError as e:
            raise HTTPException(401, f"invalid token: {e}") from e
        claims.setdefault("roles", [])
        return claims

    # -- credentials ----------------------------------------------------

    def check_credentials(self, username: str, password: str) -> bool:
        secret = self.users.get(username)
        if secret is None:
            # Burn comparable time to avoid a trivial user-enumeration oracle.
            config.verify_password("wrong", password)
            return False
        return config.verify_password(secret, password)


# -- token extraction / dependency ---------------------------------------


def extract_token(authorization: str | None, query_token: str | None) -> str | None:
    """Bearer header wins; ``?token=`` is the fallback."""
    if authorization:
        scheme, _, value = authorization.partition(" ")
        if scheme.lower() == "bearer" and value.strip():
            return value.strip()
    return query_token or None


async def require_claims(request: Request) -> dict[str, Any]:
    """FastAPI dependency: decoded claims, or anonymous claims when auth is off."""
    auth: Authenticator | None = getattr(request.app.state, "forge_auth", None)
    if auth is None:
        return anonymous_claims()
    token = extract_token(
        request.headers.get("authorization"), request.query_params.get("token")
    )
    if not token:
        raise HTTPException(
            401,
            "missing token (use 'Authorization: Bearer <jwt>' or '?token=')",
            headers={"WWW-Authenticate": "Bearer"},
        )
    return auth.decode(token)


def websocket_claims(ws: WebSocket) -> dict[str, Any]:
    """Same as :func:`require_claims` but for a WebSocket handshake."""
    auth: Authenticator | None = getattr(ws.app.state, "forge_auth", None)
    if auth is None:
        return anonymous_claims()
    token = extract_token(
        ws.headers.get("authorization"), ws.query_params.get("token")
    )
    if not token:
        raise HTTPException(401, "missing token")
    return auth.decode(token)


# -- routes ---------------------------------------------------------------


class LoginBody(BaseModel):
    username: str
    password: str


def register_routes(app: FastAPI) -> None:
    @app.post("/api/auth/login")
    async def login(body: LoginBody):
        auth: Authenticator | None = getattr(app.state, "forge_auth", None)
        if auth is None:
            raise HTTPException(404, "auth is disabled (no FORGE_JWT_SECRET configured)")
        if not auth.check_credentials(body.username, body.password):
            raise HTTPException(401, "invalid username or password")
        token, expires_at = auth.issue(body.username)
        return ok(
            {
                "token": token,
                "expires_at": expires_at,
                "user": {"name": body.username, "roles": []},
            }
        )

    @app.get("/api/auth/me")
    async def me(request: Request):
        claims = await require_claims(request)
        return ok(
            {
                "sub": claims.get("sub"),
                "roles": claims.get("roles", []),
                "iss": claims.get("iss"),
                "exp": claims.get("exp"),
            }
        )
