# button — egui

<!-- PROTOTYPE (wayfinder #64). -->

status: complete · control page: [button](../../controls/button.md) ·
index: [egui](index.md)

Do not use `egui::Button`. Its padding, radius and hover model are not Forge's, and
theming it back is longer than painting it.

Allocate the rect at the control height for the size, plus 12 horizontal padding and the
galley width. Sense `click` and `hover`. Paint in this order: fill, 1px stroke, focus ring,
content. The focus ring is a 2px accent stroke **outside** the rect, not a thicker border —
a thicker border shifts the content.

Variant selects the fill and stroke roles only. Geometry never changes with variant.

Fill roles: primary takes the accent solid with the on-accent text role; default takes the
raised surface with the border role; ghost takes no fill until hover, then the raised
surface; danger takes the danger solid.

Press state comes from `response.is_pointer_button_down_on()`, not from a timer. Hold means
held.

`disabled` uses `ui.add_enabled_ui`, and the widget must still allocate its space — an
early return collapses the layout around it.

`loading` draws a spinner in the leading slot and takes the disabled path. Measure the
galley regardless, so the width holds.

Emit `WidgetInfo::labeled` with `WidgetType::Button` and the label. egui does not do this
for hand-painted widgets.

Return `ForgeResponse` with `Outcome::Submitted` on click, `Ignored` otherwise.
