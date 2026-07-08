# Node Grid Template

Use this template when the playpen is a node-based editor: visual programming, data
flow graphs, pipeline builders, state machines — any system where users connect typed
ports between nodes.

The heaviest canvas template. Build on the **pointer-drag + SVG edge pattern** in
`reference/solid-patterns.md`, extended with ports, a temporary drag-connection, and a
viewport transform.

> **Prefer the Forge `NodeGraph` component** (`forge-design/assets/graph.jsx` — elbow
> edges, typed ports, animated/broken edge states, drag + connect built in) over
> hand-rolling this from scratch; this template remains useful for its palette/
> properties/persistence structure and for viewport panning, which NodeGraph doesn't
> do yet.

## Layout

Node palette + properties + actions (left), zoomable/pannable canvas (right,
`.pp-canvas`), prompt output (bottom) — the standard `pp-shell` works.

## Core concepts

**Nodes** — `.fcard` divs positioned from the store: a title bar (`.fcard-head` with
the type label and a ghost delete button), input ports down the left edge, output
ports down the right, and property fields (`.ffield-input`) in the body. Drag by the
title bar only (so property inputs stay clickable).

**Ports** — 10px circles (`border-radius: var(--r-pill)` — the sanctioned pill use)
absolutely positioned on the node edges, coloured by type. Input ports accept one
connection; outputs allow many; only compatible types connect (`any` matches
everything).

**Connections** — SVG bezier curves between port positions (derived from node position
+ port index, so they follow drags reactively). Click to select
(`stroke: var(--accent)`), Delete/Backspace removes.

## State

```jsx
const [graph, setGraph] = createStore({
  nodes: {},        // id → {type, x, y, properties}
  connections: [],  // {id, fromNode, fromPort, toNode, toPort}
});
const [sel, setSel] = createSignal(null);              // {kind: 'node'|'conn', id}
const [viewport, setViewport] = createStore({ x: 0, y: 0, zoom: 1 });
```

Node type definitions (static in the app — or seed `data: node-types` if the user
should be able to vary them):

```jsx
const nodeTypes = {
  'http-request': {
    label: 'HTTP request', category: 'input',
    inputs: [{ name: 'trigger', type: 'trigger' }, { name: 'url', type: 'string' }],
    outputs: [{ name: 'response', type: 'object' }, { name: 'status', type: 'number' }],
    properties: {
      url: { type: 'text', default: '' },
      method: { type: 'select', options: ['GET', 'POST', 'PUT', 'DELETE'], default: 'GET' },
    },
  },
  'json-parse': { label: 'JSON parse', category: 'transform',
    inputs: [{ name: 'input', type: 'string' }],
    outputs: [{ name: 'output', type: 'object' }], properties: {} },
};
```

## Port type colours — Forge tokens

| Type | Colour |
|---|---|
| `trigger` | `var(--fg-0)` |
| `string` | `var(--success)` |
| `number` | `var(--info)` |
| `boolean` | `var(--danger)` |
| `object` | `var(--accent)` |
| `array` | `var(--warning)` |
| `any` | `var(--fg-3)` |

Connections take their colour from the source port's type.

## Canvas interactions

**Adding nodes** — click a palette entry (a `Card` per category, `Badge` chips per
type) to add at the viewport centre.

**Connecting ports** — pointer-down on an output port arms a temp connection; a memo
renders a bezier from the port to the live cursor position (a signal updated on
`onPointerMove`); pointer-up on a compatible input port commits, elsewhere cancels:

```jsx
const [pending, setPending] = createSignal(null); // {fromNode, fromPort, type, x, y}

const portDown = (nodeId, port) => (e) => {
  e.stopPropagation();
  setPending({ fromNode: nodeId, fromPort: port.name, type: port.type,
               x: e.clientX, y: e.clientY });
};
const portUp = (nodeId, port) => (e) => {
  const p = pending();
  if (p && compatible(p.type, port.type)) {
    setGraph('connections', (c) => [...c,
      { id: crypto.randomUUID(), fromNode: p.fromNode, fromPort: p.fromPort,
        toNode: nodeId, toPort: port.name }]);
    saveDocDebounced('connections', unwrap(graph).connections);
  }
  setPending(null);
};
```

**Panning/zooming** — wrap canvas content in a `<div>` with
`transform: translate(vx, vy) scale(zoom)`; wheel zooms (clamped 0.1–3), middle-drag or
Space+drag pans. Store the viewport in `data: state` so reloads restore it.

**Selection/deletion** — click selects (node border / connection stroke →
`var(--accent)`); a `keydown` listener on the document handles Delete/Backspace (skip
when the event target is an input).

**Grid background** — pure CSS on `.pp-canvas`, no drawing loop:

```css
.pp-canvas.pp-grid {
  background-image: radial-gradient(var(--border) 1px, transparent 1px);
  background-size: 20px 20px;
}
```

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: nodes` / `data: connections` | The graph — **the user's design; read it back** |
| `data: state` | Viewport + selection UI state |
| `data: presets` | Named example graphs (also seedable by the agent) |
| `action: validate` | Check the graph (dangling connections, type mismatches, cycles) |
| `action: export` | Generate JSON/config/code from the graph |

Validate action for `server.py`'s `ACTIONS`:

```python
def validate(payload: dict):
    nodes = payload.get("nodes", {})
    issues = [f"connection references missing node: {c[k]}"
              for c in payload.get("connections", [])
              for k in ("fromNode", "toNode") if c[k] not in nodes]
    return {"valid": not issues, "issues": issues}
```

## Prompt output

Describe the graph as a specification, self-contained:

> "Create a data pipeline with the following steps: (1) HTTP request to
> https://api.example.com (GET). (2) Parse the JSON response. (3) Filter results where
> status == 'active'. (4) Map to extract name and email. (5) Write output to
> results.json. Connect them in sequence: HTTP request.response → JSON parse.input →
> Filter.input → Map.input → File write.input."

Include node properties and connection details.

## Example topics

- Data pipeline builder (sources, transforms, sinks)
- API orchestration (HTTP requests, conditions, aggregations)
- State machine editor (states, transitions, guards)
- Workflow automation (triggers, actions, conditions, loops)
- Audio processing chain (input, effects, mixer, output)
- CI/CD pipeline designer (build, test, deploy stages)
