# Playpen Architecture — the `.playpen/` contract

Every playpen lives in a `.playpen/` directory at the target repository root.

## Directory layout

```
.playpen/
  playpen.py               # CLI (uv standalone) — all lifecycle goes through this
  .gitignore               # node_modules/, dist/, run/ (data/ optional)
  server/
    server.py              # FastAPI uv standalone script (port 8765)
    data/                  # JSON document store, one <name>.json per document
  www/                     # Vite + SolidJS project
    package.json           # solid-js + lucide-solid; vite + vite-plugin-solid
    vite.config.js         # strictPort 5173, /api proxy → FastAPI
    index.html
    src/
      index.jsx            # entry: imports forge CSS (tokens first) + playpen.css
      App.jsx              # the playground app — this is what you edit
      api.js               # loadDoc/saveDoc/saveDocDebounced/callAction fetch layer
      playpen.css          # layout classes (pp-shell grid etc.), var(--token) only
      components/          # additional Solid components as the app grows
      forge/               # Forge design system — NEVER EDIT (see provenance below)
        colors_and_type.css
        console.css
        ui.jsx
  run/                     # pidfiles + logs, created by the CLI (gitignored)
```

Scaffold copy mapping (from this skill's `assets/`):

| Source | Destination |
|---|---|
| `assets/server.py` | `.playpen/server/server.py` |
| `assets/playpen.py` | `.playpen/playpen.py` |
| `assets/www/` | `.playpen/www/` |
| `assets/gitignore` | `.playpen/.gitignore` (note the rename) |

## Serving model

**Dev mode (default)** — two daemonized processes:

- FastAPI (uvicorn) on `127.0.0.1:8765` — the `/api` surface + data store.
- Vite dev server on `127.0.0.1:5173` (`--strictPort`) with a `/api` proxy to 8765,
  so the UI is same-origin and CORS never matters. HMR: edits to `www/src/` appear in
  the user's open browser instantly — no rebuild loop while iterating.

**Build mode (`playpen start --build`)** — one process: `vite build` produces
`www/dist/`, FastAPI serves it statically on `:8765`. Use for handoff ("keep this
around") or when Vite dev is undesirable. The static mount is set up at server start,
so after a manual `playpen build` you must `playpen restart --build`.

Decision rule: **dev while iterating, `--build` for handoff.**

## Process model

The CLI daemonizes both processes (`start_new_session`), records pidfiles and logs in
`run/` (`server.pid`, `vite.pid`, `server.log`, `vite.log`), health-checks before
returning, and `stop` kills the whole process group (killing npm alone would orphan
the real Vite child). Stale pidfiles (process died) are detected by `status` and
cleaned by `start`. Never run `uvicorn` or `npm run dev` as foreground shell commands —
they block the agent's shell; always go through the CLI.

## Ports and environment

| What | Default | Override |
|---|---|---|
| FastAPI | 8765 | `PLAYPEN_PORT` env or `--port` |
| Vite dev | 5173 | `PLAYPEN_VITE_PORT` env or `--vite-port` |

Both bind `127.0.0.1` only: no LAN exposure, and `navigator.clipboard` (the copy
button) requires a secure context, which `localhost` satisfies but a LAN IP does not.
Two playpens in different repos collide on the defaults — give the second one the env
vars or flags.

## .gitignore policy

The scaffolded `.playpen/.gitignore` always ignores `www/node_modules/`, `www/dist/`,
and `run/`. Whether `server/data/` is tracked is a per-playpen question — track it when
the data is a work product (annotations, node layouts the user wants kept), ignore it
when it is ephemeral UI state. Ask the user. Also ask whether the repo should ignore
`.playpen/` entirely (throwaway tool) or track it (part of the project).

## Forge provenance — `www/src/forge/`

The three files are verbatim copies of `forge-design/assets/*`, which mirror the
claude.ai design project **"Tech Tools Design System"**
(id `019dc74c-a1ff-74d0-8504-0ad85b5589fe`). Never edit them in a playpen — app-specific
CSS goes in `playpen.css` or component files. Re-sync path: update the forge-design
skill from the design project (DesignSync), then copy
`forge-design/assets/*` over this skill's `assets/www/src/forge/`. If the target repo
has the forge-design skill installed, `diff -u` the two copies before scaffolding and
prefer the newer.

## Known caveats

- `StaticFiles(html=True)` serves `index.html` at `/` but does **not** fall back to it
  for unknown paths — a playpen that adds client-side routes needs a catch-all route in
  `server.py` for build mode (dev mode is unaffected).
- The first `playpen start` runs `npm install` (30–90 s). Give the Bash call a generous
  timeout, or run `uv run .playpen/playpen.py build` first as a separate step.
- `uv run server.py` resolves fastapi/uvicorn into a venv on first run — a few extra
  seconds the CLI's 60 s health-check window already covers.
