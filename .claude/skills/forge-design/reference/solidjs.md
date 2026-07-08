# Forge in SolidJS — Setup, Components, Patterns

How to wire the Forge design system into a SolidJS project and build screens with it.

## Project setup

1. Copy the three asset files into the project (e.g. into `src/forge/`):
   ```
   cp ${CLAUDE_SKILL_DIR}/assets/colors_and_type.css <project>/src/forge/
   cp ${CLAUDE_SKILL_DIR}/assets/console.css        <project>/src/forge/
   cp ${CLAUDE_SKILL_DIR}/assets/ui.jsx             <project>/src/forge/
   ```
2. Import the CSS once, in the app entry, tokens first. Find the entry by reading the
   project's `index.html` — the `<script type="module" src="...">` points at it (Vite
   defaults: `src/index.tsx` or `src/main.tsx`):
   ```jsx
   import './forge/colors_and_type.css';
   import './forge/console.css';
   ```
   Fonts (IBM Plex Sans + JetBrains Mono) load via the `@import` inside the token CSS — no extra step.
3. Install the icon set: `npm i lucide-solid` (or `pnpm add` / `bun add` — match the project's lockfile).
   If the project cannot take the dependency, inline SVGs with `stroke="currentColor" stroke-width="1.5"`.
4. Do **not** add other runtime dependencies for styling. Tailwind is optional; if the project already
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

- **App shell**: one grid — `<div class="app-shell">` containing `<header class="ftopbar">`,
  `<nav class="fsidebar">`, `<main class="app-main">`. Topbar 48px, sidebar 240px, both sticky.
  At ≤1024px the sidebar becomes an off-canvas drawer — wire it per "Responsive patterns" below.
- **Sidebar nav**: `.fsidebar-section` uppercase group labels; links get `.is-active` for the current
  route (accent inset rail comes free); `<span class="count">` right-aligns a mono counter.
- **Page header**: `.page-head` with `<h1>` + `.sub` caption on the left, `.page-actions` buttons right.
- **Topbar**: `.ftopbar-brand`, `.ftopbar-crumbs` (separator `<span class="sep">/</span>`),
  `.ftopbar-search`, `.ftopbar-icon-btn` for bell/theme/user.
- **Tables**: `.ftable` — sticky uppercase headers, 32px hoverable rows, `.col-mono` for IDs/times.
  Standard markup: `<Card padded={false}><div class="ftable-wrap"><table class="ftable">…` — the
  wrap div scrolls the table horizontally at ≤768px so it never forces page-level scroll.
- **Logs**: `.flogs` container of `.flog-line` grids (`.flog-time` / `.flog-level info|warn|error|debug` /
  `.flog-msg`). Mono, dense, on `--bg-0`.
- **Settings**: `.settings-layout` (sticky `.settings-nav` + `.settings-section` cards, `.settings-row`
  two-column field grid).
- **Empty states**: `.empty` — dashed border, one sentence + one action button. Never a paragraph.
- **Stat rows**: `<div class="fgrid">` of `<Card><Stat …/></Card>` tiles — auto-fit columns,
  collapses to one column on narrow screens with no media query.
- **Modals**: `--bg-1` panel, `--r-lg`, `1px solid var(--border-strong)`; backdrop
  `rgb(0 0 0 / 0.5)` + `backdrop-filter: blur(4px)`; enter over `--dur-3`.

## Responsive patterns

The component CSS is responsive out of the box (breakpoints in `reference/tokens.md`); the one
thing each app must wire is the sidebar drawer — a signal plus three elements:

```jsx
import { createSignal } from 'solid-js';
import { Menu } from 'lucide-solid';
import { Icon } from './forge/ui';

function AppShell(props) {
  const [navOpen, setNavOpen] = createSignal(false);
  const close = () => setNavOpen(false);
  return (
    <div class="app-shell" classList={{ 'is-sidebar-open': navOpen() }}>
      <header class="ftopbar">
        <button class="ftopbar-icon-btn fsidebar-toggle" aria-label="Toggle navigation"
                aria-expanded={navOpen()} onClick={() => setNavOpen((o) => !o)}>
          <Icon of={Menu} />
        </button>
        {/* brand, crumbs, search … */}
      </header>
      <nav class="fsidebar" onClick={close}>{/* sections + links */}</nav>
      <div class="fscrim" onClick={close} />
      <main class="app-main">{props.children}</main>
    </div>
  );
}
```

- The toggle and scrim are `display: none` on desktop, so this markup is inert above 1024px.
- `onClick={close}` on the `<nav>` closes the drawer after a link tap (harmless on desktop).
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
