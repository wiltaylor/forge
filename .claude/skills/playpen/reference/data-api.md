# Playpen Data API — persistence and custom actions

The FastAPI server (`server/server.py`) ships a generic JSON **document store** plus an
extension point for **custom actions**. Most playpens need zero server changes — the UI
saves through the document store and the agent reads it back with the CLI.

## Endpoints

| Endpoint | Method | Purpose |
|---|---|---|
| `/api/health` | GET | Liveness: uptime, `dist_built`, registered actions |
| `/api/data` | GET | List documents (name, bytes, modified) |
| `/api/data/{name}` | GET | Read one document |
| `/api/data/{name}` | PUT | Create/replace a document (body = any JSON) |
| `/api/data/{name}` | DELETE | Delete a document (idempotent) |
| `/api/actions/{name}` | POST | Run a registered custom action |

Responses use the envelope `{"ok": true, "data": ...}`; errors are FastAPI
`HTTPException`s (`{"detail": "..."}`) with 400/404 status codes.

## Documents

- One document = one file: `server/data/<name>.json`.
- Names must match `^[a-z0-9][a-z0-9_-]{0,63}$` (server rejects anything else with 400
  — this is also the path-traversal guard).
- Writes are atomic (tmp file + rename), so a half-written JSON is never observable.
- The frontend talks to documents only through `www/src/api.js`
  (`loadDoc`/`saveDoc`/`saveDocDebounced`); the CLI mirrors it (`playpen data …`).

Recommended document names per template:

| Template | Documents |
|---|---|
| design-playground, data-explorer | `state` (whole control state) |
| document-critique | `state`, `comments` (the user's approve/reject/comment output) |
| diff-review | `state`, `comments` |
| concept-map, code-map | `nodes`, `connections`, `annotations` |
| node-grid | `nodes`, `connections`, `presets` |

Keep the live control state in `state`; put **user-authored output the agent will read
back** (comments, verdicts, annotations) in its own document so
`playpen data get comments` gives clean output.

## Adding a custom action

For server-side logic the UI can trigger (generate something, validate input, export a
file). Three steps in `server.py`:

```python
def export_palette(payload: dict):
    # payload is the parsed JSON body; return anything JSON-able
    return {"css": render_css(payload["colors"])}

ACTIONS = {
    "echo": echo,
    "export-palette": export_palette,   # ← register it
}
```

The UI calls it via `callAction('export-palette', {colors})` (from `api.js`); the agent
via `playpen call POST /api/actions/export-palette --json '{"colors": [...]}'`.
Unknown actions 404 and list what is registered. After editing `server.py`, run
`uv run .playpen/playpen.py restart`.

## Writing files into the repo (bespoke route)

When an action must write outside `.playpen/` (e.g. "export this config into the
project"), validate the target stays inside the repo:

```python
REPO_ROOT = SERVER_DIR.parent.parent  # .playpen/server → repo root

def export_file(payload: dict):
    target = (REPO_ROOT / payload["path"]).resolve()
    if not target.is_relative_to(REPO_ROOT):
        raise HTTPException(400, "path escapes the repository")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(payload["content"])
    return {"written": str(target.relative_to(REPO_ROOT))}
```

Ask the user before adding any action that writes outside `.playpen/`.

## Concurrency rule

The UI saves with a 500 ms debounce. The CLI uses HTTP whenever the server is up, so
server-side file I/O is the single writer; the CLI's direct-file path only engages when
the server is down. Don't bypass this by editing `server/data/*.json` in an editor
while the server runs — a UI save can overwrite it.
