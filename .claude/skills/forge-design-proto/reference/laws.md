# Forge laws

<!-- PROTOTYPE (wayfinder #64). Sized to be realistic, not complete. The real content is
     an extraction job (#68); the fields of a page are #66. This file exists so the
     re-read bill can be measured and so a routing test has somewhere real to land. -->

True on every platform. A control page may add to these. Nothing may contradict them.

## Colour

Forge is dark by default and defines a light theme. Never ship light-only.

Every colour is a named role, never a literal. The roles are:

| Role | Use |
|---|---|
| `surface` | The page background |
| `surface-raised` | Cards, popovers, anything above the page |
| `surface-sunken` | Wells, code blocks, log bodies |
| `border` | Every 1px division |
| `text` | Body copy |
| `text-dim` | Labels, help text, units, timestamps |
| `accent` | One per screen: focus, selection, the primary action |
| `danger`, `warn`, `ok`, `info` | Status only |

The accent never fills a large area. It is a 1–2px ring, a small solid on one button, or
a text colour. An accent-filled panel is wrong on every platform.

Status colour never carries meaning alone. Pair it with a glyph or a word, because 8% of
readers cannot separate `danger` from `ok`.

## Density

Forge is dense. A control is 32px high, or 1 cell in a terminal. Body text is 14px. Rows
in a table or a list are 32px, and they do not grow to fit their content — they truncate.

Numbers are tabular and carry their unit. `1.4 GB`, not `1400000000`.

Sentence case everywhere. No title case, no all-caps except `eyebrow`.

## Focus

Exactly one thing holds focus. Focus is always visible — a 2px accent ring on SolidJS and
egui, a reversed cell or a `>` gutter marker in a terminal. Never remove the indicator to
make something look cleaner.

Tab moves between controls. Arrow keys move within a control. A control that swallows Tab
is wrong unless it is a text area, and then only with a documented escape.

Escape closes the innermost open thing, one layer per press. It never closes two.

## Composition

<!-- #73: a page that names a role spells it in the same breath. The ten class names in
     this section are the ten unprefixed classes — the one family exempt from the `f`
     grammar — so they are spelled here rather than left to a lookup. -->

Every screen is `app-shell` > `page-head` > content. Nothing sits outside the shell.

On SolidJS these ten names are literal class names, and they carry no `f` prefix:
`app-shell`, `app-main`, `page-head`, `page-actions`, `empty`, `eyebrow`,
`settings-layout`, `settings-nav`, `settings-section`, `settings-row`. Every other Forge
class does carry the prefix — see `reference/solidjs.md`.

`page-head` carries the title and **one** primary action. Secondary actions go into the
content or a `dropdown-menu`, never into the head.

`tabs` sit directly under `page-head` and switch content only. Tabs never change the
shell, the nav selection, or the crumbs.

`crumbs` show the path to the current screen and collapse from the left when short of
room. They are never the primary navigation.

A form is one column. It stays one column under 768px, and it does not become two above
it — Forge forms are single-column at every width.

Group related fields under a heading, not inside a card. A card is for content that has
its own identity — a record, a chart, a log stream. Settings groups are headings.

A settings screen is `settings-layout` > `settings-section` > `settings-row`. One control
per row, the label on the left, the control on the right, help text under the label.

Empty states use `empty`, and they say what to do next, not that there is nothing here.

Destructive actions are `danger` and never the default focus target.

## Motion

Motion is 120ms, ease-out, and only on: overlay entry, overlay exit, and state colour
changes. Nothing else moves. Honour the platform's reduced-motion setting by dropping
duration to zero — not by removing the state change.

## Text

Every visible string is a prop or a parameter. Nothing user-facing is hardcoded inside a
control.

Truncate with an ellipsis at the end, and keep the full value reachable — a tooltip on
SolidJS and egui, a detail line in a terminal.
