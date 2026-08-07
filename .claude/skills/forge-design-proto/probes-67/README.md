# Probe projects — wayfinder #67

Primary source for [Does guidance-only actually reproduce a control?](https://github.com/wiltaylor/forge/issues/67).
Findings are in `../PROBE-67.md`; this directory is the evidence they rest on.

Six fresh agents built one control each from `.claude/skills/forge-design-proto` alone,
in scratch projects outside the repo, with no access to `packages/` or `crates/`. Each
received a sanitised copy of the skill — every `wayfinder` and `PROTOTYPE` marker stripped
and the frontmatter renamed — so none could tell it was being tested. None was told what
the other five were doing.

Isolation was honour-system, by decision. A probe that peeked cannot be caught.

| Directory | Control | Platform | Brief |
|---|---|---|---|
| `solid-button` | `button` | SolidJS | Three deploy actions, one primary, one destructive, one in-flight state |
| `ratatui-button` | `button` | ratatui | same |
| `egui-button` | `button` | egui | same |
| `solid-combobox` | `combobox` | SolidJS | A region picker over 40 options, two unavailable, unset at open |
| `ratatui-combobox` | `combobox` | ratatui | same |
| `egui-combobox` | `combobox` | egui | same |

`node_modules`, `target/` and the skill copy are stripped. The Rust projects need
`cargo build`; the SolidJS ones need `pnpm install` first.

Two files in here are **not** the probes' work: `ratatui-button/src/lib.rs` and
`ratatui-button/tests/render.rs`, added afterwards so the widget could be rendered through
`TestBackend` for the comparison. `egui-combobox` had `wgpu` and `snapshot` added to its
`egui_kittest` dev-dependency for the same reason.

## `_shots`

| File | What it is |
|---|---|
| `reference-gallery-buttons.png` | The real `@forge/gallery`. Note **Secondary** and **Large**, which `controls/button.md` says do not exist |
| `reference-gallery-combobox.png` | The real combobox, popup open, filtered |
| `probe-solid-button-idle.png` | Built from the page. The accent is invented and `--accent-contrast` is inverted — dark text on the primary |
| `probe-solid-button-loading.png` | The in-flight state. The Deploy button **widens**, which the Contract says cannot happen |
| `probe-solid-combobox.png` | The run's strongest result — near-indistinguishable from the gallery |
| `probe-egui-combobox.png` | Rendered through `kittest`. The active option carries a 2px accent focus ring, the treatment `laws.md` reserves for focus |
