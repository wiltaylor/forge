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
