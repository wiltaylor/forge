# Handoff: forge-egui — the egui variant of the Forge framework — COMPLETE

**Date:** 2026-07-11 · **Branch:** `main` (uncommitted work in tree) · **Plan:** `/home/wil/.claude/plans/i-want-to-create-rippling-fern.md`

All milestones (M0–M9 UI kit, W1–W7 streaming widgets + demo) are implemented
and verified. Nothing is committed yet. Previous epic's handoff (Tauri) is in
git history.

## What landed

- **`crates/forge-egui`** (workspace member — NOT excluded; eframe has no
  webkit-style system-dep poison, and workspace membership gives it the root
  `ironrdp-session` vendor patch for free) — egui 0.35 port of the design
  system:
  - `theme/`: token-exact palette (accent `#2389E2` dark / `#006BB9` light),
    real-alpha tints (14/16/20%), geometry tokens, embedded OFL fonts
    (feature `fonts`, extra weights as named families), `Theme::apply/of`
    via ctx.data, `with_accent`, locked CVD chart palette.
  - ~60 widgets across primitives/forms/feedback/overlays/structure/data/
    charts/date/specialty — builder + `.show(ui) -> ForgeResponse` with the
    kit-wide `#[must_use] Outcome` contract; value-bound forms; explicit
    `FooState` for real state; AccessKit `widget_info` everywhere (kittest
    queries depend on it).
  - `runtime/`: `run()` over eframe (`App::ui`/`tick`), `Shell` (topbar /
    grouped nav with Ctrl+B rail / status bar), thread-safe Toaster,
    `DialogResult<T>` result-cell dialogs + Ctrl+K palette, particle FX
    (`ctx.fx()`, `Motion` via `FORGE_EGUI_MOTION`, colors from theme or
    caller — egui can't sample pixels, documented divergence).
  - streaming (features `term`/`term-ssh`/`vnc`/`rdp`/`widgets`):
    `stream.rs` bridges forge-core `WidgetStream` over bounded tokio mpsc
    pairs (drop = session close, forge-tauri parity); `rt.rs` owns the tokio
    handle (inject via `rt::set_handle` or lazy leaked 2-worker runtime);
    `term.rs` (vt100 grid, xterm input encoding, focus capture with
    **Ctrl+Shift+Q** release, 150ms resize debounce); `desktop/` (VNC+RDP,
    raw-only rect frames → `TextureHandle::set_partial`, letterbox scale
    modes, browser-convention wheel/mouse, egui::Key→KeyboardEvent.code
    table verified against forge-core keymaps, modifier synthesis).
- **`examples/egui-gallery`** — 18-section living catalogue (tui-gallery
  parity + NodeGraph), `just egui-gallery`. Env-gated self-screenshot:
  `FORGE_GALLERY_SHOT=<png> FORGE_GALLERY_SECTION=<idx>`.
- **`examples/egui-demo`** — native app on forge-core in-process (no HTTP):
  DocStore scratchpad with `valid_doc_name` validation, action invocation
  via `ActionCtx`, live EventBus feed (lag-visible forwarder task + std
  mpsc + `request_repaint`), `Job<T>` spawn/poll bridge (never `block_on`
  on the UI thread), terminal (local + SSH) and VNC/RDP pages. Self-shot:
  `FORGE_DEMO_SHOT`/`FORGE_DEMO_PAGE`/`FORGE_DEMO_SHOT_DELAY`/
  `FORGE_DEMO_AUTOCONNECT=vnc|rdp`.
- **Integration**: root `Cargo.toml` members + `forge-egui` workspace dep;
  justfile `egui-gallery`/`egui-demo`/`egui-test` (in `just test`); README
  "Desktop UIs (forge-egui)" section + crate README with feature table.

## Verified on this box

- `cargo test -p forge-egui --features full`: 99 tests green;
  `--features widgets`: 82 green; clippy `--all-targets` zero warnings; fmt
  clean; no-default-features build is tokio-free (`cargo tree`).
- Live: local PTY e2e (printf marker, `stty size` tracks debounced resize,
  exit-code overlay); SSH e2e against a throwaway sshd container (session +
  wrong-password error path; tests are `#[ignore]`d, container recipe in the
  doc comment); VNC against `just widgets-testenv-up` (fluxbox renders,
  1024×768); RDP `127.0.0.1:3389` (xrdp login dialog at 1280×800, **no
  diagonal shearing** — the vendored stride fix works through the egui
  texture path; xrdp's own session login screen is the known docker caveat).
- All 18 gallery sections self-screenshotted and eyeballed against the
  design system.

## Invariants / gotchas for the next epic

- egui-family crates must share one minor version (currently **0.35**);
  consumers should import egui/eframe through `forge_egui::egui`/`eframe`.
  0.35 API notes: unified `egui::Panel`, eframe `App::ui(&mut Ui, ..)`,
  `ctx.run_ui` for headless frames, `TextEdit::frame(Frame)`, named font
  families panic if unbound (hence `Theme::font(ctx, ..)` checks bindings).
- If `crates/forge-egui` is ever excluded from the workspace it must copy
  the `[patch.crates-io] ironrdp-session` patch like forge-tauri does.
- Hidden/unfocused streaming widgets pause by design (bounded channels;
  engines converge on latest state when the widget resumes draining).
- The `Search` glyph is `◎` — `⌕` exists in none of the bundled fonts;
  symbol coverage comes from JetBrains Mono + egui's Hack appended to the
  proportional fallback chain.
