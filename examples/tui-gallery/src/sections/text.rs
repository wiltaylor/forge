use forge_tui::prelude::*;
use ratatui::layout::Rect;
use ratatui::Frame;

const SAMPLE: &str = "\
# forge-tui

The **Forge design system** for terminal UIs — the same *dark-default,
dense, technical* aesthetic as the web components.

## Features

- Token-exact theme with `256`/`16`-color degrade
- ~60 widgets, every one a plain ratatui `Widget`
- Opt-in runtime: focus ring, overlays, [toasts](https://forge.dev)

> Widgets never depend on the runtime — drop them into any ratatui app.

```rust
Button::new(\"Deploy\").variant(Variant::Primary)
```

1. Install with `cargo add forge-tui`
2. Run the gallery with `just tui-gallery`
3. ~~Write your own CSS~~ Ship a console
";

pub fn draw(frame: &mut Frame, area: Rect, _ctx: &mut Ctx, t: &Theme) {
    frame.render_widget(
        Markdown::new(SAMPLE).theme(t),
        Rect::new(area.x, area.y, area.width.min(64), area.height),
    );
}
