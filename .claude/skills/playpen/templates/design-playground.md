# Design Playground Template

Use this template when the playpen is about visual design decisions: components,
layouts, spacing, color, typography, animation, responsive behavior.

The scaffold's starter `App.jsx` is already a minimal design playground — extend it
rather than starting over.

## Layout

The standard `pp-shell` grid: controls (left `Card`s) → live preview (right `Card` with
`.pp-stage`) → prompt output (bottom `Card` with copy button). Group controls into one
`Card` per concern (Spacing, Color, Typography, Border, Interaction) using
`.pp-field-stack`.

## Control types

| Decision | Control | Example |
|---|---|---|
| Sizes, spacing, radius | `<input type="range">` in a `.ffield` | border-radius 0–24px |
| On/off features | `<input type="checkbox">` in a `.ffield` | show border, hover effect |
| Choosing from a set | `<select>` in `.ffield-input` | font-family, easing curve |
| Colors | Hue/sat/lightness range sliders | shadow color, accent |
| Layout structure | Clickable `Card`s (selected = `border-color: var(--accent)`) | sidebar-left / top-nav |
| Responsive behavior | Viewport-width slider resizing the stage | watch grid reflow |

## Preview rendering

Bind store values straight into the preview JSX (see `reference/solid-patterns.md` —
no render function needed):

```jsx
<div class="pp-stage">
  <div style={{
    'border-radius': `${state.radius}px`,
    padding: `${state.padding}px`,
    background: 'var(--bg-1)',
    border: '1px solid var(--border)',
  }}>…</div>
</div>
```

The preview subject may use raw CSS values — that is the thing being designed. The
playpen chrome around it stays on Forge tokens. If light/dark context matters, wrap the
stage in a `data-theme` toggle: `<div data-theme={ctx()}>` with a `Button` pair — the
Forge tokens re-theme any subtree.

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: state` | Whole control state, saved via `saveDocDebounced('state', …)` |
| `action: export` | Optional — generate CSS custom properties / design tokens from state |

Export action for `server.py`'s `ACTIONS`:

```python
def export_tokens(payload: dict):
    css = ":root {\n" + "".join(f"  --{k}: {v};\n" for k, v in payload.items()) + "}\n"
    return {"css": css}
```

## Prompt output

Frame it as a direction to a developer, not a spec sheet:

> "Update the card to feel soft and elevated: 12px border-radius, 24px horizontal
> padding, a medium box-shadow (0 4px 12px rgba(0,0,0,0.1)). On hover, lift it with
> translateY(-1px) and deepen the shadow slightly."

Only mention non-default values. Use qualitative language alongside numbers. If the
user works in Tailwind, suggest Tailwind classes; if raw CSS, CSS properties.

## Example topics

- Button style explorer (radius, padding, weight, hover/active states)
- Card component (shadow depth, radius, content layout, image)
- Layout builder (sidebar width, content max-width, header height, grid)
- Typography scale (base size, ratio, line heights across h1-body-caption)
- Color palette generator (primary hue, derive secondary/accent/surface)
- Dashboard density (airy to compact slider that scales everything)
- Modal/dialog (width, overlay opacity, entry animation, corner radius)
