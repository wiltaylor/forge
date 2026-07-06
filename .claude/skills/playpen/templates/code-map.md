# Code Map Template

Use this template when the playpen visualizes codebase architecture: component
relationships, data flow, layer diagrams, system architecture with interactive
commenting.

Canvas-based — build on the **pointer-drag + SVG edge pattern** in
`reference/solid-patterns.md`. Unlike concept-map, nodes here are usually fixed layouts
the agent generates; the user's main interactions are filtering layers and commenting
on components.

## Layout

Standard `pp-shell`: controls + comment list (left), SVG canvas with legend (right,
`.pp-canvas`), prompt output (bottom).

## Control types

| Decision | Control | Example |
|---|---|---|
| System view | Preset `Button`s | Full system, Chat flow, Data flow |
| Visible layers | Checkboxes per layer | Client, Server, SDK, Data, External |
| Connection types | Checkboxes with a coloured `StatusDot` | data flow, tool calls |
| Component feedback | Click node → comment box | textarea + Save in a floating `Card` |
| Zoom | `Button` +/−/reset | scale via a `transform` signal on the canvas group |

## Node & connection rendering

Nodes: `.fcard` divs (label + `--fs-xs` mono subtitle) positioned from the `nodes`
store; layer distinguishes by a tinted top border. Filtering = `<Show>`/`filter` on the
visible layers; edges only render when both ends are visible.

Layer accents — Forge tone tints, not raw hex:

| Layer | Border/tint |
|---|---|
| Client/UI | `var(--info)` / `var(--info-bg)` |
| Server/API | `var(--warning)` / `var(--warning-bg)` |
| SDK/Core | `var(--accent)` / `var(--accent-bg)` |
| Data | `var(--success)` / `var(--success-bg)` |
| External | `var(--fg-3)` / `var(--bg-2)` |

Connection types (SVG `stroke` + `stroke-dasharray`):

| Type | Stroke | Style |
|---|---|---|
| `data-flow` | `var(--info)` | solid |
| `tool-call` | `var(--success)` | dashed `6 3` |
| `event` | `var(--danger)` | dashed `4 4` |
| `dependency` | `var(--border-strong)` | dotted `2 3` |

Legend: a small `Card` overlaid bottom-left of the canvas listing type → `StatusDot` +
label.

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: nodes` | Components: id, label, subtitle (file path), x/y, layer |
| `data: connections` | [{from, to, type, label}] |
| `data: comments` | componentId → comment — **read back with `playpen data get comments`** |
| `data: state` | View state (visible layers, zoom, active preset) |
| `action: scan` | Optional — walk the repo server-side to regenerate nodes |

Pre-populate: the agent explores the codebase itself, writes the architecture as
`nodes`/`connections` JSON, and seeds both documents. That beats a server-side scan for
quality — reserve the `scan` action for refresh-heavy cases.

## Prompt output

Combine system context with the user's comments:

> "This is the [project] architecture, focusing on [visible layers].
> Feedback on specific components:
> **API Client** (src/api/client.ts): add retry logic with exponential backoff here.
> **Database Manager** (src/db/manager.ts): can we add connection pooling?"

## Example topics

- Codebase architecture explorer (modules, imports, data flow)
- Microservices map (services, queues, databases, API gateways)
- React component tree (components, hooks, context, state)
- Plugin/extension architecture (core, plugins, hooks, events)
