# egui grammar

<!-- Added for wayfinder #67 to the shape #73 settled: Rust needs no class analogue, so
     this page is naming and construction grammar, not a vocabulary list. Transcribed
     from crates/forge-egui/src/{lib.rs,response.rs} and the widget modules. -->

Read this once a session, before the first egui control.

## Naming

A control's type name is its catalogue name in PascalCase, one to one — and the hyphen is
the only word break. `combobox` is one word, so it is `Combobox`, **not** `ComboBox`.
`icon-button` has a hyphen, so it is `IconButton`; `date-picker` is `DatePicker`. No
prefix. Never invent a name for a control the catalogue does not list.

A control with real internal state pairs with a `<Control>State` struct the app owns.
Value-bound form controls do not: they borrow the app's data directly, as
`Input::new(&mut self.text)`.

## Construction

One shape everywhere — a builder struct, then `.show(ui)`:

```rust
Button::new("Save")
    .variant(Variant::Primary)
    .disabled(false)
    .small(false)
    .show(ui)
```

Setters are bare names taking and returning `self`. `.show(ui)` is the terminator and it
returns a `ForgeResponse`, never a bare `egui::Response`.

## Interaction

`ForgeResponse` carries egui's own response plus an `Outcome`:

```rust
pub enum Outcome {
    Ignored,    // no interaction this frame
    Consumed,   // interacted — hovered popup, focus moved, opened — no value change
    Changed,    // the value or selection changed
    Submitted,  // Enter-style commit, or a button activation
    Cancelled,  // Esc-style dismissal — dropdown closed, dialog cancelled
}
```

The same five variants as ratatui and the same meanings, deliberately. `is_handled()` is
anything but `Ignored`; `merge()` takes the more significant of two, for a control built
out of parts.

## Theme

`Theme::apply` installs the theme on the `egui::Context` once, at startup. After that a
widget reads it from the context rather than taking it as an argument — the opposite of
ratatui. Fields are in `tokens.md`, and egui is the platform where the geometry column is
full: `theme.radius`, `theme.space.x(n)`, `theme.control.md`, `theme.type_scale`,
`theme.motion`.

Text goes through `theme.font(ctx, weight, size)` or `theme.mono(size)`. Weights are
`FontWeight::{Regular, Medium, SemiBold}` — there is no bold.

Widgets work in any eframe app once the theme is applied. The runtime — `run()`, `Shell`,
toasts, dialogs — is optional and never required by a control.

## What egui does not give you

egui is immediate-mode: there is no retained widget tree, so focus, hover, keyboard
routing, popup layering and scroll-into-view are all yours, recomputed every frame. Paint
is explicit — you allocate a rect, then draw fill, stroke and text into it. This is why an
egui implementation page runs several times the length of its SolidJS sibling for the same
control.

Focus is a 2px accent ring you draw, as `laws.md` says.
