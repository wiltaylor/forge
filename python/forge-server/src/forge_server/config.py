"""Environment configuration (FORGE_* variables) per the Forge API contract."""

from __future__ import annotations

import hmac
import logging
import os

from dotenv import load_dotenv

VERSION = "0.1.0"

log = logging.getLogger("forge_server")

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8765
DEFAULT_TTL_SECS = 86400
DEFAULT_ISS = "forge"
DEFAULT_DATA_DIR = "./data"
DEFAULT_COMPONENTS_DIR = "./components"
DEFAULT_CORS_ORIGINS = ["http://localhost:5173", "http://127.0.0.1:5173"]

_ENV_LOADED = False


def load_env() -> None:
    """Load ``.env`` from the working directory (once; real env vars win).

    The path is explicit: bare ``load_dotenv()`` walks up from this installed
    package's directory, not the process CWD, and would miss the app's file.
    """
    global _ENV_LOADED
    if not _ENV_LOADED:
        load_dotenv(os.path.join(os.getcwd(), ".env"))
        _ENV_LOADED = True


def env_str(name: str, default: str | None = None) -> str | None:
    value = os.environ.get(name)
    return value if value not in (None, "") else default


def env_int(name: str, default: int) -> int:
    raw = env_str(name)
    if raw is None:
        return default
    try:
        return int(raw)
    except ValueError as e:
        raise ValueError(f"{name} must be an integer, got {raw!r}") from e


# -- the contract's variables -------------------------------------------------
#
# The nine FORGE_* variables docs/api-contract.md documents, each read by
# exactly one function here, with its default stated once. Other FORGE_*
# names in this repo (FORGE_TEST_*, the demo and gallery variables) are not
# part of the contract and are not read here.


def jwt_secret() -> str | None:
    """``FORGE_JWT_SECRET`` — no default; unset means auth-disabled mode."""
    return env_str("FORGE_JWT_SECRET")


def auth_users() -> str:
    """``FORGE_AUTH_USERS``, raw — parse with :func:`parse_users`."""
    return env_str("FORGE_AUTH_USERS", "") or ""


def jwt_ttl_secs() -> int:
    """``FORGE_JWT_TTL_SECS`` (default 86400)."""
    return env_int("FORGE_JWT_TTL_SECS", DEFAULT_TTL_SECS)


def jwt_iss() -> str | None:
    """``FORGE_JWT_ISS`` — ``None`` when unset.

    ``DEFAULT_ISS`` is not applied here on purpose: the contract validates
    the issuer only when it is set explicitly, so the caller must see the
    difference between unset and ``"forge"``.
    """
    return env_str("FORGE_JWT_ISS")


def host() -> str:
    """``FORGE_HOST`` (default 127.0.0.1)."""
    return env_str("FORGE_HOST", DEFAULT_HOST)


def port() -> int:
    """``FORGE_PORT`` (default 8765)."""
    return env_int("FORGE_PORT", DEFAULT_PORT)


def data_dir() -> str:
    """``FORGE_DATA_DIR`` (default ./data)."""
    return env_str("FORGE_DATA_DIR", DEFAULT_DATA_DIR)


def components_dir() -> str:
    """``FORGE_COMPONENTS_DIR`` (default ./components)."""
    return env_str("FORGE_COMPONENTS_DIR", DEFAULT_COMPONENTS_DIR)


def cors_origins() -> list[str]:
    """``FORGE_CORS_ORIGINS``, split on commas (default localhost:5173 pair)."""
    raw = env_str("FORGE_CORS_ORIGINS")
    if raw is None:
        return list(DEFAULT_CORS_ORIGINS)
    return [o.strip() for o in raw.split(",") if o.strip()]


def parse_users(raw: str) -> dict[str, str]:
    """Parse ``FORGE_AUTH_USERS``: comma-separated ``user:secret`` entries.

    The FIRST colon splits user from secret; a fragment with no colon
    continues the previous entry's secret. Secrets starting with ``$argon2``
    are PHC hashes; anything else is plaintext and logs a warning.
    """
    users: dict[str, str] = {}
    plaintext: list[str] = []
    last_name: str | None = None
    for entry in raw.split(","):
        entry = entry.strip()
        if not entry:
            continue
        if ":" not in entry:
            # Argon2 PHC hashes contain commas in their params
            # (`$argon2id$v=19$m=19456,t=2,p=1$…`), so a colon-less fragment
            # is the continuation of the previous entry's secret, not a new
            # user (user names always precede a colon).
            if last_name is not None:
                users[last_name] += f",{entry}"
                continue
            raise ValueError(
                f"invalid FORGE_AUTH_USERS entry {entry!r} (expected 'user:secret')"
            )
        name, secret = entry.split(":", 1)
        if not name:
            raise ValueError(f"invalid FORGE_AUTH_USERS entry {entry!r} (empty username)")
        users[name] = secret
        last_name = name
        if not secret.startswith("$argon2"):
            plaintext.append(name)
    if plaintext:
        log.warning(
            "FORGE_AUTH_USERS contains plaintext passwords for: %s — "
            "hash them with `python -m forge_server.hash <password>`",
            ", ".join(plaintext),
        )
    return users


def verify_password(secret: str, password: str) -> bool:
    """Verify ``password`` against a stored secret (argon2 PHC hash or plaintext)."""
    if secret.startswith("$argon2"):
        try:
            from argon2 import PasswordHasher
            from argon2.exceptions import Argon2Error, InvalidHashError, VerifyMismatchError
        except ImportError as e:  # pragma: no cover - depends on extras
            raise RuntimeError(
                "an argon2 password hash is configured but argon2-cffi is not "
                "installed — install the extra: pip install 'forge-server[argon2]'"
            ) from e
        try:
            return PasswordHasher().verify(secret, password)
        except (VerifyMismatchError, InvalidHashError, Argon2Error):
            return False
    return hmac.compare_digest(secret.encode(), password.encode())
