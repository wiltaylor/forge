# combobox

<!-- RECAST against the #66 template. The claim both Rust pages walked past — "filtering
     is a case-insensitive substring match" — was buried in Anatomy prose here. It is now
     a Contract line, and both Rust pages carry a Contract defect against it. -->

Implementations: [solid](../impl/solid/combobox.md) ·
[ratatui](../impl/ratatui/combobox.md) · [egui](../impl/egui/combobox.md)

## What it is for

A single-select field over a list too long to scan, where the user narrows by typing. It
is `select` plus a filter. Reach for `select` under about a dozen options, `list-box` when
the list is the screen rather than a field, and `command` when the options are actions
rather than values.

## Anatomy

A field, and a popup below it.

The field holds, left to right: a search glyph, a text input, a chevron. It carries the
field label above and help text below, like every form control.

The popup is a list of options. Each option is a label, and a check glyph on the selected
one. When the filter matches nothing, the popup holds a single empty line instead.

## State

Four pieces, and they are not independent:

| State | Meaning |
|---|---|
| `open` | The popup is showing |
| `query` | What the user typed, or **unset** |
| `active` | Which option the keyboard is on. `-1` means none |
| `value` | The committed selection. Owned by the caller |

`query` unset is not the empty string. Unset means "show the selected option's label in
the field". Empty means "the user cleared it, show every option". Collapsing the two is
the most common way to get this control wrong: the field goes blank whenever the popup
opens.

## Contract

- Filtering is a case-insensitive substring match on the option's **label**, never its
  value, and never a ranking.
- `query` unset and `query` empty are distinct states and produce different fields.
- Typing sets `query`, opens the popup, and sets `active` to 0.
- Down opens the popup if closed, then moves `active` down. It stops at the last option
  and does not wrap. Up moves `active` up, stops at the first, and does not wrap.
- Enter commits the option at `active`, closes the popup, and clears `query`.
- Escape closes the popup and clears `query`. The committed `value` does not change.
- Clearing `query` on both Enter and Escape is what returns the field to showing the
  selected label. Miss it and the field keeps a stale search string.
- Focusing the field opens the popup and selects the existing text, so typing replaces it.
- Committing a disabled option is a no-op, and the popup stays open.
- Dismissing by any other route — click away, focus away, the platform's own dismiss —
  behaves as Escape.

## Accessibility

Role `combobox` over a `listbox` of `option`; the field owns every key; the field keeps
focus and the popup never takes it.

## Platform discretion

Whether the popup renders above the field when there is no room below. Whether a caller
may supply its own filter function — the default is the Contract's, and a platform may
not change the default silently.
