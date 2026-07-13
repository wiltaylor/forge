//! Glyph-content snapshots via a bare Buffer (layout/content tier) plus
//! targeted style assertions (theming tier). Full style dumps are avoided —
//! too churn-prone.

use forge_tui::theme::Theme;
use forge_tui::widgets::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{StatefulWidget, Widget};

fn buffer_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in area.y..area.y + area.height {
        let mut line = String::new();
        for x in area.x..area.x + area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn render<W: Widget>(widget: W, w: u16, h: u16) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    widget.render(Rect::new(0, 0, w, h), &mut buf);
    buf
}

#[test]
fn button_variants() {
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
    Button::new("Save")
        .variant(Variant::Primary)
        .theme(&t)
        .render(Rect::new(0, 0, 8, 1), &mut buf);
    Button::new("Cancel")
        .theme(&t)
        .render(Rect::new(10, 0, 10, 1), &mut buf);
    Button::new("Ghost")
        .variant(Variant::Ghost)
        .theme(&t)
        .render(Rect::new(22, 0, 9, 1), &mut buf);
    insta::assert_snapshot!(buffer_text(&buf));
    // Primary buttons paint solid accent with contrast text.
    assert_eq!(buf[(2, 0)].style().bg, Some(t.accent.base));
    assert_eq!(buf[(2, 0)].style().fg, Some(t.accent.contrast));
}

#[test]
fn bordered_button_focus_ring() {
    let t = Theme::dark();
    let buf = render(Button::new("Deploy").focused(true).theme(&t), 12, 3);
    insta::assert_snapshot!(buffer_text(&buf));
    // Focused border is accent.
    assert_eq!(buf[(0, 0)].style().fg, Some(t.accent.base));
}

#[test]
fn badge_and_kbd_and_status() {
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 44, 1));
    Badge::new("ready")
        .severity(forge_tui::theme::Severity::Success)
        .theme(&t)
        .render(Rect::new(0, 0, 8, 1), &mut buf);
    Kbd::new("⌃K")
        .theme(&t)
        .render(Rect::new(9, 0, 5, 1), &mut buf);
    StatusDot::new(forge_tui::theme::Severity::Danger)
        .label("down")
        .theme(&t)
        .render(Rect::new(16, 0, 8, 1), &mut buf);
    insta::assert_snapshot!(buffer_text(&buf));
    assert_eq!(buf[(1, 0)].style().fg, Some(t.success.fg));
    assert_eq!(buf[(1, 0)].style().bg, Some(t.success.bg));
    assert_eq!(buf[(16, 0)].style().fg, Some(t.danger.base));
}

#[test]
fn card_stat_and_alert() {
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 8));
    let card = Card::new().title(" Metrics ").theme(&t);
    let inner = card.inner(Rect::new(0, 0, 20, 5));
    card.render(Rect::new(0, 0, 20, 5), &mut buf);
    Stat::new("requests", "48.2k")
        .delta("12%", Trend::Up)
        .theme(&t)
        .render(inner, &mut buf);
    Alert::new(forge_tui::theme::Severity::Warning, "Degraded")
        .body("Event bus lagging.")
        .theme(&t)
        .render(Rect::new(21, 0, 19, 3), &mut buf);
    insta::assert_snapshot!(buffer_text(&buf));
}

#[test]
fn progress_fill_math() {
    let t = Theme::dark();
    let buf = render(Progress::new(0.5).show_percent(false).theme(&t), 10, 1);
    let cells: String = (0..10)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<Vec<_>>()
        .join("");
    assert_eq!(cells, "█████     ");
}

#[test]
fn input_renders_value_cursor_and_placeholder() {
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 24, 1));
    let mut state = InputState::with_value("hello");
    Input::new()
        .focused(true)
        .theme(&t)
        .render(Rect::new(0, 0, 12, 1), &mut buf, &mut state);
    let mut empty = InputState::new();
    Input::new().placeholder("Search…").theme(&t).render(
        Rect::new(13, 0, 11, 1),
        &mut buf,
        &mut empty,
    );
    insta::assert_snapshot!(buffer_text(&buf));
    // Focused edge bar is accent; placeholder is disabled-tone.
    assert_eq!(buf[(0, 0)].style().fg, Some(t.accent.base));
    assert_eq!(buf[(14, 0)].style().fg, Some(t.fg[3]));
}

#[test]
fn input_scrolls_to_keep_cursor_visible() {
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let mut state = InputState::with_value("abcdefghijklmnop");
    Input::new()
        .focused(true)
        .theme(&t)
        .render(Rect::new(0, 0, 10, 1), &mut buf, &mut state);
    // Cursor is at the end; the visible tail must include the last chars.
    let visible: String = (1..9)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        visible.contains("op"),
        "viewport did not follow cursor: {visible:?}"
    );
}

#[test]
fn empty_and_skeleton_render_without_panic_at_tiny_sizes() {
    let t = Theme::dark();
    for (w, h) in [(1u16, 1u16), (3, 2), (0, 0)] {
        let _ = render(Empty::new("nothing").hint("hint").theme(&t), w, h);
        let _ = render(Skeleton::new().frame(7).theme(&t), w, h);
        let _ = render(Spinner::new().frame(3).label("x").theme(&t), w, h);
        let _ = render(Progress::new(0.3).theme(&t), w, h);
    }
}
