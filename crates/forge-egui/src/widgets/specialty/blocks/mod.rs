//! Block page editor (cargo feature `blocks`) — the egui editor for the
//! `forge-blocks` document model, sibling of the web and TUI block editors.
//!
//! [`BlockEditor`] renders a [`Document`] as a vertical list of blocks.
//! Unfocused text blocks show styled inline markdown (via
//! `forge_blocks::parse_inline`); clicking one swaps in a frameless
//! `TextEdit` bound to the raw markdown source.
//!
//! The keyboard model is not implemented here. [`forge_blocks::resolve_key`]
//! owns it — Enter splits, Backspace-at-0 demotes then merges, Tab indents
//! list items, Alt+↑/↓ moves blocks, `/` on an empty block opens the palette
//! — and the [`keys`] module is this kit's adapter onto it: egui input in,
//! an [`Op`](forge_blocks::Op) out, performed as an [`Action`] here. What is
//! left in this file is the kit's own work: focus, drafts, popups and paint.
//!
//! ```ignore
//! let mut state = BlockEditorState::new(Document::new());
//! let response = BlockEditor::new(&mut state).show(ui);
//! if response.changed() { save(&state.doc); }
//! ```

mod chrome;
mod data;
mod inline;
mod keys;
mod kinds;
mod popups;
mod text;

use crate::response::{ForgeResponse, Outcome};
use crate::theme::Theme;
use egui::Ui;
use forge_blocks::{
    indent_list, insert_after, merge_with_previous, move_block, next_address, prev_address, remove,
    set_kind, split, table_insert_row, wrap_in_columns, Address, Block, BlockKind, Document, Mode,
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
    /// The focused data block's fields as pretty JSON (parse-on-commit —
    /// only the Esc commit writes it back into the document).
    json_draft: String,
    json_err: Option<String>,
    json_dirty_since_err: bool,
    pending_json: Option<Address>,
    /// Decoded image textures per `src` (`None` caches decode failures).
    #[cfg(feature = "images")]
    img_cache: std::collections::HashMap<String, Option<egui::TextureHandle>>,
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
            json_draft: String::new(),
            json_err: None,
            json_dirty_since_err: false,
            pending_json: None,
            #[cfg(feature = "images")]
            img_cache: std::collections::HashMap::new(),
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

    /// Select `addr` in block mode — structural keys, no text caret.
    pub fn select(&mut self, addr: Address) {
        select_block(self, addr);
    }

    /// Enter the block at `addr` for editing, with the text caret at `caret`
    /// (a byte offset into its markdown source). Blocks that only support
    /// selection fall back to it; the return says which happened.
    pub fn edit(&mut self, addr: Address, caret: usize) -> bool {
        let doc = std::mem::take(&mut self.doc);
        focus_block(self, &doc, addr, CaretHint::Byte(caret));
        self.doc = doc;
        self.editing
    }

    /// Enter the table at `addr` on one cell: display row 0 is the header,
    /// body rows follow. Returns false when the block is not a table or the
    /// cell is outside it.
    pub fn edit_cell(&mut self, addr: Address, row: usize, col: usize) -> bool {
        let (ncols, nrows) = match self.doc.block(addr).map(|b| &b.kind) {
            Some(BlockKind::Table { header, rows }) => (header.len().max(1), rows.len()),
            _ => return false,
        };
        if col >= ncols || row > nrows {
            return false;
        }
        enter_cell(self, addr, row, col);
        true
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
            selection_keys(ui, &mut ecx, st, &doc);
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
///
/// Most of these are one resolved [`Op`](forge_blocks::Op) each, performed
/// where the document can be mutated safely; the rest are the mouse's.
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Focus(Address, CaretHint),
    Select(Address),
    Split {
        addr: Address,
        /// Byte offset in the block's markdown source to split at.
        caret: usize,
    },
    /// Turn a non-paragraph text block into a paragraph, keeping its
    /// markdown — the rule that stands between Backspace-at-0 and a merge.
    Demote(Address),
    /// Append the paragraph at the address to the block above it.
    Merge(Address),
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
    /// Cycle the focused admonition's tone.
    CycleTone(Address),
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
    /// Insert an empty table row at `at`, then focus the cell in `focus`
    /// (display row 0 is the header).
    InsertTableRow {
        addr: Address,
        at: usize,
        focus: (usize, usize),
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
        BlockKind::Footnote { .. } => text::text_row(ui, ecx, st, doc, addr, id),
        kind if kind.is_data() => data::data_block(ui, ecx, st, doc, addr, id),
        _ => {}
    }
}

/* ---------------- keyboard: block-selection mode ---------------- */

/// Selection-mode keys — a block selected, nothing being edited, no text
/// caret anywhere. What they mean is [`forge_blocks::resolve_key`]'s call;
/// this only says which block is listening.
fn selection_keys(ui: &Ui, ecx: &mut Ecx, st: &mut BlockEditorState, doc: &Document) {
    let Some(addr) = st.focus else { return };
    if st.editing || st.slash.is_some() {
        return;
    }
    // Never steal keys while some widget (ours or the app's) owns focus.
    if ui.ctx().memory(|m| m.focused().is_some()) {
        return;
    }
    keys::handle(
        ui,
        ecx,
        st,
        doc,
        keys::Focused {
            addr,
            mode: Mode::Select,
            buffer: None,
            selection: false,
        },
    );
}

/* ---------------- action application ---------------- */

fn apply(st: &mut BlockEditorState, doc: &mut Document, action: Action) {
    match action {
        Action::Focus(addr, hint) => focus_block(st, doc, addr, hint),
        Action::Select(addr) => select_block(st, addr),
        Action::Split { addr, caret } => {
            if let Some(next) = split(doc, addr, caret) {
                st.changed = true;
                focus_block(st, doc, next, CaretHint::Start);
            }
        }
        Action::Demote(addr) => {
            let md = doc
                .block(addr)
                .and_then(|b| b.kind.md())
                .unwrap_or("")
                .to_owned();
            // The caret stays at 0 in the same block, so the draft the
            // `TextEdit` is bound to is already right.
            if set_kind(doc, addr, BlockKind::Paragraph { md }) {
                st.changed = true;
            }
        }
        Action::Merge(addr) => merge_up(st, doc, addr),
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
        Action::CycleTone(addr) => {
            if let Some(BlockKind::Admonition { tone, .. }) =
                doc.block_mut(addr).map(|b| &mut b.kind)
            {
                *tone = tone.next();
                st.changed = true;
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
        Action::InsertTableRow { addr, at, focus } => {
            if table_insert_row(doc, addr, at) {
                st.changed = true;
                st.pending_cell = Some(focus);
            }
        }
    }
}

/// Merge the paragraph at `addr` into the block above it and follow the
/// caret to the seam. The one merge both binding sites use: Backspace-at-0
/// passes the focused block, Delete-at-end passes the one below it.
fn merge_up(st: &mut BlockEditorState, doc: &mut Document, addr: Address) {
    if let Some(merge) = merge_with_previous(doc, addr) {
        st.changed = true;
        focus_block(st, doc, merge.focus, CaretHint::Byte(merge.caret));
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
        Some(BlockKind::Table { .. }) => enter_cell(st, addr, 0, 0),
        Some(k) if k.is_data() => {
            st.focus = Some(addr);
            st.editing = true;
            st.json_draft = serde_json::to_string_pretty(k).unwrap_or_default();
            st.json_err = None;
            st.json_dirty_since_err = false;
            st.pending_json = Some(addr);
            st.slash = None;
            st.cell = None;
            st.pending_focus = None;
        }
        Some(_) => select_block(st, addr),
        None => {}
    }
}

/// Enter a table's cell: display row 0 is the header, body rows follow. The
/// one way in, so entering at (0, 0) and entering elsewhere cannot drift.
fn enter_cell(st: &mut BlockEditorState, addr: Address, row: usize, col: usize) {
    st.focus = Some(addr);
    st.editing = true;
    st.cell = Some((row, col));
    st.pending_cell = Some((row, col));
    st.slash = None;
    st.pending_focus = None;
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
