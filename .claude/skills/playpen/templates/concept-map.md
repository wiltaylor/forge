# Concept Map Template

Use this template when the playpen is about learning, exploration, or mapping
relationships: concept maps, knowledge-gap identification, scope mapping, task
decomposition with dependencies.

Canvas-based playpens differ from the two-panel split: the interactive visual IS the
control — users drag nodes and draw connections rather than adjusting sliders. Build on
the **pointer-drag + SVG edge pattern** in `reference/solid-patterns.md` (nodes as
absolutely-positioned `.fcard` divs inside `.pp-canvas`, edges as SVG paths that follow
drags reactively).

## Layout

Canvas on top (preview area, full width), sidebar below-left (knowledge levels,
connection type selector, node list, actions), prompt output bottom-right. Override the
grid:

```css
.pp-shell.pp-map { grid-template-columns: 1fr 380px; grid-template-areas:
  "canvas canvas" "sidebar prompt"; grid-template-rows: 1fr auto; }
```

## Control types

| Decision | Control | Example |
|---|---|---|
| Knowledge level per node | Click-to-cycle `Badge` in the node list | know → fuzzy → unknown |
| Connection type | `<select>` armed before drawing | calls, depends on, contains |
| Node arrangement | Drag on canvas | spatial layout reflects mental model |
| Which nodes to include | Checkbox per node | hide/show concepts |
| Actions | `Button`s | auto-layout, clear edges, reset |

Knowledge levels as tone triples: know = `success`, fuzzy = `warning`, unknown =
`danger` — badge on the node card (`<Badge tone={levelTone[n.level]} dot>`), and the
node card border tints to match (`border-color: var(--warning)` etc.).

## Canvas interactions

- **Drag**: the solid-patterns pointer capture recipe; persist positions on pointer-up
  (`saveDocDebounced('nodes', unwrap(nodes))`).
- **Edge drawing**: click node A then node B with a connection type armed; append to
  the `connections` store; edges re-derive from node positions automatically.
- **Tooltips**: absolutely-positioned div (`--bg-4`, `--border-strong`, `--r-sm`) on
  hover, showing the node description.
- **Force auto-layout**: simple spring simulation (repulsion between all pairs,
  attraction along edges, 100–200 damped iterations) run in plain JS, then
  `setNodes(reconcile(result))`.

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: nodes` | id → {x, y, label, description, level} — seed via `playpen data set nodes --file …` |
| `data: connections` | [{from, to, type}] — **the user's drawn understanding; read it back** |
| `action: layout` | Optional — compute auto-layout server-side for big graphs |

Pre-populate for a codebase/domain: generate the node list yourself (labels,
descriptions, file paths, starting positions) and seed `nodes` before handing over.

## Prompt output

A targeted learning request shaped by the knowledge markings:

> "I'm learning [codebase/domain]. I already understand: [know nodes]. I'm fuzzy on:
> [fuzzy nodes]. I have no idea about: [unknown nodes]. Here are the relationships I
> want to understand: [edge list]. Please explain the fuzzy and unknown concepts,
> focusing on these relationships."

Only include edges the user drew; only mention concepts marked fuzzy or unknown.

## Example topics

- Codebase architecture map (modules, data flow, state management)
- Framework learning (how React hooks connect, Next.js data fetching layers)
- System design (services, databases, queues, caches and how they relate)
- Task decomposition (goals → sub-tasks with dependency arrows, knowledge tags)
- API surface map (endpoints grouped by resource, shared middleware, auth layers)
