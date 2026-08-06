# table — ratatui

<!-- RECAST against the #66 template. -->

control page: [table](../../controls/table.md)

## Shape

Built on ratatui's `Table` and `TableState`, but the Forge behaviour is not the default.

Column widths are `Constraint`s the caller supplies. Forge's rule: text columns take
`Min`, numeric columns take `Length` at the widest formatted value, and exactly one column
takes `Fill`. A table of all-`Percentage` columns reflows on every resize and is wrong.

The header row is one row. It sits outside the scroll region, so it stays put for free.

## What ratatui gives you

Cell layout and a table primitive. No scrolling, no scroll-into-view, no truncation, no
hit-testing, no accessibility.

## Mechanism

**Truncate by hand** at the column width with a `…` in the last cell. ratatui does not
truncate for you — it wraps, and a wrapped row breaks the fixed row height the Contract
requires.

**Scrolling is manual.** Store the viewport height at draw time, keep an offset, and clamp
the offset so the active row stays inside it. There is no scroll-into-view to call.

**Right-align numeric cells with `Alignment::Right` per cell.** There is no tabular-numerals
notion to apply — every terminal cell is already one width, so the alignment is the whole
of that Contract line here.

**The active row is a reversed style, and selected rows carry a `*` in a leading gutter
column.** Never colour alone for either; 256-colour terminals collapse near colours.

## Keys and mouse

`handle_key` maps Up, Down, PageUp, PageDown, Home and End, guarded on key press, and
returns `Outcome::Changed` when the active row moves and `Submitted` on Enter. Space
toggles selection and returns `Changed`.

Mouse: store the body rect at draw time and hit-test rows against it. Wheel moves the
active row, not the offset — one cursor, always visible.
