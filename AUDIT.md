# The control-by-platform grid

Audit for [#72](https://github.com/wiltaylor/forge/issues/72), against the source at
`85ed9c8`, before anything is deleted.

Method: read the export surface of every module, then match on **what the widget is**,
not what the file is called. Filename matching is what gave #70 its 33–37 holes per Rust
platform, and that number is wrong by an order of magnitude.

---

## Headline

| | |
|---|---|
| Canonical controls in settled scope | **73** (not 79) |
| Cells in the grid | **219** (73 x 3) |
| Empty cells | **9** |
| Grid fullness | **96%** |
| Empty cells that are *not possible* | **1** (`file-picker` on SolidJS) |
| Reverse gaps (Rust-only, invisible to a SolidJS-derived list) | **6** |

The Rust platforms are not the laggards. **egui is missing one control. ratatui is
missing two. SolidJS is missing six.**

---

## 1. The canonical list — 73 controls

The three platforms spell the same widget differently, so each control gets one canonical
kebab-case name. The name is the page's address under #64's path grammar.

### What counts as a control

A control earns a page when it has **its own contract** — its own state, its own keys, or
its own class vocabulary. Anatomy with none of those folds into its parent, and the fold
is recorded here so nothing is lost.

**Folded (8 SolidJS exports):**

| Export | Folds into | Why |
|---|---|---|
| `NavSection`, `NavLink` | `app-shell` | The shell owns nav focus; neither has state or keys. egui's `NavItem` is the same fold. |
| `SettingsSection`, `SettingsRow` | `settings-layout` | Layout slots. Both Rust platforms export them as builders — a Mechanism difference, not a control. |
| `Radio` | `radio-group` | Roving tabindex belongs to the group. Both Rust platforms have only `RadioGroup`. |
| `LogLine` | `logs` | A row renderer. Both Rust platforms also export `LogLine`; it is still anatomy. |
| `ChatDivider`, `ChatTyping` | `chat-view` | No state, no keys. Both Rust platforms model them as `ChatItem` variants. |

**Excluded as infrastructure, not controls:** `ThemeProvider`/`useTheme`,
`OverlayMountProvider`/`useOverlayMount`, `toast`/`dismissToast`/`createToaster` (the API
of `toaster`), `layoutFlow`, `parseMarkdown`/`safeUrl`, `LANGUAGES`/`forgeTheme`,
`seriesColor`/`niceTicks`/`PORT_COLORS`, Rust `FormState`, `FocusRing`, `OverlayStack`.

### Where the count moved

#70's per-package figures do not survive contact with the export lists:

| Package | #70 said | Actually | Why |
|---|---|---|---|
| `ui` | 57 | **56** | 58 exported components; two are providers. Then 8 fold, and 6 Rust-only controls join. |
| `chat` | 9 | 7 | 9 exports, 2 fold into `chat-view`. |
| `charts` | 6 | 6 | 5 public + `Legend`, which SolidJS keeps private but does render. |
| `code` | 4 | 2 | `LANGUAGES` and `forgeTheme` are not controls. |
| `graph` | 3 | 2 | `layoutFlow` is a function. |

`ui` at 56 is: 14 primitives + 9 structure + 10 forms + 2 date + 7 overlays + 5 feedback +
8 data + 1 effects. Total across packages: 56 + 7 + 6 + 2 + 2 = **73**.

---

## 2. The grid

`Y` present · `—` absent · `!` absent and not possible · `(y)` present but not a public export

### Primitives (14)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `icon` | Y | Y | Y |
| `button` | Y | Y | Y |
| `icon-button` | Y | Y | Y |
| `badge` | Y | Y | Y |
| `card` | Y | Y | Y |
| `stat` | Y | Y | Y |
| `kbd` | Y | Y | Y |
| `status-dot` | Y | Y | Y |
| `separator` | Y | Y | Y |
| `skeleton` | Y | Y | Y |
| `avatar` | Y | Y | Y |
| `eyebrow` | Y | Y | Y |
| `empty` | Y | Y | Y |
| `grid` | Y | Y | Y |

### Structure (9)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `app-shell` | Y | Y | Y |
| `crumbs` | Y | Y | Y |
| `page-head` | Y | Y | Y |
| `tabs` | Y | Y | Y |
| `pagination` | Y | Y | Y |
| `split-pane` | Y | Y | Y |
| `settings-layout` | Y | Y | Y |
| `status-bar` | **—** | Y | Y |
| `help-bar` | **—** | Y | **—** |

`app-shell` lives in `runtime/shell.rs` on both Rust platforms, not under `widgets/`.
SolidJS `AppShell` takes `topbar`/`sidebar`/`children` and has **no status slot**; both
Rust shells carry `.status()` and `.status_right()`.

### Forms (10)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `input` | Y | Y | Y |
| `textarea` | Y | Y | Y |
| `checkbox` | Y | Y | Y |
| `toggle` | Y | Y | Y |
| `radio-group` | Y | Y | Y |
| `select` | Y | Y | Y |
| `list-box` | Y | Y | Y |
| `slider` | Y | Y | Y |
| `toggle-group` | Y | Y | Y |
| `combobox` | Y | Y | Y |

Full parity, all three.

### Date (2)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `calendar` | Y | Y | Y |
| `date-picker` | Y | Y | Y |

ratatui keeps both in one `date/mod.rs`; egui splits them. A file-name match reads that as
a hole. It is not one.

### Overlays (7)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `modal` | Y | Y | Y |
| `sheet` | Y | Y | Y |
| `tooltip` | Y | Y | Y |
| `popover` | Y | Y | Y |
| `dropdown-menu` | Y | Y | Y |
| `context-menu` | Y | Y | Y |
| `command-palette` | Y | Y | Y |

Three shape differences worth a Mechanism note, none of them gaps:

- ratatui makes `dropdown-menu` and `context-menu` **one widget** — `menu.rs` says so
  outright, and the constructor differs only in the anchor (a rect versus a point).
- egui exposes `context_menu` as a free function and `tooltip` as a free function, while
  the rest are builder structs.
- `command-palette` is a **widget** on ratatui (`overlays/command_palette.rs`) but a
  **runtime dialog** on egui (`ctx.open_palette(...)` returning a `DialogResult<usize>`).
  Same control, opposite ends of the composability scale.

### Feedback (5)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `alert` | Y | Y | Y |
| `progress` | Y | Y | Y |
| `spinner` | Y | Y | Y |
| `toast` | Y | Y | Y |
| `toaster` | Y | Y | Y |

`toaster` is `runtime/toaster.rs` on both Rust platforms. ratatui names the view
`ToastView` because `Toast` is the runtime's data type.

### Data (8)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `table` | Y | Y | Y |
| `logs` | Y | Y | Y |
| `collapsible` | Y | Y | Y |
| `accordion` | Y | Y | Y |
| `tree` | **—** | Y | Y |
| `key-value` | **—** | Y | Y |
| `json-viewer` | **—** | Y | Y |
| `file-picker` | **!** | Y | Y |

### Effects (1)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `fx-layer` | Y | Y | Y |

### Charts (6)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `pie-chart` | Y | Y | Y |
| `line-chart` | Y | Y | Y |
| `bar-chart` | Y | Y | Y |
| `gantt-chart` | Y | Y | Y |
| `sparkline` | Y | Y | Y |
| `chart-legend` | (y) | Y | Y |

ratatui packs all six into `widgets/charts/`, which is exactly the packing #72 predicted
would fool a name match. Every one is there. `chart-legend` renders on SolidJS
(`charts.tsx` uses a local `Legend` and a `.fchart-legend` class) but is not exported —
a public-surface difference, not a missing control.

### Code (2)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `code-editor` | Y | Y | Y |
| `diff-editor` | Y | Y | Y |

Rust names them `CodeView`/`DiffView`. The SolidJS pair wraps CodeMirror 6; both Rust
pairs are hand-rolled. Same control, and the implementation pages will diverge hard.

### Graph (2)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `node-graph` | Y | **—** | Y |
| `flowchart` | Y | Y | Y |

### Chat (7)

| Control | SolidJS | ratatui | egui |
|---|:--:|:--:|:--:|
| `chat-view` | Y | Y | Y |
| `chat-message` | Y | Y | Y |
| `chat-tool-call` | Y | Y | Y |
| `chat-prompt` | Y | Y | Y |
| `chat-composer` | Y | Y | Y |
| `link-card` | Y | **—** | Y |
| `markdown` | Y | Y | Y |

The whole ratatui chat kit is one 
`specialty/chat.rs`, which is the second packing #72 warned about. `chat-message` and
`chat-tool-call` are `ChatItem::Message` and `ChatItem::ToolCall` there, and egui models
them the same way in `chat/view.rs`. Both are real renderings with real anatomy — a
tool call carries name, status and an open/closed detail body on all three.

---

## 3. The gap list

Nine cells, ready to become tickets.

### SolidJS (6)

| Control | Verdict | Note |
|---|---|---|
| `file-picker` | **Not possible** | Both Rust versions are `std::fs::read_dir` filesystem browsers with `Enter` to descend. A browser page cannot enumerate a filesystem. The web platform answers with `<input type="file">` and the File System Access API, which is a different control with a different contract — not this one ported. **Record as not possible, do not ticket.** |
| `tree` | Absent | Buildable. `packages/blocks` already has a `TreeView`, so a general one is a lift-and-generalise, not a from-scratch build — but `blocks` is still fog. |
| `key-value` | Absent | Buildable, small. |
| `json-viewer` | Absent | Buildable. Composes `tree`, so it should follow it. |
| `status-bar` | Absent | Buildable. Needs a slot on `AppShell` as well as the control; both Rust shells already have `.status()`/`.status_right()`. |
| `help-bar` | Absent, **and question the fill** | A persistent keyboard-hint strip is a TUI convention. It is buildable in a browser (a footer of `kbd` chips) but there is no web convention it serves. See §5. |

### ratatui (2)

| Control | Verdict | Note |
|---|---|---|
| `node-graph` | Absent | **Possible, and cheaper than it looks.** ratatui already draws `flowchart` (226 lines: nodes, elbow edges, auto-layout) and handles mouse throughout via `handle_mouse`. What `node-graph` adds over `flowchart` is drag-positioning and port-level connection. egui's is 816 lines. Caveat for the Contract: positions quantise to cells, so drag is coarse. |
| `link-card` | Absent | **Possible, and small.** SolidJS `LinkCard` renders a globe glyph, title, domain and description — **no thumbnail**, because metadata cannot be fetched client-side. egui's is 72 lines. There is nothing here a terminal cannot draw. |

### egui (1)

| Control | Verdict | Note |
|---|---|---|
| `help-bar` | Absent | ratatui has `structure/help_bar.rs`; egui's `structure/` does not. Same convention question as SolidJS, but weaker — egui apps are desktop apps, and a keybinding strip is at least idiomatic there. |

### Not possible — the full register

**One cell: `file-picker` on SolidJS.** Every other empty cell is merely unbuilt.

This is the audit's most load-bearing correction. #70 assumed a meaningful share of the
grid would be structurally impossible per platform. It is one cell in 219.

---

## 4. Reverse gaps — the six the SolidJS list could not see

The 79 came from SolidJS exports, so anything Rust-only was invisible. Six controls are
real, shipped on one or both Rust platforms, and have no SolidJS counterpart:

`status-bar` · `help-bar` · `tree` · `key-value` · `json-viewer` · `file-picker`

All six are in `packages/ui`'s families — structure and data — not in specialty territory.
They are ordinary controls that the browser side never needed and so never grew.

Consequence for the catalogue: **a control page is not a SolidJS page with two ports
attached.** Six of 73 pages describe a control SolidJS does not have, and one of those six
has a permanently empty SolidJS cell. #66's control-page template already handles this —
the `## Contract` is platform-agnostic — but the platform indexes must not read as though
SolidJS is the reference rendering.

---

## 5. A gap is not automatically a ticket

#70 settled that every empty cell is recorded and becomes a ticket in the handoff spec.
This audit finds two cells where that is the wrong reflex:

- **`file-picker` on SolidJS** — not possible. Recorded with the reason; never a ticket.
- **`help-bar` on SolidJS and egui** — possible, but the control encodes a TUI convention.
  Filling it means inventing a browser idiom Forge has no evidence for, and #70 settled
  that a gap is filled by **building the control in a real target app** — which needs a
  real app that wants it. Until one does, this is a **declined** cell, not a pending one.

That is a third status the register needs, beside *filled* and *empty*: **declined, with
the reason**. Without it, `gaps.md` shows two cells that will never close and reads as
permanently incomplete.

---

## 6. What the fogged packages would do to this

#72 excludes `blocks` (31), `grid` (5) and `kanban` (2) — still fog on the map. The answer
would change materially, and not in the direction the map assumes.

**Rust already has all three**, on both platforms:

| Fogged package | ratatui | egui |
|---|---|---|
| `packages/blocks` | `specialty/blocks/` (4 files, `BlockEditor` + `CustomBlock`) | `specialty/blocks/` (7 files) |
| `packages/grid` | `data/block_grid.rs` (`BlockGrid`, `BlockSpec`) | `data/block_grid.rs` |
| `packages/kanban` | `data/kanban.rs` (`Kanban`, `KanbanColumn`, `KanbanMove`) | `data/kanban.rs` |

So bringing them in scope would add **near-zero gaps**. But the map's "+38 controls" is
wrong in shape:

- **`blocks` is not 31 controls.** The 31 SolidJS exports (`TextBlockEdit`, `TableEdit`,
  `SlashMenu`, `PieChartView`, `TimelineView`, …) are the internals of **one** block
  editor. Both Rust platforms expose exactly one — `BlockEditor` — and keep the rest
  private. Cataloguing them as 31 peer controls would describe a SolidJS implementation
  detail as if it were the contract.
- **`grid` is 1 control** (`BlockGrid`), not 5. The other four exports are geometry
  functions (`collides`, `compactUp`, `resolvePush`, `clampRect`).
- **`kanban` is 1 control** (`KanbanBoard`), not 2.

Realistic figure if the fog lifts: **73 + ~3 to ~8**, not 73 + 38. The file count lands
nearer 320 than the map's ~477. Whoever resolves the `blocks`/`grid`/`kanban` fog should
start from this, not from the export counts.

---

## 7. What this does to the file count

At 73 controls over 3 platforms, with #66's tree (control pages + implementation pages +
per-platform indexes + `laws.md` + `anti-patterns.md` + `gaps.md` + `audit.md` +
`maintenance.md`):

| | Files |
|---|---|
| Control pages | 73 |
| Implementation pages (210 filled cells) | 210 |
| Platform indexes | 3 |
| `reference/` singletons | 5 |
| **Total** | **291** |

The map's figure is ~324. This is **10% smaller**, from a real count rather than an
estimate — and the shrink comes from folding anatomy and from the corrected package
counts, not from dropping coverage.

Empty cells cost nothing: an empty cell gets a line in `gaps.md`, not a page.

---

## 8. Out, and confirmed out

`term`/`remote`/`desktop` leave with `forge-widgets` per
[#63](https://github.com/wiltaylor/forge/issues/63). Confirmed on the Rust side:
`forge-tui/src/widgets/specialty/terminal.rs`, `forge-egui/src/widgets/term.rs`,
`forge-egui/src/widgets/desktop/`, `forge-egui/src/widgets/stream.rs`. ratatui has no VNC
or RDP rendering at all — that asymmetry travels with the extraction and is not this
catalogue's problem.

---

## 9. Evidence

Read directly: every `mod.rs` under `crates/forge-tui/src/widgets/` and
`crates/forge-egui/src/widgets/`, both `runtime/mod.rs`, every `index.tsx` under
`packages/*/src/`, and the export surface of every SolidJS component module. Specific
files opened to settle a call: `packages/ui/src/shell.tsx` (no status slot),
`packages/chat/src/linkcard.tsx` (no thumbnail), `packages/charts/src/charts.tsx` (private
`Legend`), `crates/forge-tui/src/widgets/data/file_picker.rs` (`std::fs::read_dir`),
`crates/forge-tui/src/widgets/specialty/chat.rs` (`ChatItem` variants),
`crates/forge-egui/src/widgets/specialty/chat/view.rs` (same variants),
`crates/forge-tui/src/widgets/overlays/menu.rs` (dropdown and context are one widget),
`crates/forge-egui/src/runtime/mod.rs` (`open_palette`).

LOC, for the per-platform weight #70 tracked: `forge-tui/src` **17,950**,
`forge-egui/src` **20,510**, `packages/ui/src` **3,082**.
