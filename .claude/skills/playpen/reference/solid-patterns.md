# Playpen Solid Patterns — playground concepts in SolidJS

The scaffold's `App.jsx` demonstrates all core patterns working; this file names them
so templates can reference them. Forge-specific Solid rules (never destructure props,
`class`/`classList`, `onInput`, lucide-solid) are in the forge assets' conventions —
follow them everywhere.

## Single state store

One `createStore` holds every configurable value (the classic playground `state`
object). A `set` wrapper persists on every change:

```jsx
import { createStore, reconcile, unwrap } from 'solid-js/store';
import { loadDoc, saveDocDebounced } from './api';

const DEFAULTS = { radius: 4, paddingX: 12 };
const [state, setState] = createStore({ ...DEFAULTS });

const set = (key, value) => {
  setState(key, value);
  saveDocDebounced('state', unwrap(state));   // 500ms debounce → PUT /api/data/state
};
```

Load persisted state once on mount, merged over defaults so new fields keep working:

```jsx
onMount(async () => {
  const saved = await loadDoc('state');
  if (saved) setState(reconcile({ ...DEFAULTS, ...saved }));
});
```

## Live preview

No `updateAll()` — Solid's reactivity is the update loop. Bind store values straight
into JSX; every control change re-renders exactly what depends on it:

```jsx
<button class={`fbtn fbtn-${state.variant} fbtn-md`}
        style={{ 'border-radius': `${state.radius}px` }}>
  {state.label}
</button>
```

## Prompt output

A `createMemo` that mentions **only non-default values**, phrased as a natural
instruction (qualitative language alongside numbers), never a value dump:

```jsx
const prompt = createMemo(() => {
  const parts = [];
  if (state.radius !== DEFAULTS.radius) parts.push(`a ${state.radius}px corner radius`);
  if (state.shadow > 16) parts.push('a pronounced shadow');
  return parts.length
    ? `Update the card to use ${parts.join(', ')}.`
    : 'The current defaults look right — no changes needed.';
});
```

Copy button with transient feedback:

```jsx
const [copied, setCopied] = createSignal(false);
const copy = async () => {
  await navigator.clipboard.writeText(prompt());
  setCopied(true);
  setTimeout(() => setCopied(false), 1500);
};
// <Button size="sm" icon={copied() ? Check : Copy} onClick={copy}>…
```

## Presets

3–5 named, cohesive configurations plus Reset; applying one replaces the whole store:

```jsx
const PRESETS = { default: {...DEFAULTS}, compact: {...DEFAULTS, radius: 2} };
const applyPreset = (values) => {
  setState(reconcile({ ...values }));
  saveDocDebounced('state', unwrap(state));
};
```

## Layout

Use the `pp-shell` grid from `playpen.css` (controls / preview / prompt areas) with
Forge `Card`s inside each region. Controls stack with `.pp-field-stack`; range inputs
sit inside a bare `.ffield` label, text inputs inside `.ffield-input`. For canvas
templates swap the preview card body for `.pp-canvas`.

## Pointer drag + SVG edges (canvas templates)

For concept-map / code-map / node-grid: nodes live in a store keyed by id; dragging
uses pointer capture; edges are SVG paths derived from node positions (they follow
drags for free through reactivity).

```jsx
const [nodes, setNodes] = createStore({ /* id: {x, y, label, ...} */ });
let drag = null; // {id, dx, dy} — plain variable, not reactive

const down = (id) => (e) => {
  e.currentTarget.setPointerCapture(e.pointerId);
  drag = { id, dx: e.clientX - nodes[id].x, dy: e.clientY - nodes[id].y };
};
const move = (e) => {
  if (!drag) return;
  setNodes(drag.id, { x: e.clientX - drag.dx, y: e.clientY - drag.dy });
};
const up = () => {
  if (!drag) return;
  drag = null;
  saveDocDebounced('nodes', unwrap(nodes));
};

<div class="pp-canvas" onPointerMove={move} onPointerUp={up}>
  <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', 'pointer-events': 'none' }}>
    <For each={edges()}>
      {(e) => <path d={cubicBetween(nodes[e.from], nodes[e.to])}
                    stroke="var(--border-strong)" fill="none" stroke-width="1.5" />}
    </For>
  </svg>
  <For each={Object.keys(nodes)}>
    {(id) => (
      <div onPointerDown={down(id)}
           style={{ position: 'absolute', left: `${nodes[id].x}px`, top: `${nodes[id].y}px` }}
           class="fcard" >…</div>
    )}
  </For>
</div>
```

```js
function cubicBetween(a, b) {
  const mx = (a.x + b.x) / 2;
  return `M ${a.x} ${a.y} C ${mx} ${a.y}, ${mx} ${b.y}, ${b.x} ${b.y}`;
}
```

Notes: keep `drag` as a plain variable (per-frame updates shouldn't churn signals);
persist on pointer-up, not per move; highlight a selected node with
`border-color: var(--accent)`; edge hit-testing (click to select an edge) gets a wider
invisible twin path with `pointer-events: stroke`.

## Components beyond App.jsx

Split into `www/src/components/<Name>.jsx` when App.jsx passes ~200 lines. Pass the
store down via props (`props.state`) — Solid stores stay reactive through props as
long as you don't destructure.
