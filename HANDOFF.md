# Handoff: First-class Tauri support for Forge — COMPLETE

**Date:** 2026-07-10 · **Branch:** `main` (uncommitted work in tree) · **Plan:** `/home/wil/.claude/plans/can-you-add-an-glowing-popcorn.md`

All 7 phases are implemented and verified. Nothing is committed yet.

## What landed

- **`crates/forge-core`** (workspace member) — transport-agnostic extraction
  from forge-server: DocStore, EventBus, Claims, ForgeError, actions,
  envelope/health builders, and the widget session engines (term/ssh/vnc/rdp)
  generic over the new `WidgetStream` trait (`WidgetMsg::{Text,Binary,Close}`,
  explicit RPITIT + Send). forge-server keeps thin `ws_handler`s + a
  `WsStream(WebSocket)` adapter; its widget features are passthroughs.
- **`crates/forge-tauri`** (excluded from root workspace, own ironrdp patch)
  — Tauri v2 plugin: `Builder` mirroring `ForgeApp`, one `plugin:forge|request`
  command routed by pure `bridge::handle()`, `forge://event` event bridge,
  and `widget_open/send_text/send_binary/close` commands driving the
  forge-core engines over `(Channel out, mpsc in)` streams. ACL naming:
  crate `forge-tauri` + `links = "tauri-plugin-forge"` → plugin name "forge"
  (app side derives from links; plugin side only needs underscore-free name).
- **`packages/tauri`** (`@forge/tauri`) — `createClient(): ForgeClient &
  {widget(kind)}` over invoke/listen; ws/wsUrl throw; widget transports chain
  sends on a promise queue (invoke resolution is unordered); channel frames:
  string = control, ArrayBuffer = payload, null = close.
- **`@forge/term` / `@forge/desktop`** — additive
  `transport?: WidgetTransport | (() => WidgetTransport)` prop; `url` now
  optional; `connectTransport()` helper wraps `new WebSocket(url)` by default;
  gallery untouched.
- **`examples/tauri-demo`** — Overview (health/notes/events) + Terminal
  (local PTY) + Desktop (VNC/RDP) tabs, identifier
  `dev.wiltaylor.forge.tauridemo`, icons committed, `examples/*` added to
  pnpm-workspace.
- **justfile** (`tauri-build/tauri-test/tauri-demo/tauri-demo-build`),
  **README** ("## Tauri" section + package/example rows),
  **`.claude/skills/forge-tauri`** (scaffold-by-copy skill + scaffold.md
  transform table + ipc-patterns.md primer).

## Verification results (all passed)

- Root stays light: members = forge-core/forge-server/rust-demo; zero
  tauri/webkit in the root tree (ironrdp appears only via rust-demo's
  pre-existing explicit `widgets` opt-in).
- `cargo test` (13 suites), `cargo test --features widgets -p forge-server`
  (incl. term/desktop/widgets WS integration tests), parity 25/25 against a
  fresh rust-demo, live docker gates `vnc_live_frames_round_trip` +
  `rdp_live_basic_connect` (vendored xrdp stride fix exercised).
- `just tauri-test`: 13 tests (bridge parity set + PTY loopback through a
  real closure-backed `Channel`).
- `pnpm build && pnpm typecheck && pnpm test`: 15 turbo tasks green
  (incl. 20 new @forge/tauri vitest cases).
- Real-window E2E on this box: launched tauri-demo, screenshot shows health
  stats + live tick events over IPC; process tree shows
  `tauri-demo → {WebKitWebProcess, bash(pts/16)}` = the Terminal tab's
  auto-connected PTY session through the real webview ACL/invoke/channel
  path. Release bundling (`pnpm tauri build` → deb/AppImage) run at the end.

## Invariants (unchanged)

- `docs/api-contract.md` + `docs/widgets-protocol.md` frozen; parity suite is
  the regression gate.
- Excluded tauri crates each carry their own
  `[patch.crates-io] ironrdp-session` (root patch does not reach them).
- `encode_rect` forces alpha 0xFF; `@forge` npm scope not published (git deps).
