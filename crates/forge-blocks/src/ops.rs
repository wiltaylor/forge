//! Pure document editing operations shared by the TUI and egui editors.
//!
//! Ops mutate the document in place and hand back the address (and caret,
//! where relevant) the editor should focus next. Undo is snapshot-based:
//! [`Document`] is `Clone`, so editors push a clone before each op batch.

use crate::address::Address;
use crate::schema::{Block, BlockKind, Column, Document, ListStyle, Tone};

/// A line-start markdown shortcut hit: replace the block's kind with `kind`
/// (which carries the text after the prefix); the caret moves back by
/// `prefix_len` bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcut {
    pub kind: BlockKind,
    pub prefix_len: usize,
}

/// Outcome of [`merge_with_previous`]: where the caret landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MergeResult {
    pub focus: Address,
    pub caret: usize,
}

/// Insert a fresh block of `kind` after `addr` (same sibling list). Returns
/// the new block's address.
pub fn insert_after(doc: &mut Document, addr: Address, kind: BlockKind) -> Option<Address> {
    let index = Document::index_in_siblings(addr);
    let list = doc.siblings_mut(addr)?;
    if index >= list.len() {
        return None;
    }
    list.insert(index + 1, Block::new(kind));
    Some(Document::with_index(addr, index + 1))
}

/// Remove the block at `addr`. Returns the address to focus next (previous
/// sibling, else the block now occupying the slot, else the previous
/// navigation stop). Keeps the document non-blockless and column cells
/// non-empty.
pub fn remove(doc: &mut Document, addr: Address) -> Option<Address> {
    let index = Document::index_in_siblings(addr);
    let list = doc.siblings_mut(addr)?;
    if index >= list.len() {
        return None;
    }
    list.remove(index);
    let len = list.len();
    doc.normalize();
    let focus = if index > 0 {
        Document::with_index(addr, index - 1)
    } else if len > 0 {
        Document::with_index(addr, 0)
    } else {
        // List emptied; normalize() refilled root, or the cell got a fresh
        // paragraph — focus slot 0 either way.
        Document::with_index(addr, 0)
    };
    // The focus slot is guaranteed valid post-normalize for cells and root.
    doc.block(focus).map(|_| focus)
}

/// Enter inside a text block: split its `md` at `caret` (byte offset).
/// Heading/quote/admonition tails become paragraphs; list items continue the
/// list (same style/indent, todo unchecked). Enter on an *empty* list item
/// converts it to a paragraph instead (returns the same address). Returns the
/// address to focus (caret 0).
pub fn split(doc: &mut Document, addr: Address, caret: usize) -> Option<Address> {
    let block = doc.block_mut(addr)?;
    if matches!(&block.kind, BlockKind::ListItem { md, .. } if md.is_empty()) {
        block.kind = BlockKind::Paragraph { md: String::new() };
        return Some(addr);
    }
    let tail_kind = match &mut block.kind {
        BlockKind::ListItem {
            style,
            indent,
            md,
            checked,
        } => {
            let tail = md.split_off(caret.min(md.len()));
            BlockKind::ListItem {
                style: *style,
                indent: *indent,
                checked: checked.map(|_| false),
                md: tail,
            }
        }
        BlockKind::Paragraph { md } => {
            let tail = md.split_off(caret.min(md.len()));
            BlockKind::Paragraph { md: tail }
        }
        BlockKind::Heading { md, .. }
        | BlockKind::Quote { md }
        | BlockKind::Admonition { md, .. } => {
            let tail = md.split_off(caret.min(md.len()));
            BlockKind::Paragraph { md: tail }
        }
        _ => return None,
    };
    insert_after(doc, addr, tail_kind)
}

/// Backspace at offset 0 between two text blocks: append this block's `md`
/// to the previous sibling's and remove this block. If the previous sibling
/// is a divider, the divider is deleted instead (caret stays at 0). Only
/// paragraphs merge *into* other blocks — callers convert non-paragraph text
/// blocks to paragraphs first (the shared keyboard rule). Returns `None`
/// when there is nothing to merge with (first block of its list).
pub fn merge_with_previous(doc: &mut Document, addr: Address) -> Option<MergeResult> {
    let index = Document::index_in_siblings(addr);
    if index == 0 {
        return None;
    }
    let prev_addr = Document::with_index(addr, index - 1);
    let prev_kind = &doc.block(prev_addr)?.kind;

    if matches!(prev_kind, BlockKind::Divider) {
        let list = doc.siblings_mut(addr)?;
        list.remove(index - 1);
        return Some(MergeResult {
            focus: prev_addr,
            caret: 0,
        });
    }
    if !prev_kind.is_text() {
        return None;
    }

    let md = match &doc.block(addr)?.kind {
        BlockKind::Paragraph { md } => md.clone(),
        _ => return None,
    };
    let prev_md = doc.block_mut(prev_addr)?.kind.md_mut()?;
    let caret = prev_md.len();
    prev_md.push_str(&md);
    doc.siblings_mut(addr)?.remove(index);
    Some(MergeResult {
        focus: prev_addr,
        caret,
    })
}

/// Alt+↑/↓: swap the block with its sibling in `dir` (-1 up, +1 down).
/// Returns the block's new address.
pub fn move_block(doc: &mut Document, addr: Address, dir: i32) -> Option<Address> {
    let index = Document::index_in_siblings(addr);
    let target = index.checked_add_signed(dir as isize)?;
    let list = doc.siblings_mut(addr)?;
    if index >= list.len() || target >= list.len() {
        return None;
    }
    list.swap(index, target);
    Some(Document::with_index(addr, target))
}

/// Replace the block's kind (slash-menu / "turn into" conversion). Refuses
/// to place a `Columns` kind inside a column cell (no nesting).
pub fn set_kind(doc: &mut Document, addr: Address, kind: BlockKind) -> bool {
    if addr.in_column() && matches!(kind, BlockKind::Columns { .. }) {
        return false;
    }
    match doc.block_mut(addr) {
        Some(block) => {
            block.kind = kind;
            true
        }
        None => false,
    }
}

/// Tab/Shift+Tab on a list item: adjust indent by `delta`, clamped 0..=5.
pub fn indent_list(doc: &mut Document, addr: Address, delta: i8) -> bool {
    if let Some(Block {
        kind: BlockKind::ListItem { indent, .. },
        ..
    }) = doc.block_mut(addr)
    {
        let next = (*indent as i8 + delta).clamp(0, 5) as u8;
        if next != *indent {
            *indent = next;
            return true;
        }
    }
    false
}

/* ---------------- Columns ---------------------------------------------- */

/// Wrap the root block at `addr` into an `n`-column layout: the block
/// becomes column 0's content, the other columns start with an empty
/// paragraph. Only root, non-`Columns` blocks wrap. Focus lands on the
/// wrapped block.
pub fn wrap_in_columns(doc: &mut Document, addr: Address, n: usize) -> Option<Address> {
    let Address::Root(i) = addr else { return None };
    if !(2..=4).contains(&n) || matches!(doc.blocks.get(i)?.kind, BlockKind::Columns { .. }) {
        return None;
    }
    let block = doc.blocks.remove(i);
    let ratio = 1.0 / n as f32;
    let mut columns = vec![Column {
        ratio,
        blocks: vec![block],
    }];
    for _ in 1..n {
        columns.push(Column {
            ratio,
            blocks: vec![Block::new(BlockKind::Paragraph { md: String::new() })],
        });
    }
    doc.blocks
        .insert(i, Block::new(BlockKind::Columns { columns }));
    Some(Address::Cell {
        root: i,
        col: 0,
        idx: 0,
    })
}

/// Append a column (max 4) to the `Columns` block at root index `root`.
/// Existing ratios shrink proportionally. Returns the new column's index.
pub fn add_column(doc: &mut Document, root: usize) -> Option<usize> {
    let BlockKind::Columns { columns } = &mut doc.blocks.get_mut(root)?.kind else {
        return None;
    };
    if columns.len() >= 4 {
        return None;
    }
    let n = columns.len() as f32;
    for col in columns.iter_mut() {
        col.ratio *= n / (n + 1.0);
    }
    columns.push(Column {
        ratio: 1.0 / (n + 1.0),
        blocks: vec![Block::new(BlockKind::Paragraph { md: String::new() })],
    });
    Some(columns.len() - 1)
}

/// Remove column `col`; its blocks splice into the previous column (or the
/// next, when removing column 0). Removing the second-to-last column unwraps
/// the survivor's blocks back to the root. Returns the address to focus.
pub fn remove_column(doc: &mut Document, root: usize, col: usize) -> Option<Address> {
    let BlockKind::Columns { columns } = &mut doc.blocks.get_mut(root)?.kind else {
        return None;
    };
    if col >= columns.len() {
        return None;
    }
    if columns.len() <= 2 {
        let keep = if col == 0 { 1 } else { 0 };
        let blocks = std::mem::take(&mut columns.get_mut(keep)?.blocks);
        doc.blocks.remove(root);
        for (offset, b) in blocks.into_iter().enumerate() {
            doc.blocks.insert(root + offset, b);
        }
        doc.normalize();
        return Some(Address::Root(root));
    }
    let removed = columns.remove(col);
    let into = if col == 0 { 0 } else { col - 1 };
    let target = columns.get_mut(into)?;
    let idx = target.blocks.len().saturating_sub(1);
    target.blocks.extend(removed.blocks);
    let n = columns.len() as f32;
    let extra = removed.ratio / n;
    for c in columns.iter_mut() {
        c.ratio += extra;
    }
    Some(Address::Cell {
        root,
        col: into,
        idx,
    })
}

/// Set the column ratios (normalized against their sum; each clamped to a
/// 10% minimum share).
pub fn set_column_ratios(doc: &mut Document, root: usize, ratios: &[f32]) -> bool {
    let Some(Block {
        kind: BlockKind::Columns { columns },
        ..
    }) = doc.blocks.get_mut(root)
    else {
        return false;
    };
    if ratios.len() != columns.len() || ratios.iter().any(|r| !r.is_finite() || *r <= 0.0) {
        return false;
    }
    let sum: f32 = ratios.iter().sum();
    for (c, r) in columns.iter_mut().zip(ratios) {
        c.ratio = (r / sum).max(0.1);
    }
    true
}

/* ---------------- Tables ------------------------------------------------ */

fn table_at(
    doc: &mut Document,
    addr: Address,
) -> Option<(&mut Vec<String>, &mut Vec<Vec<String>>)> {
    match &mut doc.block_mut(addr)?.kind {
        BlockKind::Table { header, rows } => Some((header, rows)),
        _ => None,
    }
}

/// Insert an empty row at `at` (clamped to the row count).
pub fn table_insert_row(doc: &mut Document, addr: Address, at: usize) -> bool {
    let Some((header, rows)) = table_at(doc, addr) else {
        return false;
    };
    let cols = header.len().max(1);
    let at = at.min(rows.len());
    rows.insert(at, vec![String::new(); cols]);
    true
}

pub fn table_remove_row(doc: &mut Document, addr: Address, at: usize) -> bool {
    let Some((_, rows)) = table_at(doc, addr) else {
        return false;
    };
    if at >= rows.len() || rows.len() <= 1 {
        return false;
    }
    rows.remove(at);
    true
}

/// Insert an empty column at `at` (clamped), in the header and every row.
pub fn table_insert_col(doc: &mut Document, addr: Address, at: usize) -> bool {
    let Some((header, rows)) = table_at(doc, addr) else {
        return false;
    };
    let at = at.min(header.len());
    header.insert(at, String::new());
    for row in rows.iter_mut() {
        row.insert(at.min(row.len()), String::new());
    }
    true
}

pub fn table_remove_col(doc: &mut Document, addr: Address, at: usize) -> bool {
    let Some((header, rows)) = table_at(doc, addr) else {
        return false;
    };
    if at >= header.len() || header.len() <= 1 {
        return false;
    }
    header.remove(at);
    for row in rows.iter_mut() {
        if at < row.len() {
            row.remove(at);
        }
    }
    true
}

/* ---------------- Line-start shortcuts ---------------------------------- */

/// Detect a markdown shortcut typed at the start of a paragraph. Editors call
/// this after each edit of a paragraph and, on a hit, replace the kind and
/// pull the caret back by `prefix_len`.
///
/// `# `..`#### `, `- `/`* `, `1. `/`1) `, `- [ ] `/`- [x] `/`[] `, `> `,
/// ```` ```lang ````, `---`, `:::info` (and the other tones).
pub fn line_start_shortcut(text: &str) -> Option<Shortcut> {
    // Todo items first: their prefix contains the bullet prefix.
    for (p, checked) in [("- [ ] ", false), ("- [x] ", true), ("[] ", false)] {
        if let Some(rest) = text.strip_prefix(p) {
            return Some(Shortcut {
                kind: BlockKind::ListItem {
                    style: ListStyle::Todo,
                    checked: Some(checked),
                    indent: 0,
                    md: rest.to_string(),
                },
                prefix_len: p.len(),
            });
        }
    }
    for (p, level) in [("#### ", 4u8), ("### ", 3), ("## ", 2), ("# ", 1)] {
        if let Some(rest) = text.strip_prefix(p) {
            return Some(Shortcut {
                kind: BlockKind::Heading {
                    level,
                    md: rest.to_string(),
                },
                prefix_len: p.len(),
            });
        }
    }
    for p in ["- ", "* "] {
        if let Some(rest) = text.strip_prefix(p) {
            return Some(Shortcut {
                kind: BlockKind::ListItem {
                    style: ListStyle::Bullet,
                    checked: None,
                    indent: 0,
                    md: rest.to_string(),
                },
                prefix_len: p.len(),
            });
        }
    }
    for p in ["1. ", "1) "] {
        if let Some(rest) = text.strip_prefix(p) {
            return Some(Shortcut {
                kind: BlockKind::ListItem {
                    style: ListStyle::Number,
                    checked: None,
                    indent: 0,
                    md: rest.to_string(),
                },
                prefix_len: p.len(),
            });
        }
    }
    if let Some(rest) = text.strip_prefix("> ") {
        return Some(Shortcut {
            kind: BlockKind::Quote {
                md: rest.to_string(),
            },
            prefix_len: 2,
        });
    }
    if let Some(lang) = text.strip_prefix("```") {
        if lang.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Some(Shortcut {
                kind: BlockKind::Code {
                    lang: lang.to_string(),
                    code: String::new(),
                },
                prefix_len: text.len(),
            });
        }
    }
    if text == "---" {
        return Some(Shortcut {
            kind: BlockKind::Divider,
            prefix_len: 3,
        });
    }
    if let Some(rest) = text.strip_prefix(":::") {
        let tone = match rest {
            "info" => Some(Tone::Info),
            "success" => Some(Tone::Success),
            "warning" => Some(Tone::Warning),
            "danger" => Some(Tone::Danger),
            _ => None,
        };
        if let Some(tone) = tone {
            return Some(Shortcut {
                kind: BlockKind::Admonition {
                    tone,
                    title: String::new(),
                    md: String::new(),
                },
                prefix_len: text.len(),
            });
        }
    }
    None
}
