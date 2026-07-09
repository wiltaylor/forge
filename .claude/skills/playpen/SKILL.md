---
name: playpen
description: Builds server-backed interactive playground apps in a .playpen/ folder — a FastAPI (Python uv) backend with a JSON document store the UI saves data back to, a SolidJS + Vite frontend styled with the Forge design system, and a Python CLI the agent uses to run the app and read back what the user did. Successor to the single-file playground and playground-app skills. Use when the user asks for a playground, playpen, explorer, interactive tool, visual configurator, review UI, or node editor — "make me a playground for X", "let me tweak X visually", "build an interactive review tool".
user-invocable: true
argument-hint: <topic or description>
allowed-tools:
  - Read
  - Write
  - Edit
  - Glob
  - Grep
  - Bash
---

<overview>
Creates interactive playground apps in `.playpen/` at the target repo root: controls on
one side, a live preview that updates instantly, and a natural-language prompt output
with a copy button. Unlike single-file playgrounds these are real apps — a FastAPI uv
server persists JSON documents the UI writes back (comments, annotations, node graphs),
a Vite + SolidJS frontend gives HMR while iterating, and the `playpen.py` CLI lets the
agent start/stop the app, seed data, and read back what the user produced. The UI is
styled with the Forge design system (dark default, honors `prefers-color-scheme`).

Note: the forge repo (github:wiltaylor/forge) now ships this stack as reusable pieces —
`python/forge-server` (the doc-store contract as an installable package, plus JWT auth,
SSE and WebSockets) and `@forge/client` (typed replacement for `www/src/api.js`). For a
throwaway playpen the copy-in scaffold here stays the default; reach for the packages when
a playpen graduates into a real tool or needs auth/live events.
</overview>

<variables>
- `${CLAUDE_SKILL_DIR}`: Path to this skill's directory.
- `$ARGUMENTS`: The user's topic or description (may be empty — then ask).
- CLI: `uv run .playpen/playpen.py <command>` — all lifecycle goes through this.
- Ports: FastAPI 8765, Vite dev 5173 (override `PLAYPEN_PORT` / `PLAYPEN_VITE_PORT`).
</variables>

<templates>
| Template | Use for | File |
|---|---|---|
| design-playground | Visual design decisions (components, layout, color, type) | `${CLAUDE_SKILL_DIR}/templates/design-playground.md` |
| data-explorer | Queries, APIs, pipelines, structured config | `${CLAUDE_SKILL_DIR}/templates/data-explorer.md` |
| document-critique | Doc review with approve/reject/comment | `${CLAUDE_SKILL_DIR}/templates/document-critique.md` |
| diff-review | Code diffs with line-by-line commenting | `${CLAUDE_SKILL_DIR}/templates/diff-review.md` |
| concept-map | Learning/scope maps with draggable nodes | `${CLAUDE_SKILL_DIR}/templates/concept-map.md` |
| code-map | Architecture diagrams with commenting | `${CLAUDE_SKILL_DIR}/templates/code-map.md` |
| node-grid | Node editors with typed ports and connections | `${CLAUDE_SKILL_DIR}/templates/node-grid.md` |
</templates>

<workflow>
<step order="1">
If `$ARGUMENTS` is empty and no task is in context, ask what to build. Pick the
matching template from the table above; if the topic is ambiguous between templates,
ask. If none fits cleanly, use the closest and adapt.
</step>

<step order="2">
Preflight: run `node --version && npm --version && uv --version`. If node or npm is
missing, stop and tell the user playpen needs Node 20+ — do not attempt a no-build
fallback. If `.playpen/` already exists in the repo, ask whether to extend it or
replace it — never overwrite silently.
</step>

<step order="3">
Read `${CLAUDE_SKILL_DIR}/reference/architecture.md` and the chosen template file.
Read `${CLAUDE_SKILL_DIR}/reference/solid-patterns.md` before writing components,
`${CLAUDE_SKILL_DIR}/reference/data-api.md` before touching `server.py`, and
`${CLAUDE_SKILL_DIR}/reference/cli.md` for the full CLI surface and agent recipes.
</step>

<step order="4">
Scaffold — copy (never symlink) per this mapping:
- `${CLAUDE_SKILL_DIR}/assets/server.py` → `.playpen/server/server.py`
- `${CLAUDE_SKILL_DIR}/assets/playpen.py` → `.playpen/playpen.py`
- `${CLAUDE_SKILL_DIR}/assets/www/` → `.playpen/www/`
- `${CLAUDE_SKILL_DIR}/assets/gitignore` → `.playpen/.gitignore` (note the rename)
</step>

<step order="5">
Build the app per the template: edit `.playpen/www/src/App.jsx` (split into
`www/src/components/` as it grows), add app CSS to `www/src/playpen.css` using
`var(--token)` values only, and register custom actions in `server.py`'s `ACTIONS`
dict. Include sensible defaults, 3–5 named presets, and the prompt output + copy
button. Never edit `www/src/forge/*`. Seed initial data with
`uv run .playpen/playpen.py data set <name> --file <json>`.
</step>

<step order="6">
Run: `uv run .playpen/playpen.py start` — the CLI daemonizes both servers and returns.
The first start runs `npm install` (30–90 s): give the Bash call a generous timeout, or
run `uv run .playpen/playpen.py build` as a separate step first. Verify with
`uv run .playpen/playpen.py status`; if something is down, read
`uv run .playpen/playpen.py logs server` (or `logs vite`) and fix before proceeding.
</step>

<step order="7">
Tell the user the URL (`http://localhost:5173`). Iterate by editing `www/src/` — Vite
HMR shows changes live; after editing `server.py`, run
`uv run .playpen/playpen.py restart`. Read back what the user did in the UI with
`uv run .playpen/playpen.py data get <name>` (e.g. `comments`). When wrapping up,
mention `playpen stop`, and `playpen start --build` for a single-process handoff.
</step>
</workflow>

<boundaries>
<always>
- Drive all lifecycle through the CLI — never run `uvicorn` or `npm run dev` as
  foreground shell commands (they block the agent's shell)
- Style with Forge: `var(--token)` for every colour/size/duration, `.fbtn`/`.fcard`/…
  classes and the `ui.jsx` primitives; status via tone triples
- Solid idioms: `class`/`classList`, `splitProps`/`mergeProps`, `Show`/`For`,
  `onInput` — never destructure props
- One `createStore` for control state, debounced persistence via `api.js`
- User-authored output (comments, verdicts, graphs) in its own document so the agent
  can read it back cleanly
- Live preview with no Apply button; prompt output mentions only non-default values
- Verify both themes (toggle `data-theme` on `<html>`)
</always>

<ask>
- Which template, if the topic is ambiguous
- Whether to pre-populate with real data from the codebase
- Before overwriting an existing `.playpen/`
- Whether `server/data/` (or all of `.playpen/`) should be git-tracked
- Before adding runtime deps beyond `solid-js` + `lucide-solid`, or server deps beyond
  `fastapi` + `uvicorn`
- Before adding any server action that writes outside `.playpen/`
- Ports, if 8765/5173 are taken
</ask>

<never>
- Create a single monolithic HTML file — always the `.playpen/` structure
- Use CDN dependencies in the frontend
- Edit `www/src/forge/*` — they mirror the Forge design project
  (id `019dc74c-a1ff-74d0-8504-0ad85b5589fe`); re-sync via the forge-design skill
- Skip the server or bypass the document store with ad-hoc file I/O in the frontend
- Hardcode absolute paths in `server.py` or `playpen.py`
</never>
</boundaries>
