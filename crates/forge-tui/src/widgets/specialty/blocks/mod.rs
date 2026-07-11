//! Block page editor (cargo feature `blocks`): a Notion-style editor over
//! the shared `forge-blocks` document model — the terminal sibling of the
//! web and egui editors, driven by the same [`forge_blocks::ops`] keyboard
//! model.
//!
//! Focus is two-level: a block is *selected* (accent tint, structural keys)
//! or *entered* (text caret in the raw markdown source, code buffer, table
//! cell, or a registered [`CustomBlock`]). `/` on an empty text block opens
//! the slash palette; `:xx` opens emoji autocomplete; markdown line-start
//! shortcuts (`# `, `- `, "```rust", `---`, `:::info`, …) convert as you
//! type.
//!
//! v1 scope notes, kept simple and predictable: admonition focus edits the
//! *body*; the tone cycles with Ctrl+T and the title stays as authored
//! (editable via the document API). Table columns add/remove with
//! Alt+= / Alt+-; rows grow with Enter on the last row or Ctrl+Enter.

mod popups;
mod render;
mod wrap_edit;

use std::collections::HashMap;

use forge_blocks::{
    flatten_addresses, indent_list, insert_after, line_start_shortcut, merge_with_previous,
    move_block, next_address, prev_address, remove, set_kind, split, table_insert_col,
    table_insert_row, table_remove_col, wrap_in_columns, Address, BlockKind, Document, ListStyle,
    Tone,
};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

use crate::event::{in_area, is_press, left_down, scroll_delta, Outcome};
use crate::theme::{default_theme, Severity, Theme};
use crate::widgets::forms::TextareaState;

use popups::{builtin_kind, emoji_query, slash_commands, EmojiState, Popup, SlashState};
use render::{layout, text_prefix_width, Hit, Painter, GUTTER};
use wrap_edit::WrapEdit;

/// A consumer-defined block implementation. Registered kinds render inside
/// the editor, join the slash palette, and receive keys while entered
/// (Enter on the selected block; Esc leaves).
pub trait CustomBlock {
    /// The `BlockKind::Custom { kind, .. }` discriminator this handles.
    fn kind(&self) -> &'static str;
    /// Slash palette label.
    fn label(&self) -> &'static str;
    /// Payload for freshly inserted blocks.
    fn default_data(&self) -> serde_json::Value;
    /// Rows needed at `width` cells.
    fn height(&self, data: &serde_json::Value, width: u16, t: &Theme) -> u16;
    fn render(
        &mut self,
        data: &serde_json::Value,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        t: &Theme,
    );
    fn handle_key(&mut self, _data: &mut serde_json::Value, _key: KeyEvent) -> Outcome {
        Outcome::Ignored
    }
    fn handle_mouse(
        &mut self,
        _data: &mut serde_json::Value,
        _ev: &MouseEvent,
        _area: Rect,
    ) -> Outcome {
        Outcome::Ignored
    }
}

/// What the focused block is being edited with.
pub(crate) enum Editing {
    /// Block-selected (or no focus at all).
    None,
    /// Raw markdown source of a text kind.
    Text(WrapEdit),
    /// Code block buffer (no soft wrap, like CodeView).
    Code(TextareaState),
    /// One table cell; committed into the table on every edit.
    Cell(WrapEdit),
}

pub(crate) fn tone_severity(tone: Tone) -> Severity {
    match tone {
        Tone::Info => Severity::Info,
        Tone::Success => Severity::Success,
        Tone::Warning => Severity::Warning,
        Tone::Danger => Severity::Danger,
    }
}

fn index_of(addr: Address) -> usize {
    match addr {
        Address::Root(i) => i,
        Address::Cell { idx, .. } => idx,
    }
}

fn sibling(doc: &Document, addr: Address, idx: usize) -> Option<&forge_blocks::Block> {
    match addr {
        Address::Root(_) => doc.blocks.get(idx),
        Address::Cell { root, col, .. } => match &doc.blocks.get(root)?.kind {
            BlockKind::Columns { columns } => columns.get(col)?.blocks.get(idx),
            _ => None,
        },
    }
}

/// 1-based ordinal of a numbered list item among its contiguous same-indent
/// numbered predecessors.
pub(crate) fn list_ordinal(doc: &Document, addr: Address) -> usize {
    let Some(forge_blocks::Block {
        kind:
            BlockKind::ListItem {
                style: ListStyle::Number,
                indent,
                ..
            },
        ..
    }) = doc.block(addr)
    else {
        return 1;
    };
    let my_indent = *indent;
    let mut n = 1;
    let mut i = index_of(addr);
    while i > 0 {
        i -= 1;
        match sibling(doc, addr, i) {
            Some(forge_blocks::Block {
                kind:
                    BlockKind::ListItem {
                        style: ListStyle::Number,
                        indent,
                        ..
                    },
                ..
            }) if *indent == my_indent => n += 1,
            _ => break,
        }
    }
    n
}

fn table_dims(doc: &Document, addr: Address) -> Option<(usize, usize)> {
    match &doc.block(addr)?.kind {
        BlockKind::Table { header, rows } => Some((header.len().max(1), rows.len())),
        _ => None,
    }
}

/// Cell text at display row `r` (0 = header, body rows start at 1).
fn cell_text(doc: &Document, addr: Address, r: usize, c: usize) -> String {
    match doc.block(addr).map(|b| &b.kind) {
        Some(BlockKind::Table { header, rows }) => {
            let cell = if r == 0 {
                header.get(c)
            } else {
                rows.get(r - 1).and_then(|row| row.get(c))
            };
            cell.cloned().unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn set_cell(doc: &mut Document, addr: Address, r: usize, c: usize, s: &str) {
    if let Some(BlockKind::Table { header, rows }) = doc.block_mut(addr).map(|b| &mut b.kind) {
        let cell = if r == 0 {
            header.get_mut(c)
        } else {
            rows.get_mut(r - 1).and_then(|row| row.get_mut(c))
        };
        if let Some(cell) = cell {
            if cell != s {
                *cell = s.to_string();
            }
        }
    }
}

/// Editor state: the document plus focus, editing mode, scroll, popups, and
/// the custom-block registry. All keyboard/mouse handling lives here; the
/// [`BlockEditor`] widget only paints.
pub struct BlockEditorState {
    pub doc: Document,
    focus: Option<Address>,
    editing: Editing,
    custom_active: bool,
    /// Focused table cell as (display row, col); row 0 is the header.
    table_cell: Option<(usize, usize)>,
    popup: Popup,
    custom: Vec<Box<dyn CustomBlock>>,
    scroll: usize,
    total: usize,
    view_h: usize,
    area: Rect,
    read_only: bool,
    hits: Vec<Hit>,
    code_cache: render::CodeCache,
}

impl BlockEditorState {
    pub fn new(doc: Document) -> BlockEditorState {
        let focus = flatten_addresses(&doc).first().copied();
        BlockEditorState {
            doc,
            focus,
            editing: Editing::None,
            custom_active: false,
            table_cell: None,
            popup: Popup::None,
            custom: Vec::new(),
            scroll: 0,
            total: 0,
            view_h: 0,
            area: Rect::default(),
            read_only: false,
            hits: Vec::new(),
            code_cache: HashMap::new(),
        }
    }

    /// Register a custom block implementation; it renders matching
    /// `BlockKind::Custom` blocks and appears in the slash palette.
    pub fn register_custom(&mut self, block: Box<dyn CustomBlock>) {
        self.custom.push(block);
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    pub fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    pub fn focus(&self) -> Option<Address> {
        self.focus
    }

    pub fn is_editing(&self) -> bool {
        !matches!(self.editing, Editing::None) || self.custom_active
    }

    /// Byte offset of the text caret, when a text block or table cell is
    /// being edited.
    pub fn caret(&self) -> Option<usize> {
        match &self.editing {
            Editing::Text(we) | Editing::Cell(we) => Some(we.cursor()),
            _ => None,
        }
    }

    pub fn popup_open(&self) -> bool {
        !matches!(self.popup, Popup::None)
    }

    /// Select `addr` in block mode (no text caret).
    pub fn select(&mut self, addr: Address) {
        self.focus = Some(addr);
        self.editing = Editing::None;
        self.custom_active = false;
        self.table_cell = None;
        self.popup = Popup::None;
    }

    /// Enter the block at `addr` for editing: text kinds take a caret at
    /// `caret` (clamped), code blocks open the code buffer, tables enter
    /// cell (0, 0), registered custom blocks take key focus. Returns false
    /// when the block only supports selection (divider, columns container,
    /// unregistered custom).
    pub fn edit(&mut self, addr: Address, caret: usize) -> bool {
        let Some(block) = self.doc.block(addr) else {
            return false;
        };
        self.focus = Some(addr);
        self.custom_active = false;
        self.table_cell = None;
        self.popup = Popup::None;
        match &block.kind {
            BlockKind::Code { code, .. } => {
                self.editing = Editing::Code(TextareaState::with_value(code));
                true
            }
            BlockKind::Table { .. } => {
                self.table_cell = Some((0, 0));
                self.editing =
                    Editing::Cell(WrapEdit::new(cell_text(&self.doc, addr, 0, 0), usize::MAX));
                true
            }
            BlockKind::Custom { kind, .. } => {
                let registered = self.custom.iter().any(|c| c.kind() == kind.as_str());
                self.custom_active = registered;
                self.editing = Editing::None;
                registered
            }
            kind if kind.is_text() => {
                let md = kind.md().unwrap_or_default().to_string();
                self.editing = Editing::Text(WrapEdit::new(md, caret));
                true
            }
            _ => {
                self.editing = Editing::None;
                false
            }
        }
    }

    /// Paste into the focused editor (the terminal `Event::Paste` entry
    /// point).
    pub fn paste(&mut self, s: &str) -> Outcome {
        if self.read_only {
            return Outcome::Ignored;
        }
        match &mut self.editing {
            Editing::Text(we) | Editing::Cell(we) => {
                we.insert(s);
                self.sync_source();
                self.refresh_emoji();
                Outcome::Changed
            }
            Editing::Code(ts) => {
                ts.insert_str(s);
                self.sync_source();
                Outcome::Changed
            }
            Editing::None => Outcome::Ignored,
        }
    }

    /* ---------------- keyboard ------------------------------------------ */

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        if self.read_only {
            return self.scroll_key(key);
        }
        match self.popup {
            Popup::Slash(_) => return self.slash_key(key),
            Popup::Emoji(_) => {
                let out = self.emoji_key(key);
                if out.is_handled() {
                    return out;
                }
            }
            Popup::None => {}
        }
        if self.custom_active {
            return self.custom_key(key);
        }
        match self.editing {
            Editing::Text(_) => self.text_key(key),
            Editing::Code(_) => self.code_key(key),
            Editing::Cell(_) => self.cell_key(key),
            Editing::None => self.select_key(key),
        }
    }

    fn scroll_key(&mut self, key: KeyEvent) -> Outcome {
        let max = self.total.saturating_sub(self.view_h);
        let page = self.view_h.max(1);
        match key.code {
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll = (self.scroll + 1).min(max),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
            KeyCode::PageDown => self.scroll = (self.scroll + page).min(max),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = max,
            _ => return Outcome::Ignored,
        }
        Outcome::Consumed
    }

    /* ---------------- popup keys ---------------------------------------- */

    fn slash_key(&mut self, key: KeyEvent) -> Outcome {
        let commands = slash_commands(&self.custom);
        let Popup::Slash(sl) = &mut self.popup else {
            return Outcome::Ignored;
        };
        match sl.palette.handle_key(key, &commands) {
            Outcome::Submitted => {
                let ci = sl.palette.highlighted();
                self.popup = Popup::None;
                match ci {
                    Some(ci) => self.apply_slash(ci),
                    None => Outcome::Consumed,
                }
            }
            Outcome::Cancelled => {
                self.popup = Popup::None;
                Outcome::Consumed
            }
            _ => Outcome::Consumed, // the palette is modal while open
        }
    }

    fn open_slash(&mut self) {
        let commands = slash_commands(&self.custom);
        let mut palette = crate::widgets::overlays::PaletteState::new();
        palette.filter(&commands);
        self.popup = Popup::Slash(SlashState { palette, offset: 0 });
    }

    /// Apply a chosen slash command: replace the block when it is an empty
    /// text block, insert after it otherwise; column commands wrap instead.
    fn apply_slash(&mut self, ci: usize) -> Outcome {
        let Some(addr) = self.focus else {
            return Outcome::Consumed;
        };
        let n_builtin = popups::BUILTINS.len();
        let kind = if ci < n_builtin {
            let id = popups::BUILTINS[ci].0;
            if let Some(n) = match id {
                "col2" => Some(2),
                "col3" => Some(3),
                _ => None,
            } {
                return match wrap_in_columns(&mut self.doc, addr, n) {
                    Some(f) => {
                        self.enter_block(f);
                        Outcome::Changed
                    }
                    None => Outcome::Consumed,
                };
            }
            match builtin_kind(id) {
                Some(k) => k,
                None => return Outcome::Consumed,
            }
        } else {
            match self.custom.get(ci - n_builtin) {
                Some(c) => BlockKind::Custom {
                    kind: c.kind().to_string(),
                    data: c.default_data(),
                },
                None => return Outcome::Consumed,
            }
        };
        let replace = self
            .doc
            .block(addr)
            .is_some_and(|b| b.kind.md().is_some_and(|m| m.is_empty()));
        let target = if replace {
            if !set_kind(&mut self.doc, addr, kind) {
                return Outcome::Consumed;
            }
            addr
        } else {
            match insert_after(&mut self.doc, addr, kind) {
                Some(a) => a,
                None => return Outcome::Consumed,
            }
        };
        self.enter_block(target);
        Outcome::Changed
    }

    /// Focus `addr` and enter its natural editing mode (falls back to block
    /// selection for dividers/columns/unregistered customs).
    fn enter_block(&mut self, addr: Address) {
        if !self.edit(addr, usize::MAX) {
            self.select(addr);
        }
    }

    fn emoji_key(&mut self, key: KeyEvent) -> Outcome {
        let Popup::Emoji(em) = &mut self.popup else {
            return Outcome::Ignored;
        };
        match key.code {
            KeyCode::Up => {
                em.sel = em.sel.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                em.sel = (em.sel + 1).min(em.items.len().saturating_sub(1));
                Outcome::Consumed
            }
            KeyCode::Enter | KeyCode::Tab => {
                let (code, start) = match em.items.get(em.sel) {
                    Some((code, _)) => (*code, em.start),
                    None => {
                        self.popup = Popup::None;
                        return Outcome::Consumed;
                    }
                };
                if let Editing::Text(we) | Editing::Cell(we) = &mut self.editing {
                    let end = we.cursor();
                    we.replace_range(start, end, &format!(":{code}:"));
                }
                self.popup = Popup::None;
                self.sync_source();
                Outcome::Changed
            }
            KeyCode::Esc => {
                self.popup = Popup::None;
                Outcome::Consumed
            }
            // Anything else routes to the editor; the trigger re-evaluates
            // after the edit.
            _ => Outcome::Ignored,
        }
    }

    /// Open/refresh/close the emoji popup from the active editor text.
    fn refresh_emoji(&mut self) {
        let query = match &self.editing {
            Editing::Text(we) | Editing::Cell(we) => {
                emoji_query(we.src(), we.cursor()).map(|(s, q)| (s, q.to_string()))
            }
            _ => None,
        };
        match query {
            Some((start, q)) => {
                let items = forge_blocks::search_emoji(&q, 8);
                if items.is_empty() {
                    self.popup = Popup::None;
                } else {
                    let sel = match &self.popup {
                        Popup::Emoji(prev) => prev.sel.min(items.len() - 1),
                        _ => 0,
                    };
                    self.popup = Popup::Emoji(EmojiState { items, sel, start });
                }
            }
            None => {
                if matches!(self.popup, Popup::Emoji(_)) {
                    self.popup = Popup::None;
                }
            }
        }
    }

    /* ---------------- text editing -------------------------------------- */

    /// Copy the active editor buffer back into the document (write-through
    /// after every edit, so ops always see current source).
    fn sync_source(&mut self) {
        let Some(addr) = self.focus else { return };
        match &self.editing {
            Editing::Text(we) => {
                if let Some(md) = self.doc.block_mut(addr).and_then(|b| b.kind.md_mut()) {
                    if md != we.src() {
                        *md = we.src().to_string();
                    }
                }
            }
            Editing::Cell(we) => {
                if let Some((r, c)) = self.table_cell {
                    set_cell(&mut self.doc, addr, r, c, we.src());
                }
            }
            Editing::Code(ts) => {
                if let Some(BlockKind::Code { code, .. }) =
                    self.doc.block_mut(addr).map(|b| &mut b.kind)
                {
                    let v = ts.value();
                    if *code != v {
                        *code = v;
                    }
                }
            }
            Editing::None => {}
        }
    }

    fn active_width(&self) -> usize {
        match &self.editing {
            Editing::Text(we) | Editing::Cell(we) => we.width(),
            _ => 0,
        }
    }

    /// Move the text caret focus to the previous/next block in navigation
    /// order, skipping dividers; non-text landings fall back to selection.
    fn focus_vertical(&mut self, dir: i32, desired: usize) -> Outcome {
        let Some(start) = self.focus else {
            return Outcome::Consumed;
        };
        let width = self.active_width();
        let mut cur = start;
        loop {
            let next = if dir < 0 {
                prev_address(&self.doc, cur)
            } else {
                next_address(&self.doc, cur)
            };
            let Some(a) = next else {
                return Outcome::Consumed; // document edge: stay put
            };
            cur = a;
            let Some(block) = self.doc.block(a) else {
                return Outcome::Consumed;
            };
            match &block.kind {
                BlockKind::Divider => continue,
                kind if kind.is_text() => {
                    let md = kind.md().unwrap_or_default().to_string();
                    let mut we = WrapEdit::new(md, 0);
                    we.set_width(width);
                    let row = if dir < 0 {
                        we.rows().saturating_sub(1)
                    } else {
                        0
                    };
                    we.move_to(row, desired);
                    self.focus = Some(a);
                    self.editing = Editing::Text(we);
                    self.popup = Popup::None;
                    return Outcome::Consumed;
                }
                _ => {
                    self.select(a);
                    return Outcome::Consumed;
                }
            }
        }
    }

    fn cycle_tone(&mut self) -> Outcome {
        let Some(addr) = self.focus else {
            return Outcome::Ignored;
        };
        if let Some(BlockKind::Admonition { tone, .. }) =
            self.doc.block_mut(addr).map(|b| &mut b.kind)
        {
            *tone = match tone {
                Tone::Info => Tone::Success,
                Tone::Success => Tone::Warning,
                Tone::Warning => Tone::Danger,
                Tone::Danger => Tone::Info,
            };
            return Outcome::Changed;
        }
        Outcome::Ignored
    }

    fn text_key(&mut self, key: KeyEvent) -> Outcome {
        let Some(addr) = self.focus else {
            return Outcome::Ignored;
        };
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Esc => {
                self.editing = Editing::None;
                self.popup = Popup::None;
                Outcome::Consumed
            }
            KeyCode::Enter if shift || alt => {
                if let Editing::Text(we) = &mut self.editing {
                    we.insert("\n");
                }
                self.sync_source();
                self.popup = Popup::None;
                Outcome::Changed
            }
            KeyCode::Enter => {
                self.popup = Popup::None;
                let caret = self.caret().unwrap_or(0);
                match split(&mut self.doc, addr, caret) {
                    Some(next) => {
                        let width = self.active_width();
                        let md = self
                            .doc
                            .block(next)
                            .and_then(|b| b.kind.md())
                            .unwrap_or_default()
                            .to_string();
                        let mut we = WrapEdit::new(md, 0);
                        we.set_width(width);
                        self.focus = Some(next);
                        self.editing = Editing::Text(we);
                        Outcome::Changed
                    }
                    None => Outcome::Consumed,
                }
            }
            KeyCode::Backspace => {
                let deleted = match &mut self.editing {
                    Editing::Text(we) => we.backspace(),
                    _ => false,
                };
                if deleted {
                    self.sync_source();
                    self.refresh_emoji();
                    return Outcome::Changed;
                }
                // Caret at byte 0: demote non-paragraph text kinds first,
                // then merge paragraphs into the previous block.
                let kind_is_paragraph = matches!(
                    self.doc.block(addr).map(|b| &b.kind),
                    Some(BlockKind::Paragraph { .. })
                );
                if !kind_is_paragraph {
                    let md = match &self.editing {
                        Editing::Text(we) => we.src().to_string(),
                        _ => String::new(),
                    };
                    set_kind(&mut self.doc, addr, BlockKind::Paragraph { md });
                    return Outcome::Changed;
                }
                match merge_with_previous(&mut self.doc, addr) {
                    Some(res) => {
                        let width = self.active_width();
                        let md = self
                            .doc
                            .block(res.focus)
                            .and_then(|b| b.kind.md())
                            .unwrap_or_default()
                            .to_string();
                        let mut we = WrapEdit::new(md, res.caret);
                        we.set_width(width);
                        self.focus = Some(res.focus);
                        self.editing = Editing::Text(we);
                        Outcome::Changed
                    }
                    None => Outcome::Consumed,
                }
            }
            KeyCode::Delete => {
                let deleted = match &mut self.editing {
                    Editing::Text(we) => we.delete(),
                    _ => false,
                };
                if deleted {
                    self.sync_source();
                    self.refresh_emoji();
                    return Outcome::Changed;
                }
                // Caret at the end: pull the next paragraph in. The merge
                // only fires when the navigation-order next block really is
                // our next sibling (merge target = its previous sibling).
                let Some(next) = next_address(&self.doc, addr) else {
                    return Outcome::Consumed;
                };
                match merge_with_previous(&mut self.doc, next) {
                    Some(res) => {
                        let width = self.active_width();
                        let md = self
                            .doc
                            .block(res.focus)
                            .and_then(|b| b.kind.md())
                            .unwrap_or_default()
                            .to_string();
                        let mut we = WrapEdit::new(md, res.caret);
                        we.set_width(width);
                        self.focus = Some(res.focus);
                        self.editing = Editing::Text(we);
                        Outcome::Changed
                    }
                    None => Outcome::Consumed,
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                let delta = if key.code == KeyCode::BackTab || shift {
                    -1
                } else {
                    1
                };
                if indent_list(&mut self.doc, addr, delta) {
                    Outcome::Changed
                } else if matches!(
                    self.doc.block(addr).map(|b| &b.kind),
                    Some(BlockKind::ListItem { .. })
                ) {
                    Outcome::Consumed
                } else {
                    Outcome::Ignored
                }
            }
            KeyCode::Up | KeyCode::Down if alt => {
                let dir = if key.code == KeyCode::Up { -1 } else { 1 };
                match move_block(&mut self.doc, addr, dir) {
                    Some(a) => {
                        self.focus = Some(a);
                        Outcome::Changed
                    }
                    None => Outcome::Consumed,
                }
            }
            KeyCode::Up | KeyCode::Down => {
                let up = key.code == KeyCode::Up;
                let (moved, desired) = match &mut self.editing {
                    Editing::Text(we) => {
                        let moved = if up { we.up() } else { we.down() };
                        (moved, we.desired())
                    }
                    _ => (false, 0),
                };
                if moved {
                    Outcome::Consumed
                } else {
                    self.focus_vertical(if up { -1 } else { 1 }, desired)
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End => {
                if let Editing::Text(we) = &mut self.editing {
                    match key.code {
                        KeyCode::Left => we.left(),
                        KeyCode::Right => we.right(),
                        KeyCode::Home => we.home(),
                        _ => we.end(),
                    }
                }
                Outcome::Consumed
            }
            KeyCode::Char('t') if ctrl => self.cycle_tone(),
            KeyCode::Char(c) if !ctrl && !alt => {
                let empty = match &self.editing {
                    Editing::Text(we) => we.src().is_empty(),
                    _ => false,
                };
                if c == '/' && empty {
                    self.open_slash();
                    return Outcome::Consumed;
                }
                if let Editing::Text(we) = &mut self.editing {
                    let mut b = [0u8; 4];
                    we.insert(c.encode_utf8(&mut b));
                }
                self.sync_source();
                if self.apply_shortcut(addr) {
                    return Outcome::Changed;
                }
                self.refresh_emoji();
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }

    /// Run the markdown line-start shortcut on a paragraph after an edit.
    /// True when the block converted (the caret pulls back by the prefix).
    fn apply_shortcut(&mut self, addr: Address) -> bool {
        if !matches!(
            self.doc.block(addr).map(|b| &b.kind),
            Some(BlockKind::Paragraph { .. })
        ) {
            return false;
        }
        let (src, cursor, width) = match &self.editing {
            Editing::Text(we) => (we.src().to_string(), we.cursor(), we.width()),
            _ => return false,
        };
        let Some(sc) = line_start_shortcut(&src) else {
            return false;
        };
        let caret = cursor.saturating_sub(sc.prefix_len);
        if !set_kind(&mut self.doc, addr, sc.kind) {
            return false;
        }
        self.popup = Popup::None;
        match self.doc.block(addr).map(|b| &b.kind) {
            Some(BlockKind::Code { code, .. }) => {
                self.editing = Editing::Code(TextareaState::with_value(code));
            }
            Some(BlockKind::Divider) => self.editing = Editing::None,
            Some(kind) if kind.is_text() => {
                let mut we = WrapEdit::new(kind.md().unwrap_or_default().to_string(), caret);
                we.set_width(width);
                self.editing = Editing::Text(we);
            }
            _ => self.editing = Editing::None,
        }
        true
    }

    /* ---------------- code editing -------------------------------------- */

    fn code_key(&mut self, key: KeyEvent) -> Outcome {
        let Some(addr) = self.focus else {
            return Outcome::Ignored;
        };
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if key.code == KeyCode::Esc {
            self.editing = Editing::None;
            return Outcome::Consumed;
        }
        if alt && matches!(key.code, KeyCode::Up | KeyCode::Down) {
            let dir = if key.code == KeyCode::Up { -1 } else { 1 };
            return match move_block(&mut self.doc, addr, dir) {
                Some(a) => {
                    self.focus = Some(a);
                    Outcome::Changed
                }
                None => Outcome::Consumed,
            };
        }
        // Leave the buffer across the first/last line.
        if let Editing::Code(ts) = &self.editing {
            let (row, _) = ts.cursor();
            if key.code == KeyCode::Up && row == 0 {
                return self.focus_vertical(-1, 0);
            }
            if key.code == KeyCode::Down && row + 1 >= ts.line_count() {
                return self.focus_vertical(1, 0);
            }
        }
        let out = match &mut self.editing {
            Editing::Code(ts) => ts.handle_key(key),
            _ => Outcome::Ignored,
        };
        match out {
            Outcome::Changed => {
                self.sync_source();
                Outcome::Changed
            }
            Outcome::Submitted | Outcome::Cancelled => {
                self.editing = Editing::None;
                Outcome::Consumed
            }
            other => other,
        }
    }

    /* ---------------- table cell editing -------------------------------- */

    fn load_cell(&mut self, addr: Address, r: usize, c: usize) {
        self.table_cell = Some((r, c));
        self.editing = Editing::Cell(WrapEdit::new(cell_text(&self.doc, addr, r, c), usize::MAX));
        self.popup = Popup::None;
    }

    fn cell_key(&mut self, key: KeyEvent) -> Outcome {
        let Some(addr) = self.focus else {
            return Outcome::Ignored;
        };
        let Some((r, c)) = self.table_cell else {
            return Outcome::Ignored;
        };
        let Some((ncols, nrows)) = table_dims(&self.doc, addr) else {
            return Outcome::Ignored;
        };
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                self.sync_source();
                self.table_cell = None;
                self.editing = Editing::None;
                self.popup = Popup::None;
                Outcome::Consumed
            }
            KeyCode::Tab => {
                self.sync_source();
                let (nr, nc) = if c + 1 < ncols {
                    (r, c + 1)
                } else if r < nrows {
                    (r + 1, 0)
                } else {
                    (0, 0)
                };
                self.load_cell(addr, nr, nc);
                Outcome::Consumed
            }
            KeyCode::BackTab => {
                self.sync_source();
                let (nr, nc) = if c > 0 {
                    (r, c - 1)
                } else if r > 0 {
                    (r - 1, ncols - 1)
                } else {
                    (nrows, ncols - 1)
                };
                self.load_cell(addr, nr, nc);
                Outcome::Consumed
            }
            KeyCode::Up if !alt => {
                self.sync_source();
                self.load_cell(addr, r.saturating_sub(1), c);
                Outcome::Consumed
            }
            KeyCode::Down if !alt => {
                self.sync_source();
                self.load_cell(addr, (r + 1).min(nrows), c);
                Outcome::Consumed
            }
            KeyCode::Enter if ctrl => {
                let at = if r == 0 { 0 } else { r };
                if table_insert_row(&mut self.doc, addr, at) {
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Enter => {
                self.sync_source();
                if r < nrows {
                    self.load_cell(addr, r + 1, c);
                    Outcome::Consumed
                } else if table_insert_row(&mut self.doc, addr, nrows) {
                    self.load_cell(addr, r + 1, c);
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Char('=') if alt => {
                if table_insert_col(&mut self.doc, addr, c + 1) {
                    self.sync_source();
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Char('-') if alt => {
                if table_remove_col(&mut self.doc, addr, c) {
                    let (ncols, _) = table_dims(&self.doc, addr).unwrap_or((1, 0));
                    self.load_cell(addr, r, c.min(ncols - 1));
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Backspace => {
                let deleted = match &mut self.editing {
                    Editing::Cell(we) => we.backspace(),
                    _ => false,
                };
                self.sync_source();
                self.refresh_emoji();
                if deleted {
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Delete => {
                let deleted = match &mut self.editing {
                    Editing::Cell(we) => we.delete(),
                    _ => false,
                };
                self.sync_source();
                if deleted {
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End => {
                if let Editing::Cell(we) = &mut self.editing {
                    match key.code {
                        KeyCode::Left => we.left(),
                        KeyCode::Right => we.right(),
                        KeyCode::Home => we.home(),
                        _ => we.end(),
                    }
                }
                Outcome::Consumed
            }
            KeyCode::Char(ch) if !ctrl && !alt => {
                if let Editing::Cell(we) = &mut self.editing {
                    let mut b = [0u8; 4];
                    we.insert(ch.encode_utf8(&mut b));
                }
                self.sync_source();
                self.refresh_emoji();
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }

    /* ---------------- custom blocks -------------------------------------- */

    fn custom_key(&mut self, key: KeyEvent) -> Outcome {
        if key.code == KeyCode::Esc {
            self.custom_active = false;
            return Outcome::Consumed;
        }
        let Some(addr) = self.focus else {
            return Outcome::Ignored;
        };
        let Some(BlockKind::Custom { kind, data }) = self.doc.block_mut(addr).map(|b| &mut b.kind)
        else {
            self.custom_active = false;
            return Outcome::Ignored;
        };
        let kind = kind.clone();
        match self.custom.iter_mut().find(|c| c.kind() == kind.as_str()) {
            Some(imp) => imp.handle_key(data, key),
            None => Outcome::Ignored,
        }
    }

    /* ---------------- block selection ------------------------------------ */

    fn select_step(&mut self, dir: i32) {
        let flat = flatten_addresses(&self.doc);
        if flat.is_empty() {
            return;
        }
        let Some(cur) = self.focus else {
            self.focus = flat.first().copied();
            return;
        };
        let next = match flat.iter().position(|a| *a == cur) {
            Some(p) => {
                let np = p as i64 + dir as i64;
                if np < 0 || np as usize >= flat.len() {
                    return;
                }
                flat[np as usize]
            }
            None => {
                // A columns container: step to the block just outside it.
                let root = cur.root();
                let pos = if dir < 0 {
                    flat.iter().rposition(|a| a.root() < root)
                } else {
                    flat.iter().position(|a| a.root() > root)
                };
                match pos {
                    Some(p) => flat[p],
                    None => return,
                }
            }
        };
        self.focus = Some(next);
    }

    fn select_key(&mut self, key: KeyEvent) -> Outcome {
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if self.focus.is_none() {
            return match key.code {
                KeyCode::Up | KeyCode::Down | KeyCode::Enter => {
                    self.focus = flatten_addresses(&self.doc).first().copied();
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            };
        }
        let addr = self.focus.expect("checked above");
        match key.code {
            KeyCode::Up | KeyCode::Down if alt => {
                let dir = if key.code == KeyCode::Up { -1 } else { 1 };
                match move_block(&mut self.doc, addr, dir) {
                    Some(a) => {
                        self.focus = Some(a);
                        Outcome::Changed
                    }
                    None => Outcome::Consumed,
                }
            }
            KeyCode::Up => {
                self.select_step(-1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.select_step(1);
                Outcome::Consumed
            }
            KeyCode::Enter => {
                let _ = self.edit(addr, usize::MAX);
                Outcome::Consumed
            }
            KeyCode::Delete | KeyCode::Backspace => match remove(&mut self.doc, addr) {
                Some(a) => {
                    self.focus = Some(a);
                    Outcome::Changed
                }
                None => Outcome::Consumed,
            },
            KeyCode::Char('/') => {
                self.open_slash();
                Outcome::Consumed
            }
            KeyCode::Char('c') if !ctrl && !alt => match wrap_in_columns(&mut self.doc, addr, 2) {
                Some(f) => {
                    self.select(f);
                    Outcome::Changed
                }
                None => Outcome::Consumed,
            },
            KeyCode::Char('t') if ctrl => self.cycle_tone(),
            KeyCode::Esc => match addr {
                Address::Cell { root, .. } => {
                    self.focus = Some(Address::Root(root));
                    Outcome::Consumed
                }
                Address::Root(_) => {
                    self.focus = None;
                    Outcome::Cancelled
                }
            },
            _ => Outcome::Ignored,
        }
    }

    /* ---------------- mouse ---------------------------------------------- */

    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta != 0 {
            if !in_area(ev, self.area) {
                return Outcome::Ignored;
            }
            let max = self.total.saturating_sub(self.view_h);
            self.scroll = if delta < 0 {
                self.scroll.saturating_sub(3)
            } else {
                (self.scroll + 3).min(max)
            };
            return Outcome::Consumed;
        }
        if !left_down(ev) || !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        if self.read_only {
            return Outcome::Ignored;
        }
        if self.popup_open() {
            self.popup = Popup::None;
            return Outcome::Consumed;
        }
        let hit = self
            .hits
            .iter()
            .find(|h| !h.container && in_area(ev, h.rect))
            .or_else(|| self.hits.iter().find(|h| in_area(ev, h.rect)))
            .copied();
        let Some(hit) = hit else {
            return Outcome::Consumed;
        };
        if hit.container {
            self.select(hit.addr);
            return Outcome::Consumed;
        }
        let row = (ev.row - hit.rect.y) as usize + hit.top_skip as usize;
        let col = (ev.column - hit.rect.x) as usize;
        let Some(block) = self.doc.block(hit.addr) else {
            return Outcome::Consumed;
        };
        match &block.kind {
            BlockKind::ListItem {
                style: ListStyle::Todo,
                indent,
                ..
            } if row == 0 && {
                let mx = GUTTER as usize + (*indent as usize) * 2;
                (mx..mx + 3).contains(&col)
            } =>
            {
                // Click on the "[ ]" marker toggles the checkbox.
                if let Some(BlockKind::ListItem { checked, .. }) =
                    self.doc.block_mut(hit.addr).map(|b| &mut b.kind)
                {
                    *checked = Some(!checked.unwrap_or(false));
                }
                Outcome::Changed
            }
            kind if kind.is_text() => {
                let prefix = text_prefix_width(&self.doc, hit.addr, kind) as usize;
                let body_row_off = usize::from(matches!(kind, BlockKind::Admonition { .. }));
                let content_w = (hit.rect.width as usize)
                    .saturating_sub(GUTTER as usize + prefix)
                    .max(1);
                let md = kind.md().unwrap_or_default().to_string();
                let mut we = WrapEdit::new(md, 0);
                we.set_width(content_w);
                we.move_to(
                    row.saturating_sub(body_row_off),
                    col.saturating_sub(GUTTER as usize + prefix),
                );
                self.focus = Some(hit.addr);
                self.editing = Editing::Text(we);
                self.custom_active = false;
                self.table_cell = None;
                Outcome::Consumed
            }
            BlockKind::Code { .. } => {
                self.enter_block(hit.addr);
                Outcome::Consumed
            }
            BlockKind::Custom { kind, data } => {
                let kind = kind.clone();
                let data = data.clone();
                let handled = self
                    .custom
                    .iter_mut()
                    .find(|c| c.kind() == kind.as_str())
                    .map(|imp| {
                        let mut d = data;
                        let out = imp.handle_mouse(&mut d, ev, hit.rect);
                        (out, d)
                    });
                if let Some((out, d)) = handled {
                    if out.is_handled() {
                        if let Some(BlockKind::Custom { data, .. }) =
                            self.doc.block_mut(hit.addr).map(|b| &mut b.kind)
                        {
                            *data = d;
                        }
                        self.focus = Some(hit.addr);
                        return out;
                    }
                }
                self.select(hit.addr);
                Outcome::Consumed
            }
            _ => {
                self.select(hit.addr);
                Outcome::Consumed
            }
        }
    }
}

/// The block editor widget. Standard kit contract: builder args only; every
/// interaction lives in [`BlockEditorState`].
#[derive(Clone, Debug, Default)]
pub struct BlockEditor<'a> {
    theme: Option<&'a Theme>,
    read_only: bool,
    focused: bool,
}

impl<'a> BlockEditor<'a> {
    pub fn new() -> BlockEditor<'a> {
        BlockEditor::default()
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Render-only mode: selection, carets, and editing keys are disabled;
    /// scroll keys and the wheel still work.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'a> StatefulWidget for BlockEditor<'a> {
    type State = BlockEditorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut BlockEditorState) {
        state.area = area;
        state.view_h = area.height as usize;
        state.read_only = self.read_only;
        state.hits.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());

        let selecting =
            !self.read_only && !state.custom_active && matches!(state.editing, Editing::None);
        let container_focus = if selecting {
            state.focus.filter(|a| {
                matches!(a, Address::Root(_))
                    && matches!(
                        state.doc.block(*a).map(|b| &b.kind),
                        Some(BlockKind::Columns { .. })
                    )
            })
        } else {
            None
        };

        let mut painter = Painter {
            doc: &state.doc,
            focus: if self.read_only { None } else { state.focus },
            editing: &mut state.editing,
            table_cell: state.table_cell,
            custom_active: state.custom_active,
            custom: &mut state.custom,
            code_cache: &mut state.code_cache,
            read_only: self.read_only,
            widget_focused: self.focused,
            t,
        };
        let (slots, total) = layout(&mut painter, area.width);
        state.total = total;
        let view_h = area.height as usize;
        let max_scroll = total.saturating_sub(view_h);

        // Keep the focused block in view (skipped read-only so manual
        // scrolling wins).
        if !self.read_only {
            if let Some(f) = state.focus {
                let slot = slots
                    .iter()
                    .find(|s| !s.container && s.addr == f)
                    .or_else(|| slots.iter().find(|s| s.container && s.addr == f));
                if let Some(slot) = slot {
                    if slot.y < state.scroll {
                        state.scroll = slot.y;
                    } else if slot.y + slot.h > state.scroll + view_h {
                        state.scroll = (slot.y + slot.h).saturating_sub(view_h);
                    }
                }
            }
        }
        state.scroll = state.scroll.min(max_scroll);
        let scroll = state.scroll;

        let mut hits: Vec<Hit> = Vec::new();
        for slot in &slots {
            let vis_top = slot.y.max(scroll);
            let vis_bot = (slot.y + slot.h).min(scroll + view_h);
            if vis_top >= vis_bot {
                continue;
            }
            let top_skip = (vis_top - slot.y) as u16;
            let dst_y = area.y + (vis_top - scroll) as u16;
            let rect = Rect::new(area.x + slot.x, dst_y, slot.w, (vis_bot - vis_top) as u16);
            if slot.container {
                hits.push(Hit {
                    addr: slot.addr,
                    container: true,
                    rect,
                    top_skip,
                });
                continue;
            }
            let Some(block) = painter.doc.block(slot.addr) else {
                continue;
            };
            let mut scratch = Buffer::empty(Rect::new(0, 0, slot.w, slot.h as u16));
            let container_selected =
                container_focus == Some(Address::Root(slot.addr.root())) && slot.addr.in_column();
            render::paint_block(
                &mut painter,
                block,
                slot.addr,
                &mut scratch,
                slot.w,
                slot.h as u16,
                container_selected,
            );
            for ry in 0..rect.height {
                for rx in 0..rect.width {
                    buf[(rect.x + rx, rect.y + ry)] = scratch[(rx, top_skip + ry)].clone();
                }
            }
            hits.push(Hit {
                addr: slot.addr,
                container: false,
                rect,
                top_skip,
            });
        }
        state.hits = hits;

        // Popups render last, anchored near the focused block.
        if !matches!(state.popup, Popup::None) {
            let anchor = state
                .hits
                .iter()
                .find(|h| !h.container && Some(h.addr) == state.focus)
                .map(|h| h.rect)
                .unwrap_or(Rect::new(area.x, area.y, 1, 1));
            match &mut state.popup {
                Popup::Slash(sl) => {
                    let commands = slash_commands(&state.custom);
                    popups::render_slash(sl, &commands, anchor, area, buf, t);
                }
                Popup::Emoji(em) => popups::render_emoji(em, anchor, area, buf, t),
                Popup::None => {}
            }
        }
    }
}
