//! Editor popups: the slash block palette (built on the kit's
//! `PaletteState` fuzzy filter) and the `:shortcode:` emoji autocomplete.
//! Both render last with `Clear` + `place()` anchored near the focused
//! block.

use forge_blocks::{BlockKind, ListStyle, Tone};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::text;
use crate::theme::Theme;
use crate::widgets::forms::Input;
use crate::widgets::overlays::{Command, PaletteState, Popover};

use super::CustomBlock;

/// Open popup, if any. Emoji tracks the byte offset of the `:` that opened
/// it so accepting a completion can replace the partial query.
pub(super) enum Popup {
    None,
    Slash(SlashState),
    Emoji(EmojiState),
}

pub(super) struct SlashState {
    pub palette: PaletteState,
    pub offset: usize,
}

pub(super) struct EmojiState {
    pub items: Vec<(&'static str, &'static str)>,
    pub sel: usize,
    /// Byte offset of the opening `:` in the active editor source.
    pub start: usize,
}

/// Built-in slash entries: (command id, palette label).
pub(super) const BUILTINS: &[(&str, &str)] = &[
    ("text", "Text"),
    ("h1", "Heading 1"),
    ("h2", "Heading 2"),
    ("h3", "Heading 3"),
    ("bullet", "Bullet list"),
    ("number", "Numbered list"),
    ("todo", "To-do list"),
    ("quote", "Quote"),
    ("divider", "Divider"),
    ("code", "Code"),
    ("table", "Table"),
    ("callout", "Callout"),
    ("col2", "2 columns"),
    ("col3", "3 columns"),
];

/// The fresh block kind a built-in command id creates. Columns ids are
/// handled by the caller (they wrap rather than replace).
pub(super) fn builtin_kind(id: &str) -> Option<BlockKind> {
    let kind = match id {
        "text" => BlockKind::Paragraph { md: String::new() },
        "h1" | "h2" | "h3" => BlockKind::Heading {
            level: id.as_bytes()[1] - b'0',
            md: String::new(),
        },
        "bullet" => BlockKind::ListItem {
            style: ListStyle::Bullet,
            checked: None,
            indent: 0,
            md: String::new(),
        },
        "number" => BlockKind::ListItem {
            style: ListStyle::Number,
            checked: None,
            indent: 0,
            md: String::new(),
        },
        "todo" => BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(false),
            indent: 0,
            md: String::new(),
        },
        "quote" => BlockKind::Quote { md: String::new() },
        "divider" => BlockKind::Divider,
        "code" => BlockKind::Code {
            lang: String::new(),
            code: String::new(),
        },
        "table" => BlockKind::Table {
            header: vec![String::new(), String::new()],
            rows: vec![vec![String::new(), String::new()]],
        },
        "callout" => BlockKind::Admonition {
            tone: Tone::Info,
            title: String::new(),
            md: String::new(),
        },
        _ => return None,
    };
    Some(kind)
}

/// The full palette command list: built-ins followed by the registered
/// custom kinds (in registration order).
pub(super) fn slash_commands(custom: &[Box<dyn CustomBlock>]) -> Vec<Command<'static>> {
    let mut out: Vec<Command<'static>> = BUILTINS
        .iter()
        .map(|(id, label)| Command::new(id, label))
        .collect();
    for c in custom {
        out.push(Command::new(c.kind(), c.label()));
    }
    out
}

/// Detect a live emoji query left of the cursor: a `:` followed by at least
/// two shortcode characters. Returns (byte offset of the `:`, the query).
pub(super) fn emoji_query(src: &str, cursor: usize) -> Option<(usize, &str)> {
    let before = &src[..cursor.min(src.len())];
    let start = before.rfind(':')?;
    let query = &before[start + 1..];
    if query.len() < 2 {
        return None;
    }
    query
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '+' | '-'))
        .then_some((start, query))
}

/// Render the slash palette anchored to the focused block.
pub(super) fn render_slash(
    sl: &mut SlashState,
    commands: &[Command],
    anchor: Rect,
    bounds: Rect,
    buf: &mut Buffer,
    t: &Theme,
) {
    let rows = sl.palette.matches().len().clamp(1, 8) as u16;
    let w = 34.min(bounds.width).max(20);
    let h = (rows + 3).min(bounds.height);
    let pop = Popover::new(anchor).size(w, h).theme(t);
    let inner = pop.inner(bounds);
    pop.render(bounds, buf);
    if inner.height < 2 {
        return;
    }
    Input::new()
        .placeholder("Filter blocks…")
        .focused(true)
        .theme(t)
        .render(
            Rect::new(inner.x, inner.y, inner.width, 1),
            buf,
            &mut sl.palette.input,
        );
    let list_h = inner.height.saturating_sub(1) as usize;
    let matches = sl.palette.matches().to_vec();
    let hpos = sl
        .palette
        .highlighted()
        .and_then(|ci| matches.iter().position(|&m| m == ci))
        .unwrap_or(0);
    if hpos < sl.offset {
        sl.offset = hpos;
    } else if hpos >= sl.offset + list_h.max(1) {
        sl.offset = hpos + 1 - list_h.max(1);
    }
    if matches.is_empty() {
        buf.set_string(
            inner.x + 1,
            inner.y + 1,
            "No matching blocks",
            Style::new().fg(t.fg[3]).bg(t.bg[4]),
        );
        return;
    }
    for vis in 0..list_h {
        let mi = sl.offset + vis;
        let Some(&ci) = matches.get(mi) else { break };
        let Some(cmd) = commands.get(ci) else { break };
        let y = inner.y + 1 + vis as u16;
        let mut style = Style::new().fg(t.fg[1]).bg(t.bg[4]);
        if mi == hpos {
            style = Style::new()
                .fg(t.fg[0])
                .bg(t.bg[3])
                .add_modifier(Modifier::BOLD);
            buf.set_style(Rect::new(inner.x, y, inner.width, 1), style);
        }
        buf.set_string(
            inner.x + 1,
            y,
            text::truncate(cmd.label, inner.width.saturating_sub(2) as usize),
            style,
        );
    }
}

/// Render the emoji autocomplete anchored to the focused block.
pub(super) fn render_emoji(
    em: &EmojiState,
    anchor: Rect,
    bounds: Rect,
    buf: &mut Buffer,
    t: &Theme,
) {
    let rows = em.items.len().clamp(1, 8) as u16;
    let label_w = em
        .items
        .iter()
        .map(|(code, e)| text::width(e) + text::width(code) + 4)
        .max()
        .unwrap_or(12);
    let w = ((label_w as u16) + 2).clamp(14, bounds.width);
    let h = (rows + 2).min(bounds.height);
    let pop = Popover::new(anchor).size(w, h).theme(t);
    let inner = pop.inner(bounds);
    pop.render(bounds, buf);
    for (i, (code, emoji)) in em.items.iter().enumerate() {
        let y = inner.y + i as u16;
        if i as u16 >= inner.height {
            break;
        }
        let mut style = Style::new().fg(t.fg[1]).bg(t.bg[4]);
        if i == em.sel {
            style = Style::new()
                .fg(t.fg[0])
                .bg(t.bg[3])
                .add_modifier(Modifier::BOLD);
            buf.set_style(Rect::new(inner.x, y, inner.width, 1), style);
        }
        buf.set_string(
            inner.x + 1,
            y,
            text::truncate(
                &format!("{emoji} :{code}:"),
                inner.width.saturating_sub(2) as usize,
            ),
            style,
        );
    }
}
