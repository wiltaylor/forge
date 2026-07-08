# Forge in SolidJS — Setup, Components, Patterns

How to wire the Forge design system into a SolidJS project and build screens with it.

## Project setup

1. Copy the three asset files into the project (e.g. into `src/forge/`):
   ```
   cp ${CLAUDE_SKILL_DIR}/assets/colors_and_type.css <project>/src/forge/
   cp ${CLAUDE_SKILL_DIR}/assets/console.css        <project>/src/forge/
   cp ${CLAUDE_SKILL_DIR}/assets/ui.jsx             <project>/src/forge/
   ```
   A fourth asset, `assets/graph.jsx` (the `NodeGraph` editor), is **optional** — copy it
   only when the project builds node editors / pipeline views. It needs no extra deps and
   imports nothing from `ui.jsx`, but it does need this version's `console.css`.
2. The three files version together — `ui.jsx` components emit classes defined in `console.css`
   (e.g. `Modal` needs the `.fmodal` block), so always copy/update them as a set, never one file.
3. Import the CSS once, in the app entry, tokens first. Find the entry by reading the
   project's `index.html` — the `<script type="module" src="...">` points at it (Vite
   defaults: `src/index.tsx` or `src/main.tsx`):
   ```jsx
   import './forge/colors_and_type.css';
   import './forge/console.css';
   ```
   Fonts (IBM Plex Sans + JetBrains Mono) load via the `@import` inside the token CSS — no extra step.
4. Install the icon set: `npm i lucide-solid` (or `pnpm add` / `bun add` — match the project's lockfile).
   If the project cannot take the dependency, inline SVGs with `stroke="currentColor" stroke-width="1.5"`.
   (`ui.jsx` itself works without lucide — its hamburger and modal-close icons are inline SVGs.)
5. Do **not** add other runtime dependencies for styling. Tailwind is optional; if the project already
   uses it, expose the tokens in `theme.extend` (e.g. `colors: { 'bg-1': 'var(--bg-1)', accent: 'var(--accent)' }`)
   rather than duplicating values.

## Where new code goes

- New components: the project's existing component directory; if it has none, default to
  `src/components/<Name>.jsx`.
- New component CSS: a project-owned stylesheet `src/forge/custom.css`, imported at the app
  entry **after** `console.css`. Create it on first need.
- Never append to the copied `colors_and_type.css`, `console.css`, or `ui.jsx` — keeping them
  pristine lets them be re-synced from the design project later.

## Solid-specific rules (the JSX is plain, but these bite)

- Use `class`, not `className`; use `classList={{ 'is-error': !!props.error }}` for conditional classes.
- **Never destructure props** in a component signature — it breaks reactivity. Use `props.x`,
  or `splitProps` / `mergeProps` as in `assets/ui.jsx`.
- Use `<Show when={...}>` for conditionals and `<For each={...}>` for lists (not `.map`).
- Event handlers are `onClick`, `onInput` (Solid inputs fire `onInput`, not `onChange`, per keystroke).
- lucide-solid icons are components: `import { Terminal } from 'lucide-solid'` then
  `<Terminal size={16} strokeWidth={1.5} />`, or pass through the `Icon` wrapper: `<Icon of={Terminal} />`.

## The primitives (`assets/ui.jsx`)

| Component | Props | Notes |
|---|---|---|
| `Button` | `variant` (`primary`\|`secondary`\|`danger`\|`ghost`), `size` (`sm`\|`md`\|`lg`), `icon` (lucide component), native button props | `secondary`/`md` is the default. One `primary` per view. |
| `Input` | `label`, `help`, `error` (bool), `icon`, native input props | Wraps label + field + help text. |
| `Badge` | `tone` (`neutral`\|`success`\|`warning`\|`danger`\|`info`\|`accent`), `dot` (bool) | Status chips in tables and headers. |
| `Card` | `title`, `action` (JSX for the header right side), `padded` (default true) | Set `padded={false}` when the body is a table. |
| `Stat` | `label`, `value`, `delta`, `tone` (`success`\|`danger`\|`neutral`) | Metric tile — eyebrow label + big tabular number. |
| `Toast` | `tone`, `icon` | Inline notification strip. |
| `StatusDot` | `tone` | 6px solid dot; pair with text, don't use alone. |
| `Kbd` | children | Keyboard shortcut chip. |
| `Icon` | `of` (lucide component), `size` | Forge defaults: 16px, 1.5px stroke, `currentColor`. |

Shell & navigation (classes shown so raw-class markup stays discoverable):

| Component | Props | Notes |
|---|---|---|
| `AppShell` | `topbar` (JSX), `sidebar` (JSX), children → main | The whole `.app-shell` grid. **Owns the mobile drawer**: hamburger, scrim and close-on-nav-tap are built in — zero wiring. |
| `NavSection` | children | `.fsidebar-section` group label. |
| `NavLink` | `href`, `icon`, `active`, `count`, native `<a>` props | Sidebar link; `.is-active` rail + right-aligned `.count`. |
| `Crumbs` | `items` (array) | `.ftopbar-crumbs` with `/` separators. |
| `IconButton` | `icon`, `label` (aria-label + title), native button props | `.ftopbar-icon-btn` for bell/theme/user. |

Page & data:

| Component | Props | Notes |
|---|---|---|
| `PageHead` | `title`, `sub`, `actions` (JSX) | `.page-head` header row. |
| `Grid` | children, native div props | `.fgrid` auto-fit tile grid (stat rows, card grids). |
| `Table` | children (`<thead>`/`<tbody>`), native table props | `.ftable-wrap > .ftable` — mobile scroll built in. Markup-only; render rows with `<For>`. |
| `Logs` / `LogLine` | native div props / `time`, `level` (`info\|warn\|error\|debug`), children | `.flogs` container of `.flog-line` rows. |
| `SettingsLayout` | `nav` (JSX links), children | `.settings-layout` with sticky `.settings-nav`. |
| `SettingsSection` | `title`, `sub`, children | `.settings-section` card. |
| `SettingsRow` | children | `.settings-row` two-column field grid. |
| `Empty` | `title`, `action` (JSX), children (one sentence) | `.empty` dashed empty state. |
| `Eyebrow` | children | `.eyebrow` micro-label. |
| `Modal` | `open`, `onClose`, `title`, `footer` (JSX), children | Portal + `.fmodal`; closes on Escape, backdrop click, head X. Controlled by a signal. **Requires the `.fmodal` block in console.css — copy both files together.** |

Form controls (all controlled; Checkbox/Toggle/Radio keep a hidden native input for a11y):

| Component | Props | Notes |
|---|---|---|
| `Checkbox` | `checked`, `onChange(bool)`, `indeterminate`, `disabled`, children = label | `.fcheck`; indeterminate renders an accent dash. |
| `Toggle` | `checked`, `onChange(bool)`, `disabled`, children = label | `.ftoggle` switch — pill track is a sanctioned radius exception. |
| `Radio` / `RadioGroup` | group: `options [{value,label,disabled?}]`, `value`, `onChange`, `label?`, `row?` | `.fradio`; group name auto-generated. |
| `Select` | `options`, `value`, `onChange`, `placeholder`, `label`, `help`, `error`, `disabled` | Custom `.fselect-pop` popover (z 60 — works inside modals); Arrow/Enter/Escape/Home/End keys. Caveat: the popover is positioned in-flow, so inside a *scrolling* `.fmodal-body` it can clip — keep modal selects near the top or size the body. |
| `ListBox` | `options` + single `value`/`onChange`, or `multiple` + `values[]`/`onChange(values)`, `label?` | `.flistbox` scrollable option list; multi rows get check indicators; keyboard nav. |
| `Progress` | `value` 0–100, `tone` (`accent`\|`success`\|`warning`\|`danger`), `label?`, `showValue?`, `indeterminate?` | 4px `.fprogress` bar — the default Forge loading treatment. |
| `Spinner` | `size` (16), `label` | Inline arc spinner, `currentColor` — for inline/button waits only. |

## Node graphs (`assets/graph.jsx`, optional)

`NodeGraph` is a **controlled** node editor: nodes/edges/selection live in the consumer's
store; the component reports interactions via callbacks. Elbow (orthogonal) edge routing,
typed connection ports, drag-to-move, drag-to-connect, click-to-select, Delete-to-remove.

```jsx
import { NodeGraph } from './forge/graph';

<NodeGraph
  nodes={[{ id: 'fetch', x: 40, y: 60, title: 'HTTP request',
            inputs:  [{ id: 'run',  type: 'trigger' }],
            outputs: [{ id: 'body', type: 'object' }, { id: 'status', type: 'number' }] }]}
  edges={[{ id: 'e1', from: { node: 'fetch', port: 'body' },
            to: { node: 'parse', port: 'raw' }, state: 'active' }]}  // 'active' | 'broken' | default
  selected={sel()}                          // null | {kind: 'node'|'edge', id}
  onNodeMove={(id, x, y) => …}              // fires per pointermove — write to your store
  onConnect={({ from, to }) => …}           // append the edge (types already validated)
  onSelect={setSel}                         // null on background click
  onDelete={(sel) => …}                     // Delete/Backspace with a selection
  style={{ height: '460px' }}               // consumer sizes the canvas
/>
```

- Edge `state`: `active` = marching-ants animation (flow direction out → in); `broken` =
  flashing danger; default = solid in the source port's type colour. Under reduced motion
  both fall back to static, legible styles.
- Port types → colours (`PORT_COLORS` export): trigger `--fg-0`, string `--success`,
  number `--info`, boolean `--danger`, object `--accent`, array `--warning`, any `--fg-3`.
  `any` connects to everything; otherwise types must match.
- Nodes are fixed-width (`--fgraph-node-w`, 180px; per-node `w` override) so edge anchors
  are computed without DOM measurement. Optional per-node body: pass a render-prop child
  `{(node) => JSX}`.
- Future work (not yet built): pan/zoom viewport, auto-width nodes.

<examples>
<example name="dashboard-card">
```jsx
import { For } from 'solid-js';
import { RefreshCw, GitBranch } from 'lucide-solid';
import { Button, Card, Badge, Stat } from './forge/ui';

function DeploysCard(props) {
  return (
    <Card title="Recent deploys" padded={false}
          action={<Button variant="ghost" size="sm" icon={RefreshCw}>Refresh</Button>}>
      <table class="ftable">
        <thead>
          <tr><th>Service</th><th>Commit</th><th>Status</th><th>When</th></tr>
        </thead>
        <tbody>
          <For each={props.deploys}>
            {(d) => (
              <tr>
                <td>{d.service}</td>
                <td class="col-mono">{d.sha.slice(0, 7)}</td>
                <td><Badge tone={d.ok ? 'success' : 'danger'} dot>{d.ok ? 'deployed' : 'failed'}</Badge></td>
                <td class="col-mono">{d.ago}</td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </Card>
  );
}
```
</example>
</examples>

## Layout & screen patterns (classes from `console.css`)

- **App shell**: `<AppShell topbar={…} sidebar={…}>` (drawer built in) — or raw
  `<div class="app-shell">` containing `.ftopbar`/`.fsidebar`/`.app-main`. Topbar 48px,
  sidebar 240px, both sticky; at ≤1024px the sidebar becomes an off-canvas drawer
  (see "Responsive patterns").
- **Sidebar nav**: `.fsidebar-section` uppercase group labels; links get `.is-active` for the current
  route (accent inset rail comes free); `<span class="count">` right-aligns a mono counter.
- **Page header**: `.page-head` with `<h1>` + `.sub` caption on the left, `.page-actions` buttons right.
- **Topbar**: `.ftopbar-brand`, `.ftopbar-crumbs` (separator `<span class="sep">/</span>`),
  `.ftopbar-search`, `.ftopbar-icon-btn` for bell/theme/user.
- **Tables**: `<Card padded={false}><Table><thead>…` — sticky uppercase headers, 32px hoverable
  rows, `.col-mono` for IDs/times. `Table` renders `.ftable-wrap > .ftable`, which scrolls the
  table horizontally at ≤768px so it never forces page-level scroll.
- **Logs**: `.flogs` container of `.flog-line` grids (`.flog-time` / `.flog-level info|warn|error|debug` /
  `.flog-msg`). Mono, dense, on `--bg-0`.
- **Settings**: `<SettingsLayout nav={…}><SettingsSection title …><SettingsRow>` — sticky nav +
  section cards + two-column field grids.
- **Empty states**: `<Empty title="…" action={…}>one sentence</Empty>`. Never a paragraph.
- **Stat rows**: `<Grid>` of `<Card><Stat …/></Card>` tiles — auto-fit columns, collapses to one
  column on narrow screens with no media query.
- **Modals**: `<Modal open={sig()} onClose={…} title footer={…}>` — Portal-rendered `.fmodal`:
  `--bg-1` panel, `--r-lg`, `border-strong`, dimmed blurred backdrop, enter over `--dur-3`;
  closes on Escape / backdrop / X.

## Responsive patterns

The component CSS is responsive out of the box (breakpoints in `reference/tokens.md`), and
`AppShell` owns the mobile drawer — hamburger, scrim, and close-on-nav-tap need zero wiring:

```jsx
import { AppShell, NavSection, NavLink, Crumbs, IconButton } from './forge/ui';
import { LayoutGrid, Bell } from 'lucide-solid';

<AppShell
  topbar={<>
    <div class="ftopbar-brand"><strong>console</strong></div>
    <Crumbs items={['fleet', 'nodes']} />
    <div style={{ flex: 1 }} />
    <IconButton icon={Bell} label="Notifications" />
  </>}
  sidebar={<>
    <NavSection>Fleet</NavSection>
    <NavLink href="/" icon={LayoutGrid} active count={12}>Overview</NavLink>
  </>}
>
  {/* page content */}
</AppShell>
```

- Fallback (raw-class shells that can't use `ui.jsx`): add a `createSignal(false)`, toggle
  `is-sidebar-open` on `.app-shell` via `classList`, render a
  `button.ftopbar-icon-btn.fsidebar-toggle` first in the topbar and a `div.fscrim` after the
  nav, both clicking the signal closed. The toggle and scrim are `display: none` on desktop,
  so the markup is inert above 1024px.
- Don't hide content to make a screen fit — reflow it. The only sanctioned removal is the
  breadcrumbs at ≤768px (the page `<h1>` carries location).
- Test widths: **1280** (desktop truth), **1024** and **768** (breakpoint edges), **375** (phone).

## Copy rules (UI strings you generate)

- Sentence case everywhere. Buttons are 1–3 word verbs ("Deploy", "Roll back").
- Errors: what failed → why → what to do, one line each. Calm, no exclamation marks, no emoji.
- Numbers always carry units with a space (`12 ms`, `4.2 GB`); relative time in lists ("2m ago"),
  absolute UTC on hover/detail.

## Verifying a screen

After building, run the app: read the `scripts` in `package.json` and use `dev` (or `start`),
with the package manager the lockfile implies. If no runnable script exists, skip the live
render, do a static pass of the tokens.md checklist instead, and say so in your summary.
Then check: dark theme renders
by default; toggling `data-theme="light"` on `<html>` keeps everything legible; keyboard-Tab shows
2px accent focus rings; at 375px wide there is no page-level horizontal scroll and the drawer
opens/closes from the hamburger. If a component looks off, diff its CSS usage against the
checklist in `${CLAUDE_SKILL_DIR}/reference/tokens.md` before inventing new styles.
