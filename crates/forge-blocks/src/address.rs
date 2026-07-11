//! Block addressing — how editors point at a block inside the (one-level)
//! document tree, and the flattened navigation order arrow keys move through.

use crate::schema::{Block, BlockKind, Document};

/// Position of a block: at the document root, or inside a column cell of the
/// `Columns` block at `root`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Address {
    Root(usize),
    Cell { root: usize, col: usize, idx: usize },
}

impl Address {
    /// Index of the root-level block this address lives under.
    pub fn root(&self) -> usize {
        match *self {
            Address::Root(i) => i,
            Address::Cell { root, .. } => root,
        }
    }

    pub fn in_column(&self) -> bool {
        matches!(self, Address::Cell { .. })
    }
}

impl Document {
    pub fn block(&self, addr: Address) -> Option<&Block> {
        match addr {
            Address::Root(i) => self.blocks.get(i),
            Address::Cell { root, col, idx } => match &self.blocks.get(root)?.kind {
                BlockKind::Columns { columns } => columns.get(col)?.blocks.get(idx),
                _ => None,
            },
        }
    }

    pub fn block_mut(&mut self, addr: Address) -> Option<&mut Block> {
        match addr {
            Address::Root(i) => self.blocks.get_mut(i),
            Address::Cell { root, col, idx } => match &mut self.blocks.get_mut(root)?.kind {
                BlockKind::Columns { columns } => columns.get_mut(col)?.blocks.get_mut(idx),
                _ => None,
            },
        }
    }

    /// The sibling list containing `addr` (root list or one column's list).
    pub(crate) fn siblings_mut(&mut self, addr: Address) -> Option<&mut Vec<Block>> {
        match addr {
            Address::Root(_) => Some(&mut self.blocks),
            Address::Cell { root, col, .. } => match &mut self.blocks.get_mut(root)?.kind {
                BlockKind::Columns { columns } => Some(&mut columns.get_mut(col)?.blocks),
                _ => None,
            },
        }
    }

    pub(crate) fn index_in_siblings(addr: Address) -> usize {
        match addr {
            Address::Root(i) => i,
            Address::Cell { idx, .. } => idx,
        }
    }

    pub(crate) fn with_index(addr: Address, index: usize) -> Address {
        match addr {
            Address::Root(_) => Address::Root(index),
            Address::Cell { root, col, .. } => Address::Cell {
                root,
                col,
                idx: index,
            },
        }
    }
}

/// Navigation order: root blocks top to bottom; a `Columns` block contributes
/// its cell children column-major (all of column 0, then column 1, …). The
/// `Columns` container itself is not in the list — it is addressed as
/// `Root(i)` only for structural ops (move/delete/unwrap).
pub fn flatten_addresses(doc: &Document) -> Vec<Address> {
    let mut out = Vec::new();
    for (i, block) in doc.blocks.iter().enumerate() {
        match &block.kind {
            BlockKind::Columns { columns } => {
                for (c, col) in columns.iter().enumerate() {
                    for (j, _) in col.blocks.iter().enumerate() {
                        out.push(Address::Cell {
                            root: i,
                            col: c,
                            idx: j,
                        });
                    }
                }
            }
            _ => out.push(Address::Root(i)),
        }
    }
    out
}

/// The address after `addr` in navigation order.
pub fn next_address(doc: &Document, addr: Address) -> Option<Address> {
    let flat = flatten_addresses(doc);
    let pos = flat.iter().position(|a| *a == addr)?;
    flat.get(pos + 1).copied()
}

/// The address before `addr` in navigation order.
pub fn prev_address(doc: &Document, addr: Address) -> Option<Address> {
    let flat = flatten_addresses(doc);
    let pos = flat.iter().position(|a| *a == addr)?;
    pos.checked_sub(1).and_then(|p| flat.get(p)).copied()
}
