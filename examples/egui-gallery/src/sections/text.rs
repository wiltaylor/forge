//! Markdown: the styled markdown renderer (feature `markdown`).

use forge_egui::prelude::*;

const SAMPLE: &str = r#"# Release notes — forge 0.4

Forge ships a **markdown renderer** for every kit: *web*, *terminal*, and
now `egui`. It maps headings, emphasis, and code onto the design tokens —
see the [design system docs](https://forge.dev/docs) or write to
[the team](mailto:forge@wiltaylor.dev).

## Highlights

- Token-exact styling for **bold**, *italics*, and ~~mistakes~~
- Inline code chips: run `cargo add forge-egui`
- Sanitized links — [this one is hostile](javascript:alert(1)) and renders
  as plain text

### Upgrade steps

1. Bump the dependency
2. Call `Theme::apply` once at startup
3. Replace hand-rolled labels with `Markdown::new(src).show(ui)`

> Markdown bodies are used by the chat kit for every message bubble,
> so anything an assistant writes lands here.

```rust
use forge_egui::prelude::*;

fn ui(ui: &mut egui::Ui) {
    let _ = Markdown::new("**hello** from _forge_").show(ui);
}
```

| Kit | Crate | Status |
| --- | ----- | ------ |
| web | `@forge/markdown` | shipped |
| tui | `forge-tui` | shipped |
| egui | `forge-egui` | **new** |

---

That's all for 0.4 — file issues on the tracker.
"#;

pub fn draw(ui: &mut egui::Ui) {
    Card::new().title("Markdown").show(ui, |ui| {
        let _ = Markdown::new(SAMPLE).show(ui);
    });
}
