# combobox

<!-- PROTOTYPE (wayfinder #64). The fields on this page are #66's decision, not #64's.
     This page exists at realistic depth so the re-read bill is measurable. -->

Implementations: [solid](../impl/solid/combobox.md) ·
[ratatui](../impl/ratatui/combobox.md) · [egui](../impl/egui/combobox.md)

## What it is for

A single-select field over a list too long to scan, where the user narrows by typing. It
is `select` plus a filter. Reach for `select` under about a dozen options, `list-box` when
the list is the screen rather than a field, and `command` when the options are actions
rather than values.

## Anatomy

A field, and a popup below it.

The field is a `ffield-input` shell holding, left to right: a search glyph, a text input,
a chevron. It carries the field label above and help text below, like every form control.

The popup is a list of options. Each option is a label, and a check glyph on the selected
one. When the filter matches nothing, the popup holds a single empty line instead.

## State

Four pieces of state, and they are not independent:

| State | Meaning |
|---|---|
| `open` | The popup is showing |
| `query` | What the user typed, or **unset** |
| `active` | Which option the keyboard is on. `-1` means none |
| `value` | The committed selection. Owned by the caller |

`query` unset is not the empty string. Unset means "show the selected option's label in
the field". Empty string means "the user cleared it, show every option". Collapsing the
two is the most common way to get this control wrong: the field goes blank whenever the
popup opens.

Filtering is a case-insensitive substring match on the option's **label**, never its
value.

## Interaction contract

Normative on all three platforms.

- Typing sets `query`, opens the popup, and sets `active` to 0.
- Down arrow opens the popup if closed, then moves `active` down. It stops at the last
  option; it does not wrap.
- Up arrow moves `active` up. It stops at the first; it does not wrap.
- Enter commits the option at `active`, closes the popup, and clears `query`.
- Escape closes the popup and clears `query`. The committed `value` does not change.
- Focusing the field opens the popup and selects the existing text, so typing replaces it.
- Committing a disabled option is a no-op — the popup stays open.
- Dismissing by any other route (click away, focus away, the platform's dismiss) behaves
  as Escape.

Clearing `query` on both Enter and Escape is what returns the field to showing the
selected label. Miss it and the field keeps showing a stale search string.

## Accessibility

Coarse only. The field is a combobox and reports whether the popup is open. The popup is a
listbox; its children are options, and the selected one says so. The field keeps focus the
whole time — the popup never takes it. Fine ARIA detail is in
[anti-patterns.md](../anti-patterns.md).

## Not normative

Whether the popup renders above the field when there is no room below. Whether filtering
is substring or fuzzy — substring is the Forge default and a platform may not change it
silently, but a caller may supply its own filter.
