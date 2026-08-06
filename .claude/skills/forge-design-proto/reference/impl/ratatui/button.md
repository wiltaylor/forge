# button — ratatui

<!-- PROTOTYPE (wayfinder #64). -->

status: complete · control page: [button](../../controls/button.md) ·
index: [ratatui](index.md)

One row high. The 32px control height maps to a single cell row; there is no `sm` variant
in a terminal, and asking for one is a no-op rather than an error.

Width is the label plus two spaces of padding on each side. Buttons do not stretch.

Variant selects the style: primary is the accent as a background with the on-accent
foreground; default is the border colour as a foreground on the surface; ghost is plain
text; danger is the danger role as a background.

Focus is a reversed style plus a `>` in the cell before the label. Never a colour change
alone — 256-colour terminals collapse near colours and the focused button becomes
indistinguishable.

There is no hover unless mouse capture is on. With it on, hover is the raised surface as a
background, and the stored rect is hit-tested manually.

Icons are glyphs from the icon table, never an icon font.

`loading` replaces the leading glyph with the spinner frame for the current tick. The
caller drives the tick; the control does not own a timer.

`handle_key` returns `Outcome::Submitted` on Enter or Space, guarded on key **press** —
terminals deliver press and release, and an unguarded handler fires twice.
