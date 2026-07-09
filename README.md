# Forge

A full-stack application framework: the **Forge design system** (SolidJS,
dark-default, dense technical-tools aesthetic) plus batteries-included
**Rust** and **Python** backends speaking one HTTP contract, **JWT** auth
throughout, global **theming**, and **component federation** between apps.

## Packages (npm, pnpm workspace)

| Package | What | Depends on |
|---|---|---|
| `@forge/tokens` | Design tokens CSS (dark/light) + typed theme engine (`applyTheme`, `defineTheme`) | nothing |
| `@forge/ui` | Core components: shell, forms, overlays, feedback, data | solid-js (peer), tokens |
| `@forge/charts` | Zero-dep SVG charts (pie, line, bar, gantt, sparkline) | solid-js (peer) |
| `@forge/graph` | NodeGraph editor + auto-layout Flowchart | solid-js (peer) |
| `@forge/code` | CodeMirror 6 editor/diff with Forge theme | solid-js (peer), CodeMirror (bundled deps) |
| `@forge/client` | Typed API client: REST + SSE + WebSocket + JWT | nothing |
| `@forge/remote` | Component federation: export web-component bundles, mount remote ones | solid-js (peer), ui; `/vite` helper |

Backends:

- `crates/forge-server` — Rust (axum). REST/WS/SSE, JWT login from `.env`,
  JSON doc store, actions, static serving **with single-binary frontend
  embedding** (rust-embed). The default choice.
- `python/forge-server` — Python (FastAPI), same contract, uv-friendly for
  single-file hack tools.

The contract both implement: [`docs/api-contract.md`](docs/api-contract.md).

## Example apps

- `apps/gallery` — every component + login + live SSE/WS + doc store +
  remote-components demos. Served by both demo backends.
- `apps/remote-widgets` — builds a remote web-component bundle
  (`dist-remote/`) that the demos serve at `/api/components`.
- `examples/rust-demo` — single-binary Rust app embedding the gallery.
- `examples/python-demo` — single-file uv script serving the gallery.
- `examples/parity` — black-box contract tests run against either backend.

## Quick start

```sh
just build            # pnpm packages + cargo build
just rust-demo        # gallery on http://127.0.0.1:8765 (login admin/admin)
just python-demo      # same app on the Python backend
just test             # frontend + rust + python test suites
```

Dev loop for the frontend: `just gallery-dev` (Vite on :5173, proxying `/api`
to :8765 — start a demo backend alongside).

## Using Forge in another app

No npm registry yet — consume via git (pnpm supports subdir git deps):

```jsonc
// package.json
"dependencies": {
  "@forge/ui": "github:wiltaylor/forge#main&path:packages/ui",
  "@forge/tokens": "github:wiltaylor/forge#main&path:packages/tokens"
}
```

Each package carries a `prepare` script, so pnpm builds it on install.
In app CSS entry: import `@forge/tokens/fonts.css` (optional), then
`@forge/tokens/tokens.css`, `@forge/tokens/base.css`, then
`@forge/ui/styles.css`. Vite apps need `vite-plugin-solid` (the packages ship
preserved-JSX source under the `solid` export condition) and
`resolve.dedupe: ['solid-js']`.

Rust:

```toml
forge-server = { git = "https://github.com/wiltaylor/forge" }
```

Python (uv script):

```python
# /// script
# dependencies = ["forge-server"]
# [tool.uv.sources]
# forge-server = { git = "https://github.com/wiltaylor/forge", subdirectory = "python/forge-server" }
# ///
```

> Note: the `@forge` npm scope is taken on npmjs.com — if these packages are
> ever published to a registry they must be renamed (e.g. `@wiltaylor/*`).

## Theming

Everything routes through CSS custom properties (`--bg-0`, `--accent`, …),
dark by default, light via `prefers-color-scheme` or `data-theme`. Programmatic:

```ts
import { applyTheme, defineTheme, darkTheme } from '@forge/tokens';

applyTheme('light');                            // built-in ramps
applyTheme(defineTheme(darkTheme, {             // custom brand
  name: 'ember',
  accent: { base: 'oklch(0.65 0.17 45)', /* … */ },
}));
```

Per-control overrides are CSS vars too: `style={{ '--accent': '…' }}` on any
component recolors just that control. Because custom properties inherit into
shadow DOM, `applyTheme` also restyles **remote components from other apps**.

## Component federation

An app exports components with `defineRemoteElement` + builds a single-file
bundle via `forgeRemoteConfig` (`@forge/remote/vite`); its backend serves the
bundle at `/api/components` behind JWT. A host app calls
`loadRemote('/api/components', { headers: api.auth.header() })` and mounts
`<Remote tag=… props=… on=… />`. Rules: plain values in, CustomEvents out —
never share signals across the boundary.

## Auth

Set in `.env`: `FORGE_JWT_SECRET` (≥32 chars) and `FORGE_AUTH_USERS`
(`admin:admin` for dev, `admin:$argon2id$…` for real — hash via `forge-hash`
or `python -m forge_server.hash`). `POST /api/auth/login` issues an HS256 JWT;
serious deployments skip the built-in login and validate JWTs from an external
auth service sharing the same secret (RS256/JWKS is the planned extension
seam). With no `FORGE_JWT_SECRET`, auth is off and everything is open —
playpen-style local tools.

## Skills

`.claude/skills/forge-design` and `.claude/skills/playpen` remain the Claude
Code skills that build UIs with this system: forge-design documents the
tokens/components (the packages here are the source of truth for the Solid
port), playpen scaffolds server-backed playground apps.
