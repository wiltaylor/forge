# ratatui grammar

<!-- Added for wayfinder #67 to the shape #73 settled: Rust needs no class analogue, so
     this page is naming and construction grammar, not a vocabulary list. Transcribed
     from crates/forge-tui/src/{lib.rs,event.rs} and the widget modules. -->

Read this once a session, before the first ratatui control.

Pinned to **ratatui 0.30**. Import crossterm types through `ratatui::crossterm`, never as
a direct dependency, so the two versions can never diverge.

## Naming

A control's type name is its catalogue name in PascalCase, one to one — and the hyphen is
the only word break. `combobox` is one word, so it is `Combobox`, **not** `ComboBox`.
`list-box` has a hyphen, so it is `ListBox`; `date-picker` is `DatePicker`. No prefix —
the crate is the namespace. Never abbreviate, and never invent a name for a control the
catalogue does not list.

A control with persistent state pairs with a second type named `<Control>State` —
`ComboBoxState`, `TableState`. The app owns the state; the widget is rebuilt each frame.

## Construction

Every control is a plain ratatui `Widget` or `StatefulWidget`, so it drops into any
existing ratatui app. Build it with `new()` and chain setters that take and return `self`:

```rust
Button::new("Save")
    .variant(Variant::Primary)
    .focused(is_focused)
    .disabled(false)
    .theme(&theme)
```

Setters are bare names — `.variant()`, `.focused()`, `.disabled()`, `.theme()` — with no
`with_` or `set_` prefix. A control that can measure itself exposes `.width()`.

## Interaction

One pattern, everywhere. The state type owns `handle_key` and returns an `Outcome`:

```rust
pub enum Outcome {
    Ignored,    // not for this widget — keep routing, like DOM bubbling
    Consumed,   // handled, no observable value change (a cursor move)
    Changed,    // the value or selection changed
    Submitted,  // Enter-style commit — read the value from the state
    Cancelled,  // Esc-style dismissal
}
```

`Outcome::Ignored` is what makes key routing composable: a parent keeps offering the key
to the next handler until something returns anything else. `outcome.is_handled()` is
"anything but `Ignored`".

React to key **presses** only. Windows reports both press and release, and reacting to
release double-triggers every control.

## Theme

`Theme` is passed in, never reached for globally inside a widget — `.theme(&theme)`. Its
fields are in `tokens.md`. The geometry column there is empty on purpose: sizing here is
rows and columns, and a "32px control" is one cell high.

`Theme::quantized(mode)` degrades the palette for 256- and 16-colour terminals. Do not
hand-pick ANSI colours; quantize the real theme.

## What the terminal does not give you

Everything. There is no focus system, no tab order, no scroll-into-view, no accessibility
tree, no hover, no clipping. Each is yours to write, which is why a ratatui implementation
page is several times the length of its SolidJS sibling for the same control.

Focus is drawn, not owned: a reversed cell or a `>` gutter marker, as `laws.md` says.
