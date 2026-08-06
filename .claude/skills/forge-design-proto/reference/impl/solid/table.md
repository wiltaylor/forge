# table — SolidJS

<!-- RECAST against the #66 template. -->

control page: [table](../../controls/table.md)

## Shape

```
div.ftable-wrap                 the scroll container
  table.ftable
    thead > tr > th.ftable-h    .is-sorted-asc .is-sorted-desc .is-num
    tbody > tr.ftable-row       .is-active .is-selected
      td.ftable-cell            .is-num
```

## What the browser gives you

A real `<table>` gives you the semantics, the header association, the reading order and
the scrolling. You supply the keyboard model and the truncation, and nothing else.

## Mechanism

Sticky headers are `position: sticky; top: 0` on the `th`, **not** on the `thead` — Safari
ignores it on the section. The scroll container is the wrapper, never the page.

Right-align numeric columns with `.is-num`, which also applies tabular numerals. Do not
right-align in a style attribute; the class carries both halves of the Contract line.

Truncation is `overflow: hidden; text-overflow: ellipsis; white-space: nowrap` on the
cell, and it needs a `max-width` on the cell to take effect at all. Put the full value in
a `title` so it stays reachable.

## Keyboard

The body is one tab stop: `tabindex="0"` on `tbody`, with the arrow keys handled there.
Rows are not focusable. Use the shared roving-index primitive rather than tracking the
active row by hand.

Scroll-into-view for the active row uses `scrollIntoView({ block: 'nearest' })` on the row
element. `nearest` matters — the default scrolls the row to the top on every arrow press.

## Rows

`For` over rows, keyed by the caller's row key. An index key re-renders the whole body on
a sort.

Sorting is requested only. Fire the callback; do not sort a local copy.
