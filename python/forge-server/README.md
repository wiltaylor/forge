# forge-server (Python)

Lightweight Python backend for Forge hack tools. Implements the frozen Forge
API contract v1 (`docs/api-contract.md`). The Rust crate is the serious
default; this package is for quick FastAPI-based tools.

```python
from forge_server import ForgeApp

app = ForgeApp("my-tool")
app.with_docstore("data")
app.with_events()
app.serve_frontend("dist")

@app.action("echo")
def echo(payload):
    return payload

app.serve()
```

Auth is opt-in: `app.auth_from_env()` (requires `FORGE_JWT_SECRET`) or
`app.auth(secret=..., users=...)`. With no auth configured everything is open
and handlers see anonymous claims.

Password hashes: `python -m forge_server.hash <password>` (requires the
`argon2` extra).

## Layout

`forge_server.core` holds the contract's rules — the doc store, the action
registry, the event bus and component federation. It imports no web framework:
a rule that says no raises a `ForgeError` (`BadRequest`, `NotFound`,
`Internal`), and the routing layer maps that to a status. So the rules are
callable, and testable, with FastAPI absent:

```python
from forge_server.core import DocStore, NotFound

store = DocStore("data")
store.write("notes", {"a": 1})
```

The modules beside it — `docstore.py`, `actions.py`, `events.py`,
`components.py` and `static.py` — are the routing layer, and `app.py` wires
them onto FastAPI. `auth.py` is not unwelded yet: it still holds its rules and
its routes together. This mirrors the Rust `forge-core` / `forge-server`
split.
