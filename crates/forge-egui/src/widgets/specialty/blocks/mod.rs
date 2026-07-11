//! Block page editor (cargo feature `blocks`) — the egui editor for the
//! `forge-blocks` document model, sibling of the web and TUI block editors.
//!
//! [`BlockEditor`] renders a [`Document`] as a vertical list of blocks.
//! Unfocused text blocks show styled inline markdown (via
//! `forge_blocks::parse_inline`); clicking one swaps in a frameless
//! `TextEdit` bound to the raw markdown source. The keyboard model is the
//! shared Forge contract: Enter splits, Backspace-at-0 merges, Tab indents
//! list items, Alt+↑/↓ moves blocks, `/` on an empty block opens the block
//! palette, and `:pre` pops emoji completion. All document mutations go
//! through `forge_blocks::ops`, so every platform edits identically.
//!
//! ```ignore
//! let mut state = BlockEditorState::new(Document::new());
//! let response = BlockEditor::new(&mut state).show(ui);
//! if response.changed() { save(&state.doc); }
//! ```

mod chrome;
mod inline;
mod kinds;
mod popups;
mod text;

use crate::response::{ForgeResponse, Outcome};
use crate::theme::Theme;
use egui::Ui;
use forge_blocks::{
    indent_list, insert_after, merge_with_previous, move_block, next_address, prev_address, remove,
    set_kind, split, table_insert_row, wrap_in_columns, Address, Block, BlockKind, Document,
};

/* ---------------- public API ---------------- */

/// Where the caret should land when focus moves into a text block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaretHint {
    Start,
    End,
    /// A byte offset into the block's markdown source.
    Byte(usize),
    /// A screen-space x coordinate — arrow-key navigation preserves the
    /// caret column across blocks.
    Col(f32),
}

/// A consumer-defined block implementation, registered with
/// [`BlockEditorState::register_custom`]. Unregistered `custom` kinds render
/// as a dashed placeholder.
pub trait CustomBlock {
    /// The `custom` kind string this implementation handles.
    fn kind(&self) -> &'static str;
    /// Human label shown in the slash palette.
    fn label(&self) -> &'static str;
    /// The data a freshly inserted block starts with.
    fn default_data(&self) -> serde_json::Value;
    /// Render the block; return `true` when `data` was mutated.
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut serde_json::Value,
        focused: bool,
        t: &Theme,
    ) -> bool;
}

/// Slash-palette state: open on `addr` with keyboard highlight `hl`. The
/// query is the draft text after the leading `/`.
struct SlashState {
    addr: Address,
    hl: usize,
}

/// Cached caret geometry of the focused text block, taken from the previous
/// frame's `TextEdit` galley — key interception runs *before* the widget.
#[derive(Clone, Copy, Debug, Default)]
struct CaretCache {
    /// Caret position in chars.
    char_idx: usize,
    has_selection: bool,
    /// Wrapped-row index the caret sits on, and the total row count.
    row: usize,
    rows: usize,
    /// Screen-space caret x (column preservation) and baseline position
    /// (emoji popup anchor).
    x: f32,
    pos: egui::Pos2,
}

/// App-owned state for [`BlockEditor`]: the document plus focus, drafts,
/// popup state, and the custom-block registry.
pub struct BlockEditorState {
    /// The document being edited.
    pub doc: Document,
    focus: Option<Address>,
    /// `true` while the focused block is being edited (text caret / table
    /// cells / code body); `false` means block-selection mode.
    editing: bool,
    /// The focused text block's markdown source, bound to its `TextEdit`
    /// and committed into the document on every change.
    draft: String,
    pending_focus: Option<(Address, CaretHint)>,
    /// Whether a pending `Col` hint arrived from below (land on the last
    /// wrapped row) or above (land on the first).
    from_below: bool,
    pending_code: Option<Address>,
    /// Table-cell focus: `(row, col)` with row 0 = header.
    cell: Option<(usize, usize)>,
    pending_cell: Option<(usize, usize)>,
    custom: Vec<Box<dyn CustomBlock>>,
    slash: Option<SlashState>,
    emoji_hl: usize,
    /// Esc-dismissed emoji prefix — the popup stays closed until it changes.
    emoji_dismissed: Option<String>,
    caret: CaretCache,
    changed: bool,
}

impl BlockEditorState {
    pub fn new(doc: Document) -> BlockEditorState {
        BlockEditorState {
            doc,
            focus: None,
            editing: false,
            draft: String::new(),
            pending_focus: None,
            from_below: false,
            pending_code: None,
            cell: None,
            pending_cell: None,
            custom: Vec::new(),
            slash: None,
            emoji_hl: 0,
            emoji_dismissed: None,
            caret: CaretCache::default(),
            changed: false,
        }
    }

    /// Register a custom-block implementation; registered kinds render live
    /// and appear in the slash palette.
    pub fn register_custom(&mut self, block: impl CustomBlock + 'static) {
        self.custom.push(Box::new(block));
    }

    /// The currently focused/selected block, if any.
    pub fn focused(&self) -> Option<Address> {
        self.focus
    }
}

/// Block page editor: `BlockEditor::new(&mut state).show(ui)`. The response
/// reports [`Outcome::Changed`] whenever the document was mutated this frame.
pub struct BlockEditor<'a> {
    state: &'a mut BlockEditorState,
    read_only: bool,
}

impl<'a> BlockEditor<'a> {
    pub fn new(state: &'a mut BlockEditorState) -> BlockEditor<'a> {
        BlockEditor {
            state,
            read_only: false,
        }
    }

    /// Render without focus, chrome, or editing affordances.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let st = self.state;
        st.changed = false;
        let mut doc = std::mem::take(&mut st.doc);
        doc.normalize();

        // Drop stale focus/pending state if the doc changed under us.
        if let Some(addr) = st.focus {
            if doc.block(addr).is_none() {
                st.focus = None;
                st.editing = false;
                st.slash = None;
            }
        }
        if let Some((addr, _)) = st.pending_focus {
            if doc.block(addr).is_none_or(|b| !b.kind.is_text()) {
                st.pending_focus = None;
            }
        }

        let mut ecx = Ecx {
            t: &t,
            read_only: self.read_only,
            actions: Vec::new(),
        };

        if !self.read_only {
            selection_keys(ui, st, &doc, &mut ecx.actions);
        }

        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = t.space.x(1.5);
                for i in 0..doc.blocks.len() {
                    render_root(ui, &mut ecx, st, &mut doc, i);
                }
            })
            .response;

        for action in ecx.actions {
            apply(st, &mut doc, action);
        }
        doc.normalize();
        st.doc = doc;

        let outcome = if st.changed {
            Outcome::Changed
        } else {
            Outcome::Ignored
        };
        ForgeResponse::new(response, outcome)
    }
}

/* ---------------- internal plumbing ---------------- */

/// Per-frame render context threaded through every block renderer.
pub(super) struct Ecx<'a> {
    pub(crate) t: &'a Theme,
    pub(crate) read_only: bool,
    pub(crate) actions: Vec<Action>,
}

/// Deferred document edits — structural ops apply after the walk so block
/// indices stay valid while rendering.
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Focus(Address, CaretHint),
    Select(Address),
    Split(Address),
    BackspaceAt0(Address),
    Shortcut {
        addr: Address,
        kind: BlockKind,
        /// Byte offset the caret should keep after the prefix is stripped.
        caret: usize,
    },
    ApplySlash {
        addr: Address,
        choice: popups::SlashChoice,
    },
    NavPrev {
        addr: Address,
        x: Option<f32>,
    },
    NavNext {
        addr: Address,
        x: Option<f32>,
    },
    Indent {
        addr: Address,
        delta: i8,
    },
    MoveBlock {
        addr: Address,
        dir: i32,
    },
    Duplicate(Address),
    Remove(Address),
    TurnInto {
        addr: Address,
        kind: BlockKind,
    },
    WrapColumns {
        addr: Address,
        n: usize,
    },
    AddColumn {
        root: usize,
    },
    RemoveColumn {
        root: usize,
        col: usize,
    },
    AppendTableRow {
        addr: Address,
        col: usize,
    },
}

fn render_root(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    i: usize,
) {
    let is_columns = matches!(
        doc.blocks.get(i).map(|b| &b.kind),
        Some(BlockKind::Columns { .. })
    );
    if is_columns {
        kinds::columns_block(ui, ecx, st, doc, i);
    } else {
        render_block(ui, ecx, st, doc, Address::Root(i));
    }
}

/// One block row: hover gutter handle + content, plus the selection ring.
pub(super) fn render_block(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
) {
    let Some(block) = doc.block(addr) else { return };
    let id = egui::Id::new(("forge-block", block.id.as_str()));

    if ecx.read_only {
        dispatch_kind(ui, ecx, st, doc, addr, id);
        return;
    }

    let row_top = ui.cursor().top();
    let inner = ui
        .horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            chrome::gutter(ui, ecx, doc, addr, row_top);
            ui.vertical(|ui| dispatch_kind(ui, ecx, st, doc, addr, id));
        })
        .response;

    if st.focus == Some(addr) && !st.editing {
        ui.painter().rect_stroke(
            inner.rect.expand(2.0),
            egui::CornerRadius::same(ecx.t.radius.sm as u8),
            egui::Stroke::new(1.5, ecx.t.accent.base),
            egui::StrokeKind::Outside,
        );
    }
}

fn dispatch_kind(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let Some(block) = doc.block(addr) else { return };
    match &block.kind {
        BlockKind::Paragraph { .. }
        | BlockKind::Heading { .. }
        | BlockKind::ListItem { .. }
        | BlockKind::Quote { .. } => text::text_row(ui, ecx, st, doc, addr, id),
        BlockKind::Admonition { .. } => kinds::admonition(ui, ecx, st, doc, addr, id),
        BlockKind::Divider => kinds::divider(ui, ecx, st, addr),
        BlockKind::Code { .. } => kinds::code_block(ui, ecx, st, doc, addr, id),
        BlockKind::Table { .. } => kinds::table_block(ui, ecx, st, doc, addr, id),
        BlockKind::Custom { .. } => kinds::custom_block(ui, ecx, st, doc, addr, id),
        // Columns are handled at the root level, never as a row.
        BlockKind::Columns { .. } => {}
    }
}

/* ---------------- keyboard: block-selection mode ---------------- */

/// Selection-mode keys: with a block selected but nothing being edited,
/// ↑/↓ move the selection, Enter enters the block, Delete removes it,
/// Alt+↑/↓ moves it, Esc deselects.
fn selection_keys(ui: &Ui, st: &mut BlockEditorState, doc: &Document, actions: &mut Vec<Action>) {
    let Some(addr) = st.focus else { return };
    if st.editing || st.slash.is_some() {
        return;
    }
    // Never steal keys while some widget (ours or the app's) owns focus.
    if ui.ctx().memory(|m| m.focused().is_some()) {
        return;
    }
    use egui::{Key, Modifiers};
    let (alt_up, alt_down, up, down, enter, delete, esc) = ui.ctx().input_mut(|i| {
        (
            i.consume_key(Modifiers::ALT, Key::ArrowUp),
            i.consume_key(Modifiers::ALT, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::ArrowUp),
            i.consume_key(Modifiers::NONE, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::NONE, Key::Delete)
                || i.consume_key(Modifiers::NONE, Key::Backspace),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    if alt_up {
        actions.push(Action::MoveBlock { addr, dir: -1 });
    } else if alt_down {
        actions.push(Action::MoveBlock { addr, dir: 1 });
    } else if up {
        if let Some(prev) = prev_address(doc, addr) {
            actions.push(Action::Select(prev));
        }
    } else if down {
        if let Some(next) = next_address(doc, addr) {
            actions.push(Action::Select(next));
        }
    } else if enter {
        actions.push(Action::Focus(addr, CaretHint::End));
    } else if delete {
        actions.push(Action::Remove(addr));
    } else if esc {
        st.focus = None;
    }
}

/* ---------------- action application ---------------- */

fn apply(st: &mut BlockEditorState, doc: &mut Document, action: Action) {
    match action {
        Action::Focus(addr, hint) => focus_block(st, doc, addr, hint),
        Action::Select(addr) => select_block(st, addr),
        Action::Split(addr) => {
            let caret = doc
                .block(addr)
                .and_then(|b| b.kind.md())
                .map(|md| byte_of_char(md, st.caret.char_idx))
                .unwrap_or(0);
            if let Some(next) = split(doc, addr, caret) {
                st.changed = true;
                focus_block(st, doc, next, CaretHint::Start);
            }
        }
        Action::BackspaceAt0(addr) => {
            let kind = doc.block(addr).map(|b| b.kind.clone());
            match kind {
                Some(BlockKind::Paragraph { .. }) => {
                    if let Some(merge) = merge_with_previous(doc, addr) {
                        st.changed = true;
                        focus_block(st, doc, merge.focus, CaretHint::Byte(merge.caret));
                    }
                }
                // The shared keyboard rule: non-paragraph text kinds first
                // demote to a paragraph (caret stays at 0, same block).
                Some(k) if k.is_text() => {
                    let md = k.md().unwrap_or("").to_owned();
                    if set_kind(doc, addr, BlockKind::Paragraph { md }) {
                        st.changed = true;
                    }
                }
                _ => {}
            }
        }
        Action::Shortcut { addr, kind, caret } => {
            let md = kind.md().map(str::to_owned);
            let to_code = matches!(kind, BlockKind::Code { .. });
            let to_divider = matches!(kind, BlockKind::Divider);
            if set_kind(doc, addr, kind) {
                st.changed = true;
                if to_code {
                    st.focus = Some(addr);
                    st.editing = true;
                    st.pending_code = Some(addr);
                } else if to_divider {
                    select_block(st, addr);
                } else if let Some(md) = md {
                    st.focus = Some(addr);
                    st.editing = true;
                    st.draft = md;
                    st.pending_focus = Some((addr, CaretHint::Byte(caret)));
                    st.from_below = false;
                }
            }
        }
        Action::ApplySlash { addr, choice } => {
            st.slash = None;
            st.draft.clear();
            if let Some(md) = doc.block_mut(addr).and_then(|b| b.kind.md_mut()) {
                md.clear();
            }
            st.changed = true;
            match choice {
                popups::SlashChoice::Columns(n) => {
                    if let Some(cell) = wrap_in_columns(doc, addr, n) {
                        focus_block(st, doc, cell, CaretHint::Start);
                    }
                }
                popups::SlashChoice::Kind(kind) => {
                    if set_kind(doc, addr, kind) {
                        focus_block(st, doc, addr, CaretHint::Start);
                    }
                }
            }
        }
        Action::NavPrev { addr, x } => {
            st.from_below = true;
            nav_to(st, doc, prev_nav_target(doc, addr), x, CaretHint::End);
        }
        Action::NavNext { addr, x } => {
            st.from_below = false;
            nav_to(st, doc, next_nav_target(doc, addr), x, CaretHint::Start);
        }
        Action::Indent { addr, delta } => {
            if indent_list(doc, addr, delta) {
                st.changed = true;
            }
        }
        Action::MoveBlock { addr, dir } => {
            if let Some(next) = move_block(doc, addr, dir) {
                st.changed = true;
                if st.focus == Some(addr) {
                    // TextEdit state keys off the block id, so the caret
                    // survives the move untouched.
                    st.focus = Some(next);
                }
            }
        }
        Action::Duplicate(addr) => {
            if let Some(kind) = doc.block(addr).map(|b| b.kind.clone()) {
                if let Some(next) = insert_after(doc, addr, kind) {
                    st.changed = true;
                    select_block(st, next);
                }
            }
        }
        Action::Remove(addr) => {
            if let Some(next) = remove(doc, addr) {
                st.changed = true;
                select_block(st, next);
            }
        }
        Action::TurnInto { addr, kind } => {
            let to_code = matches!(kind, BlockKind::Code { .. });
            if set_kind(doc, addr, kind) {
                st.changed = true;
                if st.focus == Some(addr) && st.editing {
                    if to_code {
                        st.pending_code = Some(addr);
                    } else if let Some(md) = doc.block(addr).and_then(|b| b.kind.md()) {
                        st.draft = md.to_owned();
                    }
                }
            }
        }
        Action::WrapColumns { addr, n } => {
            if let Some(cell) = wrap_in_columns(doc, addr, n) {
                st.changed = true;
                focus_block(st, doc, cell, CaretHint::End);
            }
        }
        Action::AddColumn { root } => {
            if let Some(col) = forge_blocks::add_column(doc, root) {
                st.changed = true;
                focus_block(
                    st,
                    doc,
                    Address::Cell { root, col, idx: 0 },
                    CaretHint::Start,
                );
            }
        }
        Action::RemoveColumn { root, col } => {
            if let Some(next) = forge_blocks::remove_column(doc, root, col) {
                st.changed = true;
                select_block(st, next);
            }
        }
        Action::AppendTableRow { addr, col } => {
            let rows = match doc.block(addr).map(|b| &b.kind) {
                Some(BlockKind::Table { rows, .. }) => rows.len(),
                _ => return,
            };
            if table_insert_row(doc, addr, rows) {
                st.changed = true;
                st.pending_cell = Some((rows + 1, col));
            }
        }
    }
}

/// Focus `addr` for editing, dispatching on its kind: text blocks seed the
/// draft and set a caret hint, code/table enter their own edit modes, and
/// everything else falls back to selection.
fn focus_block(st: &mut BlockEditorState, doc: &Document, addr: Address, hint: CaretHint) {
    match doc.block(addr).map(|b| &b.kind) {
        Some(k) if k.is_text() => {
            st.focus = Some(addr);
            st.editing = true;
            st.draft = k.md().unwrap_or("").to_owned();
            st.pending_focus = Some((addr, hint));
            st.slash = None;
            st.cell = None;
        }
        Some(BlockKind::Code { .. }) => {
            st.focus = Some(addr);
            st.editing = true;
            st.pending_code = Some(addr);
        }
        Some(BlockKind::Table { .. }) => {
            st.focus = Some(addr);
            st.editing = true;
            st.cell = Some((0, 0));
            st.pending_cell = Some((0, 0));
        }
        Some(_) => select_block(st, addr),
        None => {}
    }
}

fn select_block(st: &mut BlockEditorState, addr: Address) {
    st.focus = Some(addr);
    st.editing = false;
    st.slash = None;
    st.cell = None;
    st.pending_focus = None;
}

/// Arrow-key targets skip dividers (they are select-only).
fn prev_nav_target(doc: &Document, addr: Address) -> Option<Address> {
    let mut cur = prev_address(doc, addr)?;
    while matches!(doc.block(cur).map(|b| &b.kind), Some(BlockKind::Divider)) {
        cur = prev_address(doc, cur)?;
    }
    Some(cur)
}

fn next_nav_target(doc: &Document, addr: Address) -> Option<Address> {
    let mut cur = next_address(doc, addr)?;
    while matches!(doc.block(cur).map(|b| &b.kind), Some(BlockKind::Divider)) {
        cur = next_address(doc, cur)?;
    }
    Some(cur)
}

fn nav_to(
    st: &mut BlockEditorState,
    doc: &Document,
    target: Option<Address>,
    x: Option<f32>,
    fallback: CaretHint,
) {
    let Some(target) = target else { return };
    match doc.block(target).map(|b| &b.kind) {
        Some(k) if k.is_text() => {
            focus_block(st, doc, target, x.map(CaretHint::Col).unwrap_or(fallback));
        }
        Some(BlockKind::Code { .. }) => focus_block(st, doc, target, fallback),
        Some(_) => select_block(st, target),
        None => {}
    }
}

/* ---------------- small string helpers ---------------- */

/// Byte offset of the `char_idx`-th char (clamped to the string end).
pub(super) fn byte_of_char(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// Char index of the char containing/starting at `byte` (clamped to a
/// boundary at or before it).
pub(super) fn char_of_byte(s: &str, byte: usize) -> usize {
    let mut b = byte.min(s.len());
    while !s.is_char_boundary(b) {
        b -= 1;
    }
    s[..b].chars().count()
}

/// Read-only sibling list + index for ordinal computation.
pub(super) fn siblings(doc: &Document, addr: Address) -> (&[Block], usize) {
    match addr {
        Address::Root(i) => (&doc.blocks, i),
        Address::Cell { root, col, idx } => match doc.blocks.get(root).map(|b| &b.kind) {
            Some(BlockKind::Columns { columns }) if col < columns.len() => {
                (&columns[col].blocks, idx)
            }
            _ => (&doc.blocks, root),
        },
    }
}
