//! The text-block path: list/quote markers, the unfocused inline-markdown
//! label, and the focused frameless `TextEdit` with the shared keyboard
//! model (Enter splits, Backspace-at-0 merges, boundary arrows hop blocks,
//! Tab indents lists, `/` opens the palette, `:pre` completes emoji).

use super::inline::{inline_job, text_style, InlineStyle};
use super::{byte_of_char, popups, siblings, Action, BlockEditorState, CaretHint, Ecx};
use crate::theme::FontWeight;
use egui::text::{CCursor, CCursorRange};
use egui::{Key, Modifiers, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use forge_blocks::{line_start_shortcut, Address, BlockKind, Document, ListStyle};

/// A plain text block row: indent + marker + body.
pub(super) fn text_row(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let Some(block) = doc.block(addr) else { return };
    let style = text_style(ecx.t, &block.kind);
    match block.kind.clone() {
        BlockKind::ListItem {
            style: list_style,
            checked,
            indent,
            ..
        } => {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.add_space(indent as f32 * 18.0);
                list_marker(ui, ecx, st, doc, addr, list_style, checked);
                text_body(ui, ecx, st, doc, addr, id, style);
            });
        }
        BlockKind::Quote { .. } => {
            let inner = ui.horizontal_top(|ui| {
                ui.add_space(12.0);
                ui.vertical(|ui| text_body(ui, ecx, st, doc, addr, id, style));
            });
            let rect = inner.response.rect;
            ui.painter().rect_filled(
                Rect::from_min_max(
                    Pos2::new(rect.min.x + 2.0, rect.min.y + 2.0),
                    Pos2::new(rect.min.x + 5.0, rect.max.y - 2.0),
                ),
                egui::CornerRadius::same(1),
                ecx.t.border.strong,
            );
        }
        _ => text_body(ui, ecx, st, doc, addr, id, style),
    }
}

/// Bullet dot, ordinal, or todo checkbox — the checkbox toggles in place.
fn list_marker(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    style: ListStyle,
    checked: Option<bool>,
) {
    let t = ecx.t;
    match style {
        ListStyle::Bullet => {
            ui.label(
                egui::RichText::new("•")
                    .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                    .color(t.fg[2]),
            );
        }
        ListStyle::Number => {
            ui.label(
                egui::RichText::new(format!("{}.", list_ordinal(doc, addr)))
                    .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                    .color(t.fg[2]),
            );
        }
        ListStyle::Todo => {
            let done = checked.unwrap_or(false);
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::splat(15.0),
                if ecx.read_only {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            );
            let rect = rect.shrink(1.0);
            if done {
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(3), t.accent.base);
                let g = ui.painter().layout_no_wrap(
                    "✓".to_owned(),
                    t.mono(t.type_scale.xs),
                    t.accent.contrast,
                );
                let pos = rect.center() - g.size() / 2.0;
                ui.painter().galley(pos, g, t.accent.contrast);
            } else {
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(3),
                    Stroke::new(1.0, t.border.strong),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                if let Some(BlockKind::ListItem { checked, .. }) =
                    doc.block_mut(addr).map(|b| &mut b.kind)
                {
                    *checked = Some(!done);
                    st.changed = true;
                }
            }
        }
    }
}

/// 1-based ordinal of a numbered list item: consecutive `Number` siblings at
/// the same indent above it (deeper indents in between don't break the run).
fn list_ordinal(doc: &Document, addr: Address) -> usize {
    let (list, idx) = siblings(doc, addr);
    let indent = match list.get(idx).map(|b| &b.kind) {
        Some(BlockKind::ListItem { indent, .. }) => *indent,
        _ => return 1,
    };
    let mut n = 1;
    let mut j = idx;
    while j > 0 {
        match &list[j - 1].kind {
            BlockKind::ListItem {
                style: ListStyle::Number,
                indent: i,
                ..
            } if *i == indent => {
                n += 1;
                j -= 1;
            }
            BlockKind::ListItem { indent: i, .. } if *i > indent => j -= 1,
            _ => break,
        }
    }
    n
}

/// The markdown body of a text block: focused → frameless `TextEdit` over
/// the raw source; otherwise a click-to-focus styled label.
pub(super) fn text_body(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
    style: InlineStyle,
) {
    let editing_here = st.focus == Some(addr) && st.editing && !ecx.read_only;
    if editing_here {
        edit_body(ui, ecx, st, doc, addr, id, style);
        return;
    }
    let md = doc
        .block(addr)
        .and_then(|b| b.kind.md())
        .unwrap_or("")
        .to_owned();
    let job = inline_job(ui, ecx.t, &md, style, ui.available_width());
    let sense = if ecx.read_only {
        Sense::hover()
    } else {
        Sense::click()
    };
    let resp = ui.add(egui::Label::new(job).sense(sense));
    if resp.clicked() {
        ecx.actions.push(Action::Focus(addr, CaretHint::End));
    }
}

/// Everything the focused text block does in one frame, in order: key
/// interception (using last frame's caret), the `TextEdit`, draft handling
/// (shortcuts / commit), caret cache refresh, pending-focus caret placement,
/// the accent rail, and the slash/emoji popups.
fn edit_body(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
    style: InlineStyle,
) {
    let ctx = ui.ctx().clone();
    let t = ecx.t;
    let kind_now = doc.block(addr).map(|b| b.kind.clone());
    let is_list = matches!(kind_now, Some(BlockKind::ListItem { .. }));
    let is_paragraph = matches!(kind_now, Some(BlockKind::Paragraph { .. }));

    if ctx.memory(|m| m.has_focus(id)) {
        handle_keys(ui, ecx, st, doc, addr, id, is_list);
    }

    let mut draft = std::mem::take(&mut st.draft);
    let hint = if is_paragraph && draft.is_empty() {
        "Type '/' for commands"
    } else {
        ""
    };
    let out = egui::TextEdit::multiline(&mut draft)
        .id(id)
        .frame(egui::Frame::NONE)
        .font(t.font(ui.ctx(), style.weight, style.size))
        .text_color(style.color)
        .hint_text(egui::RichText::new(hint).color(t.fg[3]))
        .desired_rows(1)
        .desired_width(f32::INFINITY)
        .lock_focus(true)
        .margin(egui::Margin::ZERO)
        .show(ui);

    if out.response.changed() {
        // "/" typed into an empty paragraph opens the palette; the slash
        // stays in the draft and the tail becomes the filter query. The doc
        // still holds the pre-change source here (commit happens below), so
        // "was empty" is read from it.
        let was_empty = doc
            .block(addr)
            .and_then(|b| b.kind.md())
            .is_some_and(str::is_empty);
        if st.slash.is_none() && is_paragraph && was_empty && draft.starts_with('/') {
            st.slash = Some(super::SlashState { addr, hl: 0 });
        }
        let shortcut = if is_paragraph && st.slash.is_none() {
            line_start_shortcut(&draft)
        } else {
            None
        };
        match shortcut {
            Some(hit) => {
                let caret_char = out
                    .state
                    .cursor
                    .char_range()
                    .map(|r| r.primary.index.0)
                    .unwrap_or(draft.chars().count());
                let caret = byte_of_char(&draft, caret_char).saturating_sub(hit.prefix_len);
                ecx.actions.push(Action::Shortcut {
                    addr,
                    kind: hit.kind,
                    caret,
                });
            }
            None => {
                if let Some(md) = doc.block_mut(addr).and_then(|b| b.kind.md_mut()) {
                    if *md != draft {
                        *md = draft.clone();
                        st.changed = true;
                    }
                }
            }
        }
    }

    // Refresh the caret cache from this frame's galley (rows, caret row,
    // screen x/pos) — next frame's key interception reads it.
    if let Some(range) = out.state.cursor.char_range() {
        let chars = draft.chars().count();
        let char_idx = range.primary.index.0.min(chars);
        let rect = out.galley.pos_from_cursor(CCursor::new(char_idx));
        let mid = rect.center().y;
        let rows = out.galley.rows.len().max(1);
        let row = out
            .galley
            .rows
            .iter()
            .position(|r| mid <= r.max_y() + 0.1)
            .unwrap_or(rows - 1);
        st.caret = super::CaretCache {
            char_idx,
            has_selection: range.primary.index != range.secondary.index,
            row,
            rows,
            x: out.galley_pos.x + rect.left(),
            pos: Pos2::new(
                out.galley_pos.x + rect.left(),
                out.galley_pos.y + rect.bottom(),
            ),
        };
    }

    // Place the caret for a freshly focused block.
    if let Some((paddr, hint)) = st.pending_focus {
        if paddr == addr {
            let chars = draft.chars().count();
            let char_idx = match hint {
                CaretHint::Start => 0,
                CaretHint::End => chars,
                CaretHint::Byte(b) => super::char_of_byte(&draft, b),
                CaretHint::Col(x) => {
                    let row = if st.from_below {
                        out.galley.rows.last()
                    } else {
                        out.galley.rows.first()
                    };
                    let y = row.map(|r| r.rect().center().y).unwrap_or(0.0);
                    out.galley
                        .cursor_from_pos(Vec2::new(x - out.galley_pos.x, y))
                        .index
                        .0
                }
            }
            .min(chars);
            let mut s = out.state.clone();
            s.cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(char_idx))));
            s.store(&ctx, id);
            ctx.memory_mut(|m| m.request_focus(id));
            st.pending_focus = None;
            st.caret.char_idx = char_idx;
        }
    }

    if out.response.has_focus() {
        // Keep Tab/arrows/Escape flowing into the widget so this editor —
        // not egui's focus traversal — decides what they mean.
        ctx.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            );
        });
    } else if out.response.lost_focus() && st.pending_focus.is_none() {
        // Clicked away: drop to block-selection.
        st.editing = false;
        st.slash = None;
    }

    // Accent rail along the focused block.
    let rect = out.response.rect;
    ui.painter().rect_filled(
        Rect::from_min_max(
            Pos2::new(rect.min.x - 8.0, rect.min.y + 1.0),
            Pos2::new(rect.min.x - 5.5, rect.max.y - 1.0),
        ),
        egui::CornerRadius::same(1),
        t.accent.base,
    );

    st.draft = draft;
    popups::slash_popup(ui, ecx, st, addr, id, rect);
    popups::emoji_popup(ui, ecx, st, doc, addr, id);
}

/// The per-key booleans we may consume this frame. Consumption is
/// conditional: boundary arrows only at the galley's first/last row,
/// Backspace only with the caret at 0, and popups take nav keys first.
fn handle_keys(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
    is_list: bool,
) {
    let slash_open = st.slash.as_ref().is_some_and(|s| s.addr == addr);
    let emoji = if slash_open {
        None
    } else {
        popups::emoji_prefix(&st.draft, st.caret.char_idx)
            .filter(|(_, p)| st.emoji_dismissed.as_deref() != Some(p.as_str()))
    };
    let popup = slash_open || emoji.is_some();
    let at_start = st.caret.char_idx == 0 && !st.caret.has_selection;
    let on_first = st.caret.row == 0;
    let on_last = st.caret.row + 1 >= st.caret.rows.max(1);

    struct Keys {
        enter: bool,
        tab: bool,
        shift_tab: bool,
        alt_up: bool,
        alt_down: bool,
        up: bool,
        down: bool,
        backspace: bool,
        esc: bool,
    }
    let keys = ui.ctx().input_mut(|i| Keys {
        alt_up: i.consume_key(Modifiers::ALT, Key::ArrowUp),
        alt_down: i.consume_key(Modifiers::ALT, Key::ArrowDown),
        enter: i.consume_key(Modifiers::NONE, Key::Enter),
        tab: i.consume_key(Modifiers::NONE, Key::Tab),
        shift_tab: i.consume_key(Modifiers::SHIFT, Key::Tab),
        up: (popup || on_first) && i.consume_key(Modifiers::NONE, Key::ArrowUp),
        down: (popup || on_last) && i.consume_key(Modifiers::NONE, Key::ArrowDown),
        backspace: at_start && !popup && i.consume_key(Modifiers::NONE, Key::Backspace),
        esc: i.consume_key(Modifiers::NONE, Key::Escape),
    });

    if slash_open {
        let query = st.draft.strip_prefix('/').unwrap_or("").to_lowercase();
        let n = popups::slash_choices(st, addr.in_column(), &query).len();
        if let Some(slash) = st.slash.as_mut() {
            if keys.down && n > 0 {
                slash.hl = (slash.hl + 1).min(n - 1);
            }
            if keys.up {
                slash.hl = slash.hl.saturating_sub(1);
            }
        }
        if keys.enter {
            let hl = st.slash.as_ref().map(|s| s.hl).unwrap_or(0);
            let mut choices = popups::slash_choices(st, addr.in_column(), &query);
            if hl < choices.len() {
                let (_, choice) = choices.swap_remove(hl);
                ecx.actions.push(Action::ApplySlash { addr, choice });
            }
        }
        if keys.esc {
            st.slash = None;
        }
        return;
    }

    if let Some((start, prefix)) = emoji {
        let hits = forge_blocks::search_emoji(&prefix, popups::EMOJI_LIMIT);
        if !hits.is_empty() {
            if keys.down {
                st.emoji_hl = (st.emoji_hl + 1).min(hits.len() - 1);
            }
            if keys.up {
                st.emoji_hl = st.emoji_hl.saturating_sub(1);
            }
            if keys.enter || keys.tab {
                let (code, _) = hits[st.emoji_hl.min(hits.len() - 1)];
                popups::complete_emoji(ui.ctx(), st, doc, addr, id, start, code);
                return;
            }
        }
        if keys.esc {
            st.emoji_dismissed = Some(prefix);
            return;
        }
        // Fall through: Backspace/arrows behave normally while typing a code.
    }

    if keys.enter {
        ecx.actions.push(Action::Split(addr));
    } else if keys.backspace {
        ecx.actions.push(Action::BackspaceAt0(addr));
    } else if keys.up {
        ecx.actions.push(Action::NavPrev {
            addr,
            x: Some(st.caret.x),
        });
    } else if keys.down {
        ecx.actions.push(Action::NavNext {
            addr,
            x: Some(st.caret.x),
        });
    } else if keys.alt_up {
        ecx.actions.push(Action::MoveBlock { addr, dir: -1 });
    } else if keys.alt_down {
        ecx.actions.push(Action::MoveBlock { addr, dir: 1 });
    } else if keys.tab && is_list {
        ecx.actions.push(Action::Indent { addr, delta: 1 });
    } else if keys.shift_tab && is_list {
        ecx.actions.push(Action::Indent { addr, delta: -1 });
    } else if keys.esc {
        st.editing = false;
        ui.ctx().memory_mut(|m| m.surrender_focus(id));
    }
}
