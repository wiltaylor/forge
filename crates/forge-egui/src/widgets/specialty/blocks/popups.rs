//! The block palette (slash popup) and emoji completion popup. Both are
//! forced-open [`egui::Popup`]s driven by the focused block's draft; their
//! keyboard handling lives in `text::handle_keys` (keys must be consumed
//! before the `TextEdit` sees them).

use super::{byte_of_char, Action, BlockEditorState, Ecx};
use crate::theme::{FontWeight, Theme};
use egui::text::{CCursor, CCursorRange};
use egui::{CornerRadius, Frame, Margin, Popup, PopupAnchor, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use forge_blocks::{search_emoji, Address, BlockKind, Document, ListStyle, Tone};

pub(super) const EMOJI_LIMIT: usize = 8;

/// What a slash-palette row does when picked.
#[derive(Clone, Debug)]
pub(crate) enum SlashChoice {
    /// Convert the (empty) block to this kind.
    Kind(BlockKind),
    /// Wrap the block into `n` columns.
    Columns(usize),
}

/// Built-ins plus registered custom kinds, filtered by `query` (lowercase).
pub(super) fn slash_choices(
    st: &BlockEditorState,
    in_column: bool,
    query: &str,
) -> Vec<(String, SlashChoice)> {
    let text = |md: &str| md.to_owned();
    let mut all: Vec<(String, SlashChoice)> = vec![
        (
            "Text".into(),
            SlashChoice::Kind(BlockKind::Paragraph { md: text("") }),
        ),
        (
            "Heading 1".into(),
            SlashChoice::Kind(BlockKind::Heading {
                level: 1,
                md: text(""),
            }),
        ),
        (
            "Heading 2".into(),
            SlashChoice::Kind(BlockKind::Heading {
                level: 2,
                md: text(""),
            }),
        ),
        (
            "Heading 3".into(),
            SlashChoice::Kind(BlockKind::Heading {
                level: 3,
                md: text(""),
            }),
        ),
        (
            "Heading 4".into(),
            SlashChoice::Kind(BlockKind::Heading {
                level: 4,
                md: text(""),
            }),
        ),
        (
            "Bullet list".into(),
            SlashChoice::Kind(BlockKind::ListItem {
                style: ListStyle::Bullet,
                checked: None,
                indent: 0,
                md: text(""),
            }),
        ),
        (
            "Numbered list".into(),
            SlashChoice::Kind(BlockKind::ListItem {
                style: ListStyle::Number,
                checked: None,
                indent: 0,
                md: text(""),
            }),
        ),
        (
            "Todo list".into(),
            SlashChoice::Kind(BlockKind::ListItem {
                style: ListStyle::Todo,
                checked: Some(false),
                indent: 0,
                md: text(""),
            }),
        ),
        (
            "Quote".into(),
            SlashChoice::Kind(BlockKind::Quote { md: text("") }),
        ),
        ("Divider".into(), SlashChoice::Kind(BlockKind::Divider)),
        (
            "Code".into(),
            SlashChoice::Kind(BlockKind::Code {
                lang: String::new(),
                code: String::new(),
            }),
        ),
        (
            "Table".into(),
            SlashChoice::Kind(BlockKind::Table {
                header: vec![String::new(); 3],
                rows: vec![vec![String::new(); 3], vec![String::new(); 3]],
            }),
        ),
        (
            "Callout".into(),
            SlashChoice::Kind(BlockKind::Admonition {
                tone: Tone::Info,
                title: String::new(),
                md: String::new(),
            }),
        ),
    ];
    if !in_column {
        all.push(("2 columns".into(), SlashChoice::Columns(2)));
        all.push(("3 columns".into(), SlashChoice::Columns(3)));
    }
    for custom in &st.custom {
        all.push((
            custom.label().to_owned(),
            SlashChoice::Kind(BlockKind::Custom {
                kind: custom.kind().to_owned(),
                data: custom.default_data(),
            }),
        ));
    }
    all.retain(|(label, _)| query.is_empty() || label.to_lowercase().contains(query));
    all
}

fn popup_frame(t: &Theme) -> Frame {
    Frame::new()
        .fill(t.bg[4])
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(4))
}

fn popup_row(ui: &mut Ui, t: &Theme, label: &str, highlighted: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), t.control.sm),
        Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        if response.hovered() || highlighted {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(t.radius.sm as u8), t.bg[2]);
        }
        let color = if response.hovered() || highlighted {
            t.fg[0]
        } else {
            t.fg[1]
        };
        let g = ui.painter().layout_no_wrap(
            label.to_owned(),
            t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
            color,
        );
        ui.painter().galley(
            Pos2::new(rect.min.x + 8.0, rect.center().y - g.size().y / 2.0),
            g,
            color,
        );
    }
    response
}

/// The block palette, anchored under the focused block. Filter-as-you-type
/// comes from the draft (everything after the leading `/`).
pub(super) fn slash_popup(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    addr: Address,
    id: egui::Id,
    anchor: Rect,
) {
    let open_here = st.slash.as_ref().is_some_and(|s| s.addr == addr);
    if !open_here {
        return;
    }
    if !st.draft.starts_with('/') {
        st.slash = None;
        return;
    }
    let query = st.draft[1..].to_lowercase();
    let choices = slash_choices(st, addr.in_column(), &query);
    let hl = st
        .slash
        .as_ref()
        .map(|s| s.hl.min(choices.len().saturating_sub(1)))
        .unwrap_or(0);
    if let Some(slash) = st.slash.as_mut() {
        slash.hl = hl;
    }
    let t = ecx.t;
    Popup::new(
        id.with("slash-popup"),
        ui.ctx().clone(),
        PopupAnchor::ParentRect(anchor),
        ui.layer_id(),
    )
    .open(true)
    .gap(4.0)
    .frame(popup_frame(t))
    .show(|ui| {
        ui.set_min_width(200.0);
        if choices.is_empty() {
            ui.label(
                egui::RichText::new("No matching block")
                    .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                    .color(t.fg[2]),
            );
            return;
        }
        for (i, (label, choice)) in choices.iter().enumerate() {
            if popup_row(ui, t, label, i == hl).clicked() {
                ecx.actions.push(Action::ApplySlash {
                    addr,
                    choice: choice.clone(),
                });
            }
        }
    });
}

/// `:prefix` before the caret, as `(start_char_of_colon, prefix)`. Requires
/// two or more `[a-z0-9_+-]` chars directly after a `:`.
pub(super) fn emoji_prefix(draft: &str, caret_char: usize) -> Option<(usize, String)> {
    let chars: Vec<char> = draft.chars().collect();
    let caret = caret_char.min(chars.len());
    let mut i = caret;
    while i > 0 {
        let c = chars[i - 1];
        if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '+' | '-') {
            i -= 1;
        } else {
            break;
        }
    }
    if i == 0 || chars[i - 1] != ':' || caret - i < 2 {
        return None;
    }
    Some((i - 1, chars[i..caret].iter().collect()))
}

/// Replace the partial `:prefix` before the caret with `:code:` — storage
/// keeps shortcodes; rendering resolves them. Restores focus and caret.
pub(super) fn complete_emoji(
    ctx: &egui::Context,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
    start_char: usize,
    code: &str,
) {
    let sb = byte_of_char(&st.draft, start_char);
    let cb = byte_of_char(&st.draft, st.caret.char_idx);
    if sb > cb || cb > st.draft.len() {
        return;
    }
    st.draft.replace_range(sb..cb, &format!(":{code}:"));
    if let Some(md) = doc.block_mut(addr).and_then(|b| b.kind.md_mut()) {
        md.clone_from(&st.draft);
        st.changed = true;
    }
    let new_char = start_char + code.chars().count() + 2;
    let mut s = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    s.cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(new_char))));
    s.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
    st.caret.char_idx = new_char;
    st.emoji_hl = 0;
    st.emoji_dismissed = None;
}

/// Emoji completion popup near the caret: `search_emoji(prefix, 8)` rows,
/// Enter/Tab (or click) completes the shortcode text.
pub(super) fn emoji_popup(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    if ecx.read_only || st.slash.is_some() {
        return;
    }
    let Some((start, prefix)) = emoji_prefix(&st.draft, st.caret.char_idx) else {
        st.emoji_hl = 0;
        return;
    };
    if st.emoji_dismissed.as_deref() == Some(prefix.as_str()) {
        return;
    }
    st.emoji_dismissed = None;
    let hits = search_emoji(&prefix, EMOJI_LIMIT);
    if hits.is_empty() {
        return;
    }
    let hl = st.emoji_hl.min(hits.len() - 1);
    st.emoji_hl = hl;
    let t = ecx.t;
    let anchor = st.caret.pos + Vec2::new(0.0, 4.0);
    let mut picked: Option<&'static str> = None;
    Popup::new(
        id.with("emoji-popup"),
        ui.ctx().clone(),
        PopupAnchor::Position(anchor),
        ui.layer_id(),
    )
    .open(true)
    .gap(2.0)
    .frame(popup_frame(t))
    .show(|ui| {
        ui.set_min_width(160.0);
        for (i, (code, emoji)) in hits.iter().copied().enumerate() {
            if popup_row(ui, t, &format!("{emoji}  :{code}:"), i == hl).clicked() {
                picked = Some(code);
            }
        }
    });
    if let Some(code) = picked {
        complete_emoji(ui.ctx(), st, doc, addr, id, start, code);
    }
}
