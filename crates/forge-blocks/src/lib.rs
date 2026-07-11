//! Forge block document model — the cross-platform page-builder contract.
//!
//! A [`Document`] is a flat list of typed [`Block`]s (paragraphs, headings,
//! lists, quotes, dividers, code, tables, admonitions, one level of columns,
//! and consumer-defined `custom` blocks). Inline content is stored as raw
//! markdown source strings (`**bold**`, `` `code` ``, `[link](url)`,
//! `:emoji:`); renderers parse it on the fly via [`parse_inline`].
//!
//! The JSON shape produced by serde here is the interchange contract shared
//! verbatim with `@forge/blocks` on the web — see `tests/schema.rs` for the
//! literal fixtures. Editors in forge-tui and forge-egui build on [`ops`] and
//! [`Address`] so every platform shares one keyboard/editing model.

mod address;
mod emoji;
mod id;
mod ops;
mod schema;

#[cfg(feature = "md")]
mod convert;
#[cfg(feature = "md")]
mod inline;

#[doc(hidden)]
pub mod sample;

pub use address::{flatten_addresses, next_address, prev_address, Address};
pub use emoji::{emoji, resolve_shortcodes, search_emoji, EMOJI};
pub use id::new_id;
pub use ops::{
    add_column, indent_list, insert_after, line_start_shortcut, merge_with_previous, move_block,
    remove, remove_column, set_column_ratios, set_kind, split, table_insert_col, table_insert_row,
    table_remove_col, table_remove_row, wrap_in_columns, MergeResult, Shortcut,
};
pub use schema::{Block, BlockKind, Column, Document, ListStyle, Tone, DOCUMENT_VERSION};

#[cfg(feature = "md")]
pub use convert::{from_markdown, to_markdown};
#[cfg(feature = "md")]
pub use inline::{parse_inline, safe_url, wrap_spans, InlineSpan};
