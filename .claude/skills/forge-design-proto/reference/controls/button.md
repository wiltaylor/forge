# button

<!-- PROTOTYPE (wayfinder #64). Deliberately short — this page demonstrates that an easy
     control is simply a short page, with no rule deciding which controls are "important". -->

Implementations: [solid](../impl/solid/button.md) · [ratatui](../impl/ratatui/button.md) ·
[egui](../impl/egui/button.md)

## What it is for

One action, labelled. `icon-button` when the label is an icon alone. `nav-link` when it
navigates rather than acts.

## Anatomy

A label, optionally an icon before it, optionally an icon after it. 32px high, 4px radius,
12px horizontal padding. Never full width unless the caller asks.

## Variants

| Variant | Look | Use |
|---|---|---|
| `primary` | Accent solid | One per screen |
| `default` | `surface-raised`, 1px `border` | Everything else |
| `ghost` | No border, no fill until hover | Toolbars, table rows |
| `danger` | `danger` solid | Destructive, and never the default focus |

Sizes are `sm` (24px) and the 32px default. There is no large.

## State

`disabled`, `loading`, `pressed`. `loading` shows a `spinner` in place of the leading icon
and disables the button — it does not replace the label, so the button does not resize.

## Interaction contract

Enter and Space activate it. Activation while `disabled` or `loading` is a no-op. The
press state is visible for the duration of the press, not a fixed animation.

## Accessibility

It is a button. It has an accessible name. That is the whole requirement.
