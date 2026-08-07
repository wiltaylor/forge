//! button — ratatui.
//!
//! Built from `reference/controls/button.md` and
//! `reference/impl/ratatui/button.md`.
//!
//! ```text
//!   > [ icon label ]        focused: reversed style, plus `>` in the cell before
//!     [ icon label ]        unfocused
//! ```
//!
//! One cell row. Width is the label plus two spaces of padding on each side, and
//! the control never stretches. The focus gutter is always reserved, so nothing
//! shifts when focus moves along a row.

use std::borrow::Cow;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use super::outcome::Outcome;
use super::theme::Theme;

/// Cells given to the focus marker, before the button body.
const GUTTER: u16 = 1;
/// Spaces of padding on each side of the body.
const PAD: u16 = 2;

/// Spinner frames for `loading`. The caller drives the tick; this control owns
/// no timer.
const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Fill and stroke only. Geometry never changes with variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// The accent solid with the on-accent text role. One per screen.
    Primary,
    /// The border colour as a foreground on the surface. Everything else.
    Default,
    /// Plain text. Toolbars, table rows.
    Ghost,
    /// The danger solid. Destructive.
    Danger,
}

impl Default for Variant {
    fn default() -> Self {
        Variant::Default
    }
}

/// There is no `sm` in a terminal — every control is one cell high. The control
/// page names size as platform discretion; this platform accepts the size and
/// ignores it. Asking for `Sm` is a no-op, not an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
}

/// One action, labelled.
///
/// ```ignore
/// Button::new("Deploy")
///     .variant(Variant::Primary)
///     .focused(is_focused)
///     .loading(in_flight)
///     .tick(tick)
///     .theme(&theme)
/// ```
#[derive(Debug, Clone)]
pub struct Button<'a> {
    label: Cow<'a, str>,
    variant: Variant,
    // Stored and never read: see `Size`.
    #[allow(dead_code)]
    size: Size,
    icon: Option<char>,
    trailing_icon: Option<char>,
    focused: bool,
    disabled: bool,
    loading: bool,
    pressed: bool,
    tick: u64,
    theme: Option<&'a Theme>,
}

impl<'a> Button<'a> {
    /// A button carrying `label`. Every visible string is a parameter; nothing
    /// user-facing is hardcoded inside the control.
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            variant: Variant::Default,
            size: Size::Md,
            icon: None,
            trailing_icon: None,
            focused: false,
            disabled: false,
            loading: false,
            pressed: false,
            tick: 0,
            theme: None,
        }
    }

    /// Fill and stroke. Geometry never changes with variant.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Accepted and ignored — a terminal has one row height.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// A glyph before the label. Glyphs *are* the icons here; an icon font is
    /// wrong in a terminal.
    pub fn icon(mut self, icon: char) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A glyph after the label.
    pub fn trailing_icon(mut self, icon: char) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    /// Draws the reversed style and the `>` gutter marker. The caller owns
    /// focus; this control assumes it has focus whenever `handle_key` is called.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Activation is a no-op while disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replaces the leading glyph with the spinner frame for the current tick,
    /// and takes the disabled path. The label stays mounted.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Visible for the duration of the press, not for a fixed animation. The
    /// caller holds it while the key is held.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// The spinner frame counter. The caller drives it.
    pub fn tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    /// The palette. Passed in, never reached for globally.
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Cells this button occupies, including the focus gutter. Buttons do not
    /// stretch, so a row is laid out by summing these.
    pub fn width(&self) -> u16 {
        let label = self.label.chars().count() as u16;
        let leading = if self.leading_glyph().is_some() { 2 } else { 0 };
        let trailing = if self.trailing_icon.is_some() { 2 } else { 0 };
        GUTTER + PAD + leading + label + trailing + PAD
    }

    /// `Outcome::Submitted` on Enter or Space, guarded on key press. Activation
    /// while disabled or loading is a no-op.
    pub fn handle_key(&self, key: KeyEvent) -> Outcome {
        // Windows reports press and release; an unguarded handler fires twice.
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        if self.disabled || self.loading {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => Outcome::Submitted,
            _ => Outcome::Ignored,
        }
    }

    /// True while the control cannot be activated.
    pub fn is_interactive(&self) -> bool {
        !self.disabled && !self.loading
    }

    fn leading_glyph(&self) -> Option<char> {
        if self.loading {
            Some(SPINNER[(self.tick as usize) % SPINNER.len()])
        } else {
            self.icon
        }
    }

    fn body_style(&self, theme: &Theme) -> Style {
        // Loading takes the disabled path.
        if self.disabled || self.loading {
            return Style::new().fg(theme.fg[3]).bg(theme.bg[0]);
        }

        let base = match self.variant {
            Variant::Primary => Style::new().fg(theme.accent.contrast).bg(theme.accent.base),
            Variant::Default => Style::new().fg(theme.border.default).bg(theme.bg[0]),
            Variant::Ghost => Style::new().fg(theme.fg[0]).bg(theme.bg[0]),
            // No `contrast` token exists for a status solid; the surface role is
            // the readable text on it in both themes.
            Variant::Danger => Style::new().fg(theme.bg[0]).bg(theme.danger.base),
        };

        // A colour change alone is unreadable — 256-colour terminals collapse
        // near colours. Focus and press are a reversed style.
        if self.focused || self.pressed {
            base.add_modifier(Modifier::REVERSED)
        } else {
            base
        }
    }

    fn body(&self) -> String {
        let mut body = String::new();
        body.push_str("  ");
        if let Some(glyph) = self.leading_glyph() {
            body.push(glyph);
            body.push(' ');
        }
        body.push_str(&self.label);
        if let Some(glyph) = self.trailing_icon {
            body.push(' ');
            body.push(glyph);
        }
        body.push_str("  ");
        body
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let fallback;
        let theme = match self.theme {
            Some(theme) => theme,
            None => {
                fallback = Theme::dark();
                &fallback
            }
        };

        let y = area.y;

        // The gutter marker is the whole focus indicator when the body cannot
        // reverse, and it is reserved on every button so nothing shifts.
        let marker = if self.focused { ">" } else { " " };
        let marker_style = Style::new().fg(theme.accent.base).bg(theme.bg[0]);
        buf.set_stringn(area.x, y, marker, area.width as usize, marker_style);

        if area.width <= GUTTER {
            return;
        }

        let room = area.width - GUTTER;
        let body = truncate(&self.body(), room as usize);
        buf.set_stringn(area.x + GUTTER, y, body, room as usize, self.body_style(theme));
    }
}

/// Truncate with an ellipsis at the end. ratatui wraps rather than truncating,
/// and a wrapped button breaks the one-row height.
fn truncate(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_string();
    }
    if room == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(room - 1).collect();
    out.push('…');
    out
}
