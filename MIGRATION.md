# MIGRATION.md — work order

Forge becomes one Claude Code skill. The code that the skill describes is deleted.
This file carries the four things no ticket holds: the merged order, the two gate
lists, the file inventory, and the gap-register seed.

**Reader:** the agent fleet working in this repo. Follow the order. Read a ticket only
when you want the reasoning.

**Map:** [Turn Forge into skills](https://github.com/wiltaylor/forge/issues/61). The map
holds every constraint. This file restates none of them.

**This file dies.** The last step of gate 2 deletes it. See §2, Gate 2 closing steps.

---

## 1. Sequencing

Two tracks run at the same time. The **catalogue track** writes ~325 pages in six
phases ([#68](https://github.com/wiltaylor/forge/issues/68)). The **backend track**
writes six pages in three phases
([#80](https://github.com/wiltaylor/forge/issues/80),
[#81](https://github.com/wiltaylor/forge/issues/81)). Their source is disjoint and
their spines are separate, so nothing serialises them. The backend track is five agents
against ~14 family slices, so it lands first by a wide margin.

Both tracks start after step 1.

### Step 1 — the #63 file moves

Move the streaming-widget vertical to a new `forge-widgets` repo. Move the IdP to a new
`forge-auth` repo. See [#63](https://github.com/wiltaylor/forge/issues/63) for the
precise contents of each, the `WidgetStream` seam, and the two rewrites.

**Only the file moves block authoring.** #63's build-and-verify blocks *deletion*, and
it is gate-1 condition 1. Phase C0 must sweep the final tree, so the moves must land
first.

### Catalogue track

#### C0 — Promote the proto tree, then freeze the roster

**The proto tree is not on `main`.** It lives on a linear chain of branches, all on
`origin`:

```
prototype/skill-routing → prototype/page-templates → prototype/guidance-probe → prototype/description-firing
```

The tip holds 479 files under `.claude/skills/forge-design-proto/`:

| Path | Files | What |
|---|---|---|
| `reference/controls/` | 76 | control pages, mostly stubs |
| `reference/impl/` | 231 | implementation pages, mostly stubs |
| `reference/api/` | 2 | stubs; #75 measured `index.md` as *"Not written"* |
| `reference/` root | 7 | `laws.md`, `tokens.md`, `anti-patterns.md`, `gaps.md`, `solidjs.md`, `ratatui.md`, `egui.md` |
| `probes-67/`, `probes-75/` | 157 | probe corpora — working material, never shipped |
| root | 6 | `SKILL.md`, `README.md`, `TEMPLATES.md`, `FINDINGS.md`, `PROBE-67.md`, `PROBE-75.md` |

Twenty-two of those `reference/` pages are the frozen `button` / `combobox` / `table`
calibration set. Do not edit them.

1. Merge the chain onto `main` as `.claude/skills/forge-design-proto/`. The directory
   has no `SKILL.md` at the top level of a live skill path, so it is inert. `main` can
   accumulate the tree with no risk to the live `forge-design`.
2. Sweep the source for **non-exported shared chrome**
   ([#79](https://github.com/wiltaylor/forge/issues/79)). #72 counted exports, so it
   could not see `field` or the chart chrome. Add what you find to the roster.
3. Sweep the source for **cross-control mechanisms** that earn a spine line (#68).
4. Freeze the roster. The 81 controls and ~325 files are a **floor**, not a measurement
   (#79). Record the new total.

Finish the sweep before C1. `solidjs.md`'s owner column needs the frozen roster, and a
composition link needs its target to be in the roster.

#### C1 — Spine

One agent writes `laws.md`, `anti-patterns.md`, `tokens.md` with values, and the three
grammar pages `solidjs.md` / `ratatui.md` / `egui.md`. The spine then goes
**read-only to families**. A family that writes to the spine mints a second name for a
role that nobody else knows.

#### C2 — Family slices

About 14 slices. A slice is a **family of 5–6 controls drawn by source locality**. One
agent writes every page in the family, across all three platforms. Never slice per
platform, and never slice per control (#68).

Per slice:

1. Take a worktree.
2. Read the 22 calibration pages first. They set how deep a Contract goes.
3. Run `apps/gallery`, `examples/tui-gallery` and `examples/egui-gallery`, and **look**
   at each control. Galleries are a required tool. Answer every visual checklist item
   from what renders, not from source.
4. Read each source file **whole**, against the fixed behaviour checklist: what Escape
   does, what focus loss does, what empty and overflow do, what the control returns or
   bubbles, coarse-pointer behaviour, and what happens when the theme lacks a colour.
   **Silence on a checklist item is a defect, not a permitted omission.**
5. Need a word the spine lacks? A filled cell missing a name is a **defect — report,
   never mint**. Stop and escalate. Only an empty cell licenses minting, marked
   `[minted]` inline.
6. Pass a **fresh-reader review** — an agent that did not write the family, checking
   against the live source.
7. Pass the **per-control defect pass** as each control's four pages land: conform ·
   amend · release · reclassify
   ([#74](https://github.com/wiltaylor/forge/issues/74)).
8. Merge the worktree into `.claude/skills/forge-design-proto/` on `main`.

Cap concurrency at about 3–4. Gallery build cost is the limit, not git. Share one
`CARGO_TARGET_DIR`. Collisions are confined to `gaps.md` and the three platform
indexes, which are append-only.

**Human ratification is rolling for Contract moves.** A Contract move changes binding
text and needs the live source to adjudicate. Clear it before the family merges.
Minted lines do not block; they queue to C5.

#### C3 — Probe sample

About 42 sessions. Draw **one control per family, all three platforms**, at random
*after* the family lands. Fix the asks before you see any result.

- A probe never sees a gallery.
- A probe never knows it is a test. Sanitise the skill.
- Each probe records the skill path it loaded. A session that loaded anything but this
  repo's project-scope copy is **void**. Re-run it. Do not count it.

Fix every entry defect the probes find.

#### C4 — Grounding check

Grep every token name, class and prop that the catalogue enumerates against the real
source, while the source still exists. Scope is **all pages**, not the five name pages
([#73](https://github.com/wiltaylor/forge/issues/73)). The check is mechanical and
name-class-scoped, from code spans and tables only, never prose.

**It is a transcription check, not a design review. The source wins, warts included. It
may not rename.** A name that verifies nowhere is fixed or cut.

`crates/forge-blocks` (4,415 LOC) is in scope. Its literal fixtures in `tests/schema.rs`
are the only thing that can ever prove a transcription of the JSON interchange schema
correct.

#### C5 — Batched human pass over `[minted]`

The human ratifies or cuts every `[minted]` line, then strips the marker. This is
**ratification, not proof**. It is also the one place where systemic Contract over-reach
becomes visible.

### Backend track

Runs concurrently with C0–C5.

#### B1 — Spine

One agent writes `reference/api/contract.md` and `reference/api/index.md`. Recast
`docs/api-contract.md`; do not move it verbatim.

- **Transcribes intact:** the envelope, auth and claims including the `?token=` leak
  caveat, auth-disabled mode, all 13 endpoint rows, doc-store semantics, event fan-out,
  the 9 environment variables.
- **Recast:** the three-implementation conformance claim becomes *conformance is what
  you build against*; the `examples/parity` pointer is rewritten as a named pointer to
  `conformance/`; `forge-hash` becomes argon2id PHC verification stated as a
  requirement, not a tool.
- **Add** the normative **client-interface section** defining the `ForgeClient` surface:
  `auth`, `data`, `actions`, `events`, `ws`, `wsUrl`, `onUnauthorized`, `health`,
  `request` (#81).
- **Add** ~3 lines stating that the wire is the one exception to accepted drift.
  Deviation there is a bug, not discretion (#80). Ship no checker.

Verify against both servers. `contract.md` is then read-only **by construction** — it is
normative, so a rendering page cannot write to it.

#### B2 — Four renderings, in parallel

One worktree each. **No agent reads source that another agent read.**

| Page | Source | LOC |
|---|---|---|
| `axum.md` | `crates/forge-core` + non-widget `crates/forge-server` | ~2,200 |
| `fastapi.md` | `python/forge-server` | 1,045 |
| `client.md` | `packages/client` | 614 |
| `tauri.md` | `crates/forge-tauri` + `packages/tauri` | ~1,500 |

`tauri.md` covers **both ends on one page**. Its substance is today's
`.claude/skills/forge-tauri/reference/ipc-patterns.md`. Carry the Forge facts — the ACL
triad, `links = "tauri-plugin-forge"` ↔ `Builder::new("forge")`, the one-command
`plugin:forge|request` bridge, `emit("forge://event")`, the promise-queue send ordering.
**Cut every Tauri fact** — identifier rules, the webkit preflight, `tauri icon`, the Vite
port, `beforeDevCommand`, `frontendDist`. The toolkit wins.

**`tauri.md` is a pull-forward. It must be complete before gate 1 closes** (gate-1
condition 10). `reference/api/index.md` landing at gate 1 as a stub is accepted; no
skill dies underneath it.

#### B3 — Cross-rendering defect pass

Four renderings, not three. The divergences that matter are axum against FastAPI against
client against Tauri.

### Step 2 — Gate 1

Check the ten conditions in §2. Then make **one commit**:

1. **Promote** `.claude/skills/forge-design-proto/` into `.claude/skills/forge-design/`.
   Replace `SKILL.md` with the settled 811-char description
   ([#71](https://github.com/wiltaylor/forge/issues/71),
   [#75](https://github.com/wiltaylor/forge/issues/75), trimmed by #81). Drop
   `assets/`, `preview/`, the old reference pages, `probes-67/`, `probes-75/`,
   `TEMPLATES.md`, `FINDINGS.md`, `PROBE-67.md` and `PROBE-75.md`.
2. **Promote** `reference/api/tauri.md`.
3. **Delete** `.claude/skills/forge-tauri/`, both its reference pages with it.
4. **Delete** every path that §3 marks *gate 1*.
5. **Keep** every path that §3 marks *gate 2*.

Deletion is one commit. Staged-per-platform is incompatible with family slicing, with
the per-control defect pass, and with the one-shot grounding check. Recoverability is
free; `git show` reaches everything.

**The gates gate deletions, not authoring** (#81).

### Step 3 — Gate 2

Check the eight conditions in §2. Then make the gate-2 commit, and run its closing steps
in order. The last step deletes this file.

---

## 2. The gates

Twenty-one conditions. #63 splits across both gates: its file moves gate **authoring**
(step 1); its build-and-verify gates **deletion** (gate-1 condition 1).

### Gate 1 — the catalogue gate. Ten conditions.

| # | Condition | Source |
|---|---|---|
| 1 | **#63 extraction verified.** `forge-widgets` and `forge-auth` build in their new repos with their demos running, and the ported ratatui terminal is checked against the original. | #63, #68 |
| 2 | **Roster complete.** Every cell has a page, or a `reference/gaps.md` line with a status (filled / not possible / declined) and a reason. | #68 |
| 3 | **Every family passed** its fresh-reader review and its per-control defect pass. | #68 |
| 4 | **Zero open Contract defects.** | #74 |
| 5 | **Zero `[minted]` markers.** | #78 |
| 6 | **Grounding check clean** over all pages. | #69, #73 |
| 7 | **Probe sample passed.** Every entry defect fixed. No void session counted. | #68 |
| 8 | **All three galleries build and run.** | #68 |
| 9 | **No `forge-design` outside this repo's project scope.** Precedence is enterprise > personal > project, so a stale copy shadows silently and voids a probe. | #68 |
| 10 | **`reference/api/tauri.md` is complete.** Tauri-scoped only, not the rest of `reference/api/`. | #81 |

Nothing exists at personal, plugin or enterprise scope today. Condition 9 is a check,
not a removal step. Keep it on the checklist; the hazard is latent.

Condition 8 is maintenance on code that dies in the same commit. Pay it. #67's contrast
defect and its `active`-painting defect are invisible in source.

### Gate 2 — the backend gate. Eight conditions.

| # | Condition | Source |
|---|---|---|
| 1 | **Roster complete.** Six pages, and every endpoint × rendering hole recorded in `reference/gaps.md`. | #80, #81 |
| 2 | **Fresh-reader review passed**, and the **cross-rendering** defect pass passed. At n=1 there is nothing to iterate over, so the per-control pass becomes cross-rendering. | #80 |
| 3 | **Zero open Contract defects.** | #74, #80 |
| 4 | **Zero `[minted]` markers.** | #78, #80 |
| 5 | **Grounding check clean.** Unusually mechanical here: env var names, endpoint paths, both regexes, every JSON key. | #80 |
| 6 | **Probe sample passed.** This includes #81's two description probes over 6 sessions: *"Build me a Tauri desktop app with a Forge UI"* in an empty directory must fire; *"Add a Tauri command to my app"* in a plain Tauri repo must not fire. | #80, #81 |
| 7 | **Parity green**, four ways: against a server a probe built from `axum.md` alone; against a server a probe built from `fastapi.md` alone; against `crates/forge-server` and `python/forge-server` immediately before deletion; and the **import swap** for `tauri.md`. | #80, #81 |
| 8 | **No `forge-design` outside this repo's project scope.** | #80 |

#68's condition 1 **drops** here. The `widgets/` subtrees left at step 1, and #63 was
verified at gate 1.

Condition 7 is the only check in this migration that returns an exit code. Gate 1 asserts
that the ground truth is intact. Gate 2 asserts that the guidance **reproduces** it.

**The import swap** (#81): the corpus is black-box HTTP and a Tauri app has no HTTP
server, so the corpus cannot reach `tauri.md`. Instead, build a working Forge app from
`client.md` plus the control pages. Change only the `createClient` import. Every screen
must keep working. Stated limit: it exercises only the rows that the app touches. No
gate-2 condition otherwise produces a client-side app, so the probe builds its own first.

### Gate 2 commit and closing steps

1. Delete every path that §3 marks *gate 2*.
2. Delete `docs/api-contract.md`. One normative copy now lives inside the skill.
3. **File one issue per buildable gap** from `reference/gaps.md`. That is 8 of the 17
   seed lines, plus whatever C0's sweep added.
4. **Delete `MIGRATION.md`.** This is the final gate-2 condition (#82). `HANDOFF.md`
   rotted because nothing ever scheduled its death. This file has one.

---

## 3. Repo file inventory

Every path gets a verdict and the gate that takes it. Reconciled across #63, #68, #80
and #81.

**Verdicts:** *#63* leaves at step 1 · *gate 1* · *gate 2* · *stays*.

### `packages/`

| Path | Verdict | Why |
|---|---|---|
| `packages/ui` | gate 1 | catalogue source, 56 controls |
| `packages/chat` | gate 1 | catalogue source, 7 controls |
| `packages/charts` | gate 1 | catalogue source, 6 controls |
| `packages/code` | gate 1 | catalogue source, 2 controls |
| `packages/graph` | gate 1 | catalogue source, 2 controls |
| `packages/blocks` | gate 1 | catalogue source, `block-editor` plus 4 block kinds (#76) |
| `packages/grid` | gate 1 | catalogue source, `dashboard-grid`, SolidJS-only (#76) |
| `packages/kanban` | gate 1 | catalogue source, `kanban` (#76) |
| `packages/tokens` | gate 1 | source for `tokens.md`, written in C1 |
| `packages/remote` | gate 1 | **see correction 2 below** |
| `packages/term` | #63 | streaming widgets |
| `packages/desktop` | #63 | streaming widgets |
| `packages/client` | **gate 2** | the only source for `client.md`. 614 LOC, zero runtime dependencies, no control names |
| `packages/tauri` | **gate 2** | the client half of `tauri.md`. `src/widgets.ts` (110 LOC) leaves at step 1 |

### `crates/`

| Path | Verdict | Why |
|---|---|---|
| `crates/forge-tui` | gate 1 | catalogue source, ratatui. `widgets/specialty/terminal.rs` (555 LOC) leaves at step 1 |
| `crates/forge-egui` | gate 1 | catalogue source, egui. `widgets/term.rs`, `widgets/desktop/`, `widgets/stream.rs` leave at step 1 |
| `crates/forge-blocks` | gate 1 | the block document model, consumed by `forge-egui` across 7 files and mirrored in `forge-tui`. Catalogue-side (#76) |
| `crates/forge-core` | **gate 2** | source for `axum.md`. `src/widgets/` (2,810 LOC) leaves at step 1 |
| `crates/forge-server` | **gate 2** | source for `axum.md`. `src/widgets/` and the `with_*` wiring in `app.rs` leave at step 1 |
| `crates/forge-tauri` | **gate 2** | the Rust half of `tauri.md`. `widget_stream.rs` and the widget halves of `bridge.rs` / `commands.rs` / `lib.rs` / `state.rs` leave at step 1 |
| `crates/forge-auth` | #63 | its own repo, with migrations, Dockerfile and console. Its two `ForgeApp` call sites are cut during extraction |

### `python/`

| Path | Verdict | Why |
|---|---|---|
| `python/forge-server` | **gate 2** | the only source for `fastapi.md`, 1,045 LOC |

### `apps/`

| Path | Verdict | Why |
|---|---|---|
| `apps/gallery` | gate 1 | imports `@forge/ui` and friends. Ground truth until the gate, then dies with it |
| `apps/auth` | #63 | the IdP console, travels with `forge-auth` |
| `apps/remote-widgets` | gate 1 | **see correction 1 below** |

### `examples/`

| Path | Verdict | Why |
|---|---|---|
| `examples/tui-gallery` | gate 1 | consumes `forge-tui` |
| `examples/egui-gallery` | gate 1 | consumes `forge-egui` |
| `examples/egui-demo` | gate 1 | consumes `forge-egui` |
| `examples/tauri-demo` | gate 1 | consumes the Tauri pair and `@forge/ui` |
| `examples/rust-demo` | gate 1 | serves `apps/gallery/dist`, and takes `forge-server` with `features = ["widgets"]`. **Corrects #68**, which put it at gate 2 |
| `examples/python-demo` | gate 1 | serves `apps/gallery/dist`. **Corrects #68**, which put it at gate 2 |
| `examples/auth-demo` | #63 | travels with `forge-auth` |
| `examples/widgets-testenv` | #63 | docker-compose plus `rdp-probe.mjs`, travels with `forge-widgets` |
| `examples/parity` | **stays**, renamed | depends on nothing in this repo — `httpx`, `pytest`, `websockets`, `FORGE_TEST_BASE_URL`. Move it to `conformance/`. Keep its `just parity-test` recipe. Replace its *"both example apps do this"* note with explicit preconditions in a README |

Stripping `rust-demo` and `python-demo` was genuinely available — they serve a **path**,
which is repointable, where an import is not. It is **declined**. A demo backend in this
repo is a reference implementation, which is the exact thing that guidance-only exists to
delete.

### `docs/`

| Path | Verdict | Why |
|---|---|---|
| `docs/api-contract.md` | **gate 2** | recast into `reference/api/contract.md` in B1, deleted in the gate-2 commit. `docs/` sits outside the skill, so a link to it dangles |
| `docs/widgets-protocol.md` | #63 | 210 lines, still normative. It scopes itself to endpoints that all leave at step 1. Leave no pointer stub — it would sit in a skill that must never mention widgets |

### `.claude/skills/`

| Path | Verdict | Why |
|---|---|---|
| `forge-design` | gate 1, replaced | rewritten in place from the proto tree. `assets/` and `preview/` go with it; all 15 gallery sections import through the `@forge/*` alias |
| `forge-design-proto` | gate 1, promoted | created in C0, promoted into `forge-design` in the gate-1 commit, and gone as a separate directory |
| `forge-tauri` | gate 1 | deleted in the gate-1 commit, both reference pages with it. `reference/scaffold.md` dies outright. `reference/ipc-patterns.md` becomes the substance of `tauri.md` |
| `playwright-cli` | **stays** | unrelated to this migration |
| `playpen` | **already deleted** | 28 files, removed by `85ed9c8` ahead of the gate, recoverable from git history |

### Root and support files

| Path | Verdict | Why |
|---|---|---|
| `MIGRATION.md` | gate 2, last step | this file |
| `HANDOFF.md` | **gate 1** | 80 lines, dated 2026-07-11, still says *"Nothing is committed yet."* It describes `crates/forge-egui` milestones for a crate that dies at gate 1. It is already rotted. Delete it |
| `README.md` | **rewrite at gate 2** | it describes the packages, the crates and the backends as products. After gate 2 this repo holds one skill and a conformance corpus. Rewrite it in the gate-2 commit to say that. Do not carry the package tables |
| `Cargo.toml` | edited at both gates | drop each workspace member as its crate goes |
| `package.json`, `pnpm-workspace.yaml`, `turbo.json`, `tsconfig.base.json` | edited at both gates | drop each workspace entry as its package goes |
| `justfile` | edited at both gates | keep the `parity-test` recipe; it moves with `conformance/` |
| `scripts/prepare-package.mjs` | **gate 2** | npm packaging helper, used by `packages/client` and `packages/remote`. It outlives gate 1 |
| `scripts/oidc_flow_test.py` | #63 | travels with `forge-auth` |
| `vendor/ironrdp-session` | #63 | the RDP patch, travels with `forge-widgets` |

### Two corrections, and what forced them

**Correction 1 — `apps/remote-widgets` dies at gate 1.** #68's table puts it in *"leaves
earlier with #63"*. #63 says the opposite: it is micro-frontend remote elements
(`@forge/remote` — `defineRemoteElement`), unrelated to streaming widgets, and its own
words are *"It stays. Do not delete it by grep."* Both readings are wrong for this
inventory. #63's line is a **naming trap warning**, scoped to the widget extraction — it
means *do not take this to `forge-widgets`*, not *this survives*. Dependency tracing
settles it: `apps/remote-widgets` imports `@forge/ui` and `@forge/charts`, and both die
at gate 1, so nothing can build it afterwards.

**Correction 2 — `packages/remote` dies at gate 1, and does not travel with #63.** The
map's Size note says *"`term`/`remote`/`desktop` leave with `forge-widgets`"*. #63's own
extraction table lists only `term`, `desktop` and `tauri/src/widgets.ts` — not `remote` —
and its naming trap says why. `packages/remote` depends on `@forge/ui`, so it cannot
outlive gate 1 in any case. No gate-2 page sources from it either: #80 records the two
`/api/components` rows as **declined** for the client, because federation is fetched
elsewhere, so `client.md` never describes the mount side.

---

## 4. Gap register seed

**Seventeen lines. Eight are buildable work.** C1 writes `reference/gaps.md` from this
seed and **spends it**. This section is input, and transient. The single register is the
only place a status is asserted; platform indexes are link lists with no status column.

Source today: `AUDIT.md` on the `audit/control-grid` branch, plus the prose of #76 and
#81.

### Buildable — 8 lines. Each becomes an issue at gate 2, closing step 3.

| Control | Platform | Source |
|---|---|---|
| `tree` | SolidJS | #72. `TreeView` is a static `└─ ├─` renderer — a different control sharing a word |
| `key-value` | SolidJS | #72 |
| `json-viewer` | SolidJS | #72 |
| `status-bar` | SolidJS | #72 |
| `node-graph` | ratatui | #72. Cheap; ratatui already draws `flowchart` |
| `link-card` | ratatui | #72. Cheap; `LinkCard` has no thumbnail to render |
| `dashboard-grid` | ratatui | #76 |
| `dashboard-grid` | egui | #76 |

Fill a gap by **building the control in a real target app from the guidance**, then
writing the page from that working code. Build nothing to throw away.

### Not possible — 1 line

| Cell | Reason |
|---|---|
| `file-picker` on SolidJS | a browser page cannot enumerate a filesystem |

### Declined — 3 lines

A declined cell is possible, but it encodes another platform's convention.

| Cell | Reason |
|---|---|
| `help-bar` on SolidJS | a terminal convention |
| `help-bar` on egui | a terminal convention |
| `flow-grid` on SolidJS | the browser supplies `grid-column: span N` |

### Backend — 5 lines, zero work (#81)

All five are Tauri rows. A hole here is a scope statement, never a ticket waiting.

| Row | Status | Reason |
|---|---|---|
| `/api/events` (SSE) | **not a gap — Mechanism** | Tauri carries events by `emit('forge://event')` and one shared `listen()`, filtered client-side. Same behaviour, different route |
| `/api/ws` | declined | a second transport for events that IPC does not need. `ws.connect()` throws with that sentence today |
| `/api/components` | declined | federation fetches bundles at runtime; a Tauri frontend is compiled into the binary |
| `/api/components/{file}` | declined | same |
| `/*` static | not possible | the webview loads from `tauri://localhost`. No HTTP server in the app is the plugin's whole purpose |

A Tauri app is **permanently auth-disabled by construction** — `auth/login` answers 404
always, and `auth/me` answers `Claims::anonymous()`. That is not a gap. The contract has
an auth-disabled mode and Tauri never leaves it, because the OS user is the authenticated
party. State it as a Contract line on the page.

### The register grows — treat 17 as a floor

Two additions are already known, and neither is in the 17.

1. **`packages/client`'s own holes** (#80). The client covers 9 of the 13 endpoint rows.
   `/api/components` and `/api/components/{file}` are **declined**, because federation is
   fetched elsewhere. Static serving is **not possible**, because it is not a client
   concern. #83 counted only #81's five backend lines, so these were never added to the
   seed total. Record them in B2.
2. **`field` on ratatui** is **declined** (#79). A terminal has no room for a help line.
   `field` is shared chrome that #72 could not see, because #72 counted exports.

C0's sweep adds more of kind 2. Neither addition is buildable work, so the count of 8
issues at gate 2 holds unless the sweep finds a buildable cell.
