# button

<!-- RECAST against the #66 template. The #64 version stated its geometry in Anatomy
     prose; all of it turned out to be laws.md's, and dropped out entirely. -->

Implementations: [solid](../impl/solid/button.md) · [ratatui](../impl/ratatui/button.md) ·
[egui](../impl/egui/button.md)

## What it is for

One action, labelled. `icon-button` when the label is an icon alone. `nav-link` when it
navigates rather than acts.

## Anatomy

A label, optionally an icon before it, optionally an icon after it.

## Variants

| Variant | Use |
|---|---|
| `primary` | One per screen |
| `default` | Everything else |
| `ghost` | Toolbars, table rows |
| `danger` | Destructive |

Sizes are `sm` and the default. There is no large.

## State

`disabled`, `loading`, `pressed`.

## Contract

- `primary` takes the accent solid with the on-accent text role. `default` takes the
  raised surface with a 1px border role. `ghost` takes no fill until hover, then the
  raised surface. `danger` takes the danger solid.
- Variant selects fill and stroke only. Geometry never changes with variant.
- `loading` shows a spinner in place of the leading icon and takes the disabled path. The
  label stays mounted, so the button does not resize.
- Activation while `disabled` or `loading` is a no-op.
- Enter and Space activate.
- The pressed state is visible for the duration of the press, not for a fixed animation.
- Never full width unless the caller asks.

## Accessibility

Role `button`; the element owns Enter and Space; it takes focus itself.

## Platform discretion

The hover model, and whether `sm` exists at all — a platform with one row height may
accept the size and ignore it.
