# Playpen CLI — `uv run .playpen/playpen.py <command>`

The CLI is the only supported way to run, stop, and inspect a playpen. It daemonizes
the servers (so the agent's shell never blocks), manages pidfiles/logs in
`.playpen/run/`, and gives direct access to the persisted data.

## Commands

| Command | What it does |
|---|---|
| `start [--build] [--port N] [--vite-port N]` | Start FastAPI; in dev mode (default) also npm-install (first run) and start Vite with HMR. `--build` = vite build, serve dist from FastAPI alone. Prints the UI URL. |
| `stop` | SIGTERM (then SIGKILL) both process groups, remove pidfiles. |
| `restart [--build] …` | `stop` + `start`. Required after editing `server.py`. |
| `status` | JSON: pids, liveness, stale-pidfile flags, `/api/health` result, URLs. |
| `logs [server\|vite] [-n N]` | Print the last N (default 40) lines of a daemon log. |
| `build` | `npm install` (if needed) + `vite build` → `www/dist/`. |
| `data list` | List persisted documents. |
| `data get NAME` | Print a document as JSON. |
| `data set NAME --json '{"x":1}'` | Write a document (also `--file f.json`, or `--file -` for stdin). |
| `data delete NAME` | Delete a document. |
| `call METHOD PATH [--json S]` | Arbitrary API call, e.g. `call POST /api/actions/echo --json '{"a":1}'`. |

Ports come from `--port`/`--vite-port` or `PLAYPEN_PORT`/`PLAYPEN_VITE_PORT` env vars
(defaults 8765/5173).

## Offline fallback

`data` commands use the HTTP API when the server is up. When it is down they fall back
to direct file access in `server/data/` — so the agent can always read what the user
produced, even after `playpen stop`. Writes prefer HTTP for the same reason the UI
does: one writer. (Offline `data set` writes the file directly and atomically.)

## Agent recipes

**Read back what the user did in the UI** (comments, verdicts, annotations):

```
uv run .playpen/playpen.py data get comments
```

**First start** (npm install can take 30–90 s — use a generous Bash timeout, or split):

```
uv run .playpen/playpen.py build     # slow part, separate call
uv run .playpen/playpen.py start     # fast now
```

**After editing `server.py`** (new action, new route):

```
uv run .playpen/playpen.py restart
```

(Editing `www/src/*` needs nothing — Vite HMR picks it up.)

**Server won't start / UI errors:**

```
uv run .playpen/playpen.py status
uv run .playpen/playpen.py logs server -n 50
uv run .playpen/playpen.py logs vite -n 50
```

`status` reporting `"stale_pidfile": true` means the process died — check the log, then
`start` (it cleans stale pidfiles itself).

**Seed the UI with data before the user opens it:**

```
uv run .playpen/playpen.py data set nodes --file /tmp/nodes.json
```

**Smoke-test an action:**

```
uv run .playpen/playpen.py call POST /api/actions/echo --json '{"ping": 1}'
```
