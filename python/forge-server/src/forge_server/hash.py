"""Argon2id password hashing helper.

Usage: ``python -m forge_server.hash <password>`` — prints a PHC-format
argon2id hash suitable for ``FORGE_AUTH_USERS``. Requires the ``argon2``
extra (``pip install 'forge-server[argon2]'``).
"""

from __future__ import annotations

import sys


def hash_password(password: str) -> str:
    try:
        from argon2 import PasswordHasher
    except ImportError as e:
        raise RuntimeError(
            "argon2-cffi is not installed — install the extra: "
            "pip install 'forge-server[argon2]' (or: uv pip install argon2-cffi)"
        ) from e
    return PasswordHasher().hash(password)


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1:
        print("usage: python -m forge_server.hash <password>", file=sys.stderr)
        return 2
    try:
        print(hash_password(argv[0]))
    except RuntimeError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
