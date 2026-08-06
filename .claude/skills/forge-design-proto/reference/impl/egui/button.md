# button — egui

<!-- RECAST against the #66 template. -->

control page: [button](../../controls/button.md)

## Shape

Do not use `egui::Button`. Its padding, radius and hover model are not Forge's, and
theming it back is longer than painting it.

Allocate the rect at the control height for the size, plus 12 horizontal padding and the
galley width. Sense `click` and `hover`. Paint in this order: fill, 1px stroke, focus
ring, content.

## What egui gives you

Layout, input sensing and a focus id. It gives you no accessibility at all, and no
styling that matches Forge.

## Mechanism

The focus ring is a 2px accent stroke **outside** the rect, not a thicker border. A
thicker border shifts the content, and the button appears to twitch as focus moves.

`disabled` uses `ui.add_enabled_ui`, and the widget must still allocate its space — an
early return collapses the layout around it. `loading` draws a spinner in the leading slot
and takes the disabled path. Measure the galley regardless, so the width holds.

## Fill roles

Primary takes the accent solid with the on-accent text role. Default takes the raised
surface with the border role. Ghost takes no fill until hover, then the raised surface.
Danger takes the danger solid. Geometry never changes with variant.

## Press

Press state comes from `response.is_pointer_button_down_on()`, not from a timer. Hold
means held.

## Accessibility

Emit `WidgetInfo::labeled` with `WidgetType::Button` and the label. egui does not do this
for hand-painted widgets, and nothing warns you.

## Outcome

Return `ForgeResponse` with `Outcome::Submitted` on click, `Ignored` otherwise.
