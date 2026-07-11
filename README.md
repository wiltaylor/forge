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
| `@forge/chat` | Chat UI: 1:1/room transcripts, tool-call boxes, interactive prompts, link cards, media, zero-dep markdown, composer | solid-js (peer), ui |
| `@forge/client` | Typed API client: REST + SSE + WebSocket + JWT | nothing |
| `@forge/tauri` | The same `ForgeClient` interface over Tauri IPC + widget transports | client, `@tauri-apps/api` |
| `@forge/remote` | Component federation: export web-component bundles, mount remote ones | solid-js (peer), ui; `/vite` helper |

Backends:

- `crates/forge-server` — Rust (axum). REST/WS/SSE, JWT login from `.env`,
  JSON doc store, actions, static serving **with single-binary frontend
  embedding** (rust-embed). The default choice.
- `python/forge-server` — Python (FastAPI), same contract, uv-friendly for
  single-file hack tools.
- `crates/forge-tauri` — Tauri v2 plugin serving the same contract (and the
  streaming widgets) over **pure IPC** — desktop apps with no HTTP server.
  Shares `crates/forge-core` with forge-server.

The contract all of them implement: [`docs/api-contract.md`](docs/api-contract.md).

Terminal UIs (no contract, standalone): `crates/forge-tui` — the design
system as a ratatui widget kit, see [Terminal UIs](#terminal-uis-forge-tui).

Desktop UIs (no contract, standalone): `crates/forge-egui` — the design
system as an egui widget kit with native streaming widgets, see
[Desktop UIs](#desktop-uis-forge-egui).

## Example apps

- `apps/gallery` — every component + login + live SSE/WS + doc store +
  remote-components demos. Served by both demo backends.
- `apps/remote-widgets` — builds a remote web-component bundle
  (`dist-remote/`) that the demos serve at `/api/components`.
- `examples/rust-demo` — single-binary Rust app embedding the gallery.
- `examples/python-demo` — single-file uv script serving the gallery.
- `examples/tauri-demo` — native Tauri app: doc store, actions, live events,
  local-PTY terminal and VNC/RDP viewers, all over IPC (`just tauri-demo`).
- `examples/parity` — black-box contract tests run against either backend.
- `examples/tui-gallery` — the forge-tui widget catalogue in the terminal
  (`just tui-gallery`).
- `examples/egui-gallery` — the forge-egui widget catalogue as a native
  window (`just egui-gallery`).
- `examples/egui-demo` — native egui app on forge-core directly: doc store,
  actions, live events, terminal and VNC/RDP viewers, all in-process with no
  HTTP (`just egui-demo`).

## Quick start

```sh
just build            # pnpm packages + cargo build
just rust-demo        # gallery on http://127.0.0.1:8899 (login admin/admin)
just python-demo      # same app on the Python backend (http://127.0.0.1:8765)
just test             # frontend + rust + python test suites
```

Dev loop for the frontend: `just gallery-dev` (Vite on :5173, proxying `/api`
to `FORGE_PORT`, default :8765 — start a demo backend alongside; use
`FORGE_PORT=8899 just gallery-dev` to pair with rust-demo).

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

Git-dep gotchas (pnpm; vmlab is the reference consumer):

- pnpm ≥ 10.26 blocks git-dep `prepare` unless the **resolved** URL is
  allowlisted in `onlyBuiltDependencies`
  (`"@forge/ui@https://codeload.github.com/wiltaylor/forge/tar.gz/<rev>#path:packages/ui"`
  — bare package names don't match git deps).
- Add an `overrides` entry pinning `@forge/tokens` to the git dep: `@forge/ui`
  declares it as `workspace:^`, which packs to `^0.1.0` and would otherwise
  resolve against the foreign npmjs `@forge` scope.
- Don't commit the consuming app's `pnpm-lock.yaml`: pnpm 10.34 installs
  `path:` git deps FROM a lockfile as the raw monorepo tarball on a cold
  store (no subdir extraction, no prepare).
- **Never let the pnpm store live under a `node_modules` path.** On GitHub
  Actions, `pnpm/action-setup`'s default store does — pass
  `--store-dir "$RUNNER_TEMP/pnpm-store"` to `pnpm install`. The git-dep
  checkout otherwise builds inside a node_modules path, where TypeScript's
  wildcard matching silently skips source files and the packages ship stub
  `.d.ts` files.

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

## Tauri

`forge-tauri` makes a Tauri v2 desktop app a conforming Forge backend over
pure IPC — the frozen contract (`docs/api-contract.md`) and widgets protocol
(`docs/widgets-protocol.md`) are untouched; only the carrier differs. The
whole backend is a plugin:

```rust
fn main() {
    let forge = forge_tauri::Builder::new("my-app")
        .with_docstore_default()             // <app_data_dir>/data
        .with_term().with_vnc().with_rdp()   // opt-in widgets (cargo features)
        .action("echo", |payload, _ctx| async move { Ok(payload) });
    tauri::Builder::default()
        .plugin(forge.build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Frontend-side, the only change versus a web app is the client import —
`@forge/tauri`'s `createClient()` implements `@forge/client`'s `ForgeClient`
interface over `invoke`/`listen` (`ws.connect()`/`wsUrl()` throw; events ride
one `forge://event` Tauri event). Widgets take a transport instead of a URL:

```jsx
const api = createClient();                            // from '@forge/tauri'
<Terminal transport={() => api.widget('term')} />      // local PTY in-app
<DesktopViewer transport={() => api.widget('vnc')} … />
```

Wiring checklist for your own app (or scaffold via the
`.claude/skills/forge-tauri` skill, which copies `examples/tauri-demo`):

```toml
# src-tauri/Cargo.toml — forge-tauri is NOT in the root workspace; use git
forge-tauri = { git = "https://github.com/wiltaylor/forge", features = ["widgets"] }

# REQUIRED when the `rdp` feature is on: your app is its own workspace, so it
# must carry the vendored ironrdp-session patch itself (xrdp stride fix —
# without it RDP against xrdp shears diagonally):
[patch.crates-io]
ironrdp-session = { git = "https://github.com/wiltaylor/forge" }
```

Capability: add `"forge:default"` (plus `"core:default"`) to
`src-tauri/capabilities/default.json`. Trim widget features you don't use —
they dominate compile time.

Linux prereqs: `webkit2gtk-4.1`, `gtk3`, `librsvg` (Arch/CachyOS package
names; see Tauri docs for other distros). On NVIDIA, if the window renders
blank set `WEBKIT_DISABLE_DMABUF_RENDERER=1`. AppImage bundling needs
`NO_STRIP=true` on distros with recent binutils (the `just tauri-demo-build`
recipe sets it). Inside webkitgtk, xterm.js's WebGL addon composites a black
canvas — pass `webgl={false}` to `<Terminal>` in Tauri apps (the demo does;
web browsers are unaffected). Widgets must also mount in a visible container
(xterm can't initialize at zero size), so lazy-mount tab panels.

## Terminal UIs (forge-tui)

`crates/forge-tui` is the Forge design system for the terminal — a ratatui
0.29 widget kit with the same dark-default, dense, technical aesthetic as the
web components, plus an opt-in app runtime. It is independent of the API
contract and of the web stack: any TUI app can depend on it alone.

- **Theme**: an exact Rust mirror of the web tokens (bg/fg ramps, borders,
  accent, semantic triples; OKLCH converted to sRGB). Degrades cleanly to
  256-color (collision-avoiding quantizer keeps the five near-black
  backgrounds distinct) and 16-color (semantic ANSI mapping). Override via
  struct-update syntax or `Theme::dark().with_accent(color)`; force a mode
  with `FORGE_TUI_COLOR=truecolor|256|16`.
- **Widgets** (~60, all plain ratatui `Widget`/`StatefulWidget`): primitives
  (Button, Badge, Card, Stat, Avatar, Skeleton, …), shell/structure
  (AppShell, Tabs, Pagination, SplitPane, Settings, HelpBar), full forms
  (Input with readline editing, Textarea, Select, ListBox, Slider,
  ToggleGroup, fuzzy Combobox, Calendar/DatePicker), overlays (Modal, Sheet,
  Popover, Tooltip, menus, Ctrl+K command palette), feedback (Toast, Alert,
  Progress, Spinner), data (Table, Logs, Tree, FilePicker, JsonViewer,
  Accordion, KeyValue, Kanban), charts on the locked CVD palette (line, bar,
  donut pie, gantt, sparkline), and specialty widgets behind cargo features:
  `markdown`, `chat` (transcripts, tool-call boxes, composer, prompts),
  `code` (syntect CodeView/DiffView), `term` (embedded PTY terminal) — or
  `full` for everything.
- **Interaction pattern**: every stateful widget pairs a per-frame view with
  a persistent `FooState` whose `handle_key` returns a `#[must_use] Outcome`
  (`Ignored` bubbles like DOM events). No callbacks, no framework.
- **Mouse**: interactive states also expose `handle_mouse` with built-in
  hit-testing (each widget caches its rendered rect) — click to focus/toggle/
  select, wheel to scroll, drag sliders and split dividers, hover to
  highlight menus, click-away to dismiss popups. Capture is on by default
  (`RunOptions { mouse: false, .. }` restores native text selection;
  Shift+drag usually selects even while captured).
- **Runtime (optional)**: `runtime::run(app, theme, opts)` gives you a
  panic-safe terminal guard, tick-driven animation, an immediate-mode
  `FocusRing` (Tab order = render order), a modal overlay stack (Esc closes),
  ready-made dialogs (Confirm/Help/Menu/Palette with result cells), and an
  `mpsc`-backed Toaster any thread can push to.

```rust
use forge_tui::prelude::*;

struct Hello { name: InputState }

impl App for Hello {
    fn draw(&mut self, frame: &mut ratatui::Frame, ctx: &mut Ctx) {
        let focused = ctx.focus.register(FocusId::new("name"));
        let input = Input::new().placeholder("Who?").focused(focused).theme(&ctx.theme);
        frame.render_stateful_widget(input, frame.area(), &mut self.name);
    }
    fn on_event(&mut self, event: Event, ctx: &mut Ctx) {
        if let Event::Key(key) = event {
            if self.name.handle_key(key) == Outcome::Submitted {
                ctx.toast().success(format!("Hello {}", self.name.value()));
            }
        }
    }
}
```

`just tui-gallery` runs the living catalogue (one section per widget family,
mirroring `apps/gallery`); `just tui-test` runs the suite with all features.

## Desktop UIs (forge-egui)

`crates/forge-egui` is the Forge design system for native desktop apps — an
egui 0.35 widget kit with the same dark-default, dense, technical aesthetic,
an app runtime over eframe, and (optionally) the streaming widgets driven by
forge-core's engines entirely in-process. Like forge-tui it is independent of
the API contract; unlike Tauri there is no webview and no JS anywhere.

- **Theme**: the exact web tokens (bg/fg ramps, borders, accent `#2389E2`,
  semantic triples) with real alpha tints, plus the geometry tokens a pixel
  canvas can express (radii, 4pt spacing, type scale, control heights,
  motion durations). IBM Plex Sans + JetBrains Mono are embedded behind the
  default-on `fonts` feature (SIL OFL, see `LICENSES/`). Install once with
  `Theme::apply(ctx)` — it also maps onto egui's `Style`/`Visuals` so
  third-party egui widgets look approximately right; swap themes at runtime
  with `Ctx::set_theme`. Override via struct-update syntax or
  `Theme::dark().with_accent(color)`.
- **Widgets** (~60): one shape everywhere — builder + `.show(ui)` returning
  a `ForgeResponse` whose `#[must_use] Outcome` mirrors the kit-wide
  contract (`Ignored/Consumed/Changed/Submitted/Cancelled`). Value-bound
  forms borrow your data (`Input::new(&mut name)`); widgets with real state
  pair with an explicit `FooState` you own. Primitives, full forms
  (Select/Combobox/ListBox/Slider/ToggleGroup, indeterminate Checkbox),
  overlays (Modal 480/720/960, Sheet, Popover, menus, tooltips), feedback
  (Alert/Progress/Spinner + a thread-safe Toaster), structure (Tabs,
  Pagination, SplitPane, Settings rows, Crumbs, PageHead), data (sortable
  Table, follow-mode Logs, Tree, JsonViewer, FilePicker, drag-and-drop
  Kanban, BlockGrid), charts on the locked CVD palette (bar/line/donut/
  gantt/sparkline + niceTicks), particle FX (`ctx.fx().explode(rect)`,
  Motion-gated via `FORGE_EGUI_MOTION`), and behind features: `calendar`
  (Calendar/DatePicker), `markdown`, `chat` (transcripts, tool-call boxes,
  prompts, composer), `code` (syntect CodeView/DiffView + LSP-style
  annotations), a NodeGraph/Flowchart pair, or `full` for all UI features.
- **Runtime (optional)**: `forge_egui::run(app, theme, opts)` wraps eframe —
  `App::ui`/`App::tick`, the `Shell` app frame (topbar, grouped sidebar nav
  with Ctrl+B rail collapse, status bar), result-cell dialogs
  (`ctx.confirm_danger(..)` polled via `DialogResult::take`), a Ctrl+K
  command palette, and toasts any thread can push.
- **Streaming widgets** (features `term`/`term-ssh`/`vnc`/`rdp`, or
  `widgets`): the embedded terminal (vt100 grid over forge-core's PTY/SSH
  engines) and VNC/RDP viewers (dirty-rect texture updates) run over an
  in-process channel bridge — no server, no websocket. Click a well to
  capture the keyboard; **Ctrl+Shift+Q** releases. Sessions spawn onto an
  injected tokio handle (`forge_egui::rt::set_handle`) or a lazy internal
  runtime.

```rust
use forge_egui::prelude::*;

struct Hello { name: String }

impl App for Hello {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx) {
        if Input::new(&mut self.name).label("Who?").show(ui).submitted() {
            ctx.toast().success(format!("Hello {}", self.name));
        }
    }
}

fn main() -> forge_egui::Result<()> {
    forge_egui::run(Hello { name: String::new() }, Theme::dark(), RunOptions::default())
}
```

`just egui-gallery` runs the living catalogue; `just egui-demo` runs the
backend-integration demo (forge-core doc store, actions, and events consumed
in-process, plus the terminal and VNC/RDP pages against
`just widgets-testenv-up`); `just egui-test` runs the suite.

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
auth service sharing the same secret. With no `FORGE_JWT_SECRET`, auth is off
and everything is open — playpen-style local tools.

For a full identity provider, `crates/forge-auth` ships a self-hosted OIDC IdP:
discovery, authorization code + PKCE, RS256/JWKS, RFC 8693 token exchange
(including legacy HS256 minting for unmodified forge apps), username/password +
upstream-OIDC + LDAP federation, and an admin console (`apps/auth`). See
`crates/forge-auth/README.md`; `examples/auth-demo` is a minimal relying party.

## Skills

`.claude/skills/forge-design` and `.claude/skills/playpen` remain the Claude
Code skills that build UIs with this system: forge-design documents the
tokens/components (the packages here are the source of truth for the Solid
port), playpen scaffolds server-backed playground apps.
