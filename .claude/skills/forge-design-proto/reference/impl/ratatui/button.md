# button — ratatui

<!-- RECAST against the #66 template. The missing `sm` size used to read like a quiet
     departure; the control page now names it as platform discretion, so it is an
     ordinary statement here rather than something the reader has to judge. -->

control page: [button](../../controls/button.md)

## Shape

One cell row. Width is the label plus two spaces of padding on each side. Buttons do not
stretch.

```
  > [ icon label ]        focused: reversed style, plus `>` in the cell before
    [ icon label ]        unfocused
```

## What ratatui gives you

Nothing but a cell grid. No focus, no focus ring, no hover, no hit-testing, no roles. The
caller owns focus and routes keys to whichever control holds it; this control assumes it
has focus whenever `handle_key` is called.

## Mechanism

The pressed and focused states are a **reversed style** plus a `>` gutter marker, not a
ring and not a colour change. A colour change alone is unreadable — 256-colour terminals
collapse near colours and the focused button becomes indistinguishable from its
neighbours. Do not reach for a border to look more like the graphical platforms; a
one-row control has nowhere to put one.

Icons are glyphs from the icon table, never an icon font. This is the inverse of SolidJS
and egui, and it is deliberate.

## Sizes

There is no `sm` in a terminal — the control page names size as platform discretion, and
this platform accepts it and ignores it. Asking for `sm` is a no-op, not an error.

## Variants

Primary is the accent as a background with the on-accent foreground. Default is the border
colour as a foreground on the surface. Ghost is plain text. Danger is the danger role as a
background.

## Hover

There is none unless mouse capture is on. With it on, hover is the raised surface as a
background, and the stored rect is hit-tested manually.

## Keys and loading

`handle_key` returns `Outcome::Submitted` on Enter or Space, guarded on key **press** —
terminals deliver press and release, and an unguarded handler fires twice.

`loading` replaces the leading glyph with the spinner frame for the current tick. The
caller drives the tick; the control does not own a timer.
