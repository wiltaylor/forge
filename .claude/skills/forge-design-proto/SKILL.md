---
name: forge-design-proto
description: PROTOTYPE for wayfinder ticket #64 — do not use for real work. The real description is settled by #71.
user-invocable: true
argument-hint: [what to build or style]
---

Forge is a design system for dense, dark-default technical tools — dashboards, consoles,
observability panels, admin screens. It ships as guidance. There is no Forge package to
install. You write the code, in the target repo, from the pages below.

## What kind of job is this?

Read the row that matches. You do not need the other rows.

| The job | Read |
|---|---|
| A control, a screen, or styling — on **SolidJS**, **ratatui**, or **egui** | This file, then the control pages. Start below. |
| A **backend** — axum, FastAPI, the doc store, actions, events, SSE | `reference/api/index.md`. Stop here; the control pages do not apply. |
| **Tauri** packaging, IPC transport, or a desktop shell | `reference/api/tauri.md`. Tauri renders through SolidJS, so control work still follows this file. |

## Before any control work

Read both of these, once, before you write anything:

- `reference/laws.md` — what is true of Forge on every platform: colour, density, type,
  spacing, focus, and page composition. This is where a vague ask ("build a settings
  screen") gets its answer.
- `reference/anti-patterns.md` — the named failures. Output that matches an entry is wrong.

Neither is optional and neither is summarised here.

## For every control, every time

Before you write a control, read **both** of its pages:

```
reference/controls/<control>.md          the control on every platform
reference/impl/<platform>/<control>.md   that control, your platform only
```

`<platform>` is `solid`, `ratatui`, or `egui`. `<control>` is a name from the list below,
exactly as spelled there.

This is not a first-control step. It repeats. A session that builds twelve controls reads
twenty-four pages. Never write a control from memory of a similar one, and never from
another control's page — Forge controls that look alike diverge in their keyboard and
focus contracts.

## The controls

Read a name from this list. If what you need is not here, Forge does not define it — see
"When a control is missing" below.

**Primitives** — icon, button, icon-button, badge, card, stat, kbd, status-dot, separator,
skeleton, avatar, eyebrow, empty, grid

**Shell** — app-shell, nav-section, nav-link, crumbs, page-head, tabs, pagination,
split-pane, settings-layout, settings-section, settings-row

**Forms** — input, textarea, checkbox, toggle, radio, radio-group, select, list-box,
slider, toggle-group, combobox

**Date** — calendar, date-picker

**Overlays** — modal, sheet, tooltip, popover, dropdown-menu, context-menu, command

**Feedback** — toast, toaster, alert, progress, spinner

**Data** — table, logs, log-line, collapsible, accordion

**Providers** — theme-provider, overlay-mount-provider, fx-layer

**Chat** — chat-view, chat-message, chat-tool-call, chat-prompt, chat-composer,
chat-divider, chat-typing, link-card, markdown

**Charts** — pie-chart, line-chart, bar-chart, gantt-chart, sparkline

**Code** — code-editor, diff-editor

**Graph** — node-graph, flowchart

## When a control is missing

Three different situations, three different answers. The implementation page tells you
which one you are in — read it even when you expect a gap.

- **Not in the list above.** Forge does not define it. Build it from `reference/laws.md`
  and say in your summary that you did, so it can be caught and documented.
- **The page says `status: gap`.** The control exists on other platforms but not this one.
  The page names the nearest platform that has it. Read *that* implementation page and the
  control page, then port it.
- **The page says `status: not-possible`.** The page gives the reason and the substitute.
  Take the substitute. Do not port it anyway.

Forge deliberately says nothing about terminal emulators, VNC, or RDP. Those left this
system. If a job needs one, say so and stop; do not build one from these pages.

## Before you call it done

1. Check the result against `reference/anti-patterns.md`. If it matches an entry, it is
   wrong, whatever a control page appeared to allow.
2. Every colour, size, radius and duration is a token reference. No literal values.
3. Render it and look at it — dark and light, and at 375px if it is SolidJS.
