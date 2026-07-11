/* Pure document operations — the kanban board.ts idiom: fresh arrays on
   change, untouched blocks keep object identity, no-ops return the same
   document reference. Mirrors crates/forge-blocks/src/ops.rs. */
import type { Block, BlockDocument, BlockColumn, TextBlock } from './types';
import { createBlock, isTextBlock, newId } from './types';

export type Parent = { kind: 'root' } | { kind: 'column'; columnsId: string; col: number };

export interface Located {
  block: Block;
  parent: Parent;
  index: number;
}

/* ---------------- Lookup ---------------------------------------------------- */

/** Every block in document order; `columns` containers contribute their cell
    children column-major (the container itself is not yielded). */
export function* iterBlocks(doc: BlockDocument): Generator<Located> {
  for (let i = 0; i < doc.blocks.length; i++) {
    const b = doc.blocks[i]!;
    if (b.type === 'columns') {
      for (let c = 0; c < b.columns.length; c++) {
        const col = b.columns[c]!;
        for (let j = 0; j < col.blocks.length; j++) {
          yield {
            block: col.blocks[j]!,
            parent: { kind: 'column', columnsId: b.id, col: c },
            index: j,
          };
        }
      }
    } else {
      yield { block: b, parent: { kind: 'root' }, index: i };
    }
  }
}

export function findBlock(doc: BlockDocument, id: string): Located | null {
  for (let i = 0; i < doc.blocks.length; i++) {
    const b = doc.blocks[i]!;
    if (b.id === id) return { block: b, parent: { kind: 'root' }, index: i };
    if (b.type === 'columns') {
      for (let c = 0; c < b.columns.length; c++) {
        const col = b.columns[c]!;
        for (let j = 0; j < col.blocks.length; j++) {
          if (col.blocks[j]!.id === id)
            return {
              block: col.blocks[j]!,
              parent: { kind: 'column', columnsId: b.id, col: c },
              index: j,
            };
        }
      }
    }
  }
  return null;
}

function listOf(doc: BlockDocument, parent: Parent): Block[] | null {
  if (parent.kind === 'root') return doc.blocks;
  const host = doc.blocks.find((b) => b.id === parent.columnsId);
  if (host?.type !== 'columns') return null;
  return host.columns[parent.col]?.blocks ?? null;
}

/** Rebuild the document with `parent`'s sibling list replaced. */
function withList(doc: BlockDocument, parent: Parent, next: Block[]): BlockDocument {
  if (parent.kind === 'root') return { ...doc, blocks: next };
  return {
    ...doc,
    blocks: doc.blocks.map((b) => {
      if (b.id !== parent.columnsId || b.type !== 'columns') return b;
      return {
        ...b,
        columns: b.columns.map((c, i) => (i === parent.col ? { ...c, blocks: next } : c)),
      };
    }),
  };
}

/* ---------------- Basic edits ------------------------------------------------ */

/** Shallow-merge a patch into the block with `id` (same type assumed). */
export function updateBlock(doc: BlockDocument, id: string, patch: object): BlockDocument {
  const loc = findBlock(doc, id);
  if (!loc) return doc;
  const list = listOf(doc, loc.parent)!;
  const next = list.slice();
  next[loc.index] = { ...loc.block, ...patch } as Block;
  return withList(doc, loc.parent, next);
}

/** Replace the block with `id` wholesale (type conversions keep the id). */
export function replaceBlock(doc: BlockDocument, id: string, block: Block): BlockDocument {
  // A columns block never nests inside a column.
  const loc = findBlock(doc, id);
  if (!loc || (block.type === 'columns' && loc.parent.kind === 'column')) return doc;
  const list = listOf(doc, loc.parent)!;
  const next = list.slice();
  next[loc.index] = block;
  return withList(doc, loc.parent, next);
}

/** Insert `block` after the block with `id` (same sibling list). */
export function insertAfter(doc: BlockDocument, id: string, block: Block): BlockDocument {
  const loc = findBlock(doc, id);
  if (!loc) return doc;
  const list = listOf(doc, loc.parent)!;
  const next = list.slice();
  next.splice(loc.index + 1, 0, block);
  return withList(doc, loc.parent, next);
}

/** Remove the block; the document never goes blockless and column cells
    never go empty (they refill with an empty paragraph). */
export function removeBlock(doc: BlockDocument, id: string): BlockDocument {
  const loc = findBlock(doc, id);
  if (!loc) return doc;
  const list = listOf(doc, loc.parent)!;
  let next = list.filter((b) => b.id !== id);
  if (next.length === 0) {
    if (loc.parent.kind === 'root') next = [createBlock('paragraph')];
    else next = [createBlock('paragraph')];
  }
  return withList(doc, loc.parent, next);
}

/** Swap the block with its sibling above (-1) or below (+1). */
export function moveBlock(doc: BlockDocument, id: string, delta: -1 | 1): BlockDocument {
  const loc = findBlock(doc, id);
  if (!loc) return doc;
  const list = listOf(doc, loc.parent)!;
  const target = loc.index + delta;
  if (target < 0 || target >= list.length) return doc;
  const next = list.slice();
  next[loc.index] = list[target]!;
  next[target] = list[loc.index]!;
  return withList(doc, loc.parent, next);
}

/* ---------------- Text ops --------------------------------------------------- */

/** Enter inside a text block: split `md` at the caret. Heading/quote/
    admonition tails become paragraphs; list items continue the list (todo
    unchecked). Enter on an empty list item converts it to a paragraph in
    place instead. Returns the id to focus (caret 0). */
export function splitTextBlock(
  doc: BlockDocument,
  id: string,
  offset: number,
): { doc: BlockDocument; focusId: string } {
  const loc = findBlock(doc, id);
  if (!loc || !isTextBlock(loc.block)) return { doc, focusId: id };
  const b = loc.block;

  if (b.type === 'list_item' && b.md === '') {
    return {
      doc: replaceBlock(doc, id, { id, type: 'paragraph', md: '' }),
      focusId: id,
    };
  }

  const head = b.md.slice(0, offset);
  const tail = b.md.slice(offset);
  const tailBlock: Block =
    b.type === 'list_item'
      ? {
          id: newId(),
          type: 'list_item',
          style: b.style,
          indent: b.indent,
          ...(b.checked !== undefined ? { checked: false } : {}),
          md: tail,
        }
      : { id: newId(), type: 'paragraph', md: tail };

  let next = updateBlock(doc, id, { md: head });
  next = insertAfter(next, id, tailBlock);
  return { doc: next, focusId: tailBlock.id };
}

/** Backspace at offset 0: append this paragraph's `md` to the previous text
    block. A previous divider is deleted instead (caret stays at 0). Returns
    null when there is nothing to merge with. Callers convert non-paragraph
    text blocks to paragraphs first (the shared keyboard rule). */
export function mergeWithPrevious(
  doc: BlockDocument,
  id: string,
): { doc: BlockDocument; focusId: string; caret: number } | null {
  const loc = findBlock(doc, id);
  if (!loc || loc.index === 0 || loc.block.type !== 'paragraph') return null;
  const list = listOf(doc, loc.parent)!;
  const prev = list[loc.index - 1]!;

  if (prev.type === 'divider') {
    const next = list.slice();
    next.splice(loc.index - 1, 1);
    return { doc: withList(doc, loc.parent, next), focusId: id, caret: 0 };
  }
  if (!isTextBlock(prev)) return null;

  const caret = prev.md.length;
  const next = list.slice();
  next[loc.index - 1] = { ...prev, md: prev.md + loc.block.md } as Block;
  next.splice(loc.index, 1);
  return { doc: withList(doc, loc.parent, next), focusId: prev.id, caret };
}

/** Tab/Shift+Tab on a list item: indent by delta, clamped 0..=5. */
export function setListIndent(doc: BlockDocument, id: string, delta: number): BlockDocument {
  const loc = findBlock(doc, id);
  if (loc?.block.type !== 'list_item') return doc;
  const indent = Math.max(0, Math.min(5, loc.block.indent + delta));
  if (indent === loc.block.indent) return doc;
  return updateBlock(doc, id, { indent });
}

/** Previous/next block in navigation order, skipping dividers. */
export function prevEditable(doc: BlockDocument, id: string): Located | null {
  return step(doc, id, -1);
}
export function nextEditable(doc: BlockDocument, id: string): Located | null {
  return step(doc, id, 1);
}

function step(doc: BlockDocument, id: string, dir: -1 | 1): Located | null {
  const flat = [...iterBlocks(doc)];
  const at = flat.findIndex((l) => l.block.id === id);
  if (at < 0) return null;
  for (let i = at + dir; i >= 0 && i < flat.length; i += dir) {
    if (flat[i]!.block.type !== 'divider') return flat[i]!;
  }
  return null;
}

/* ---------------- Columns ---------------------------------------------------- */

/** Wrap a root block into an n-column layout: the block becomes column 0's
    content, other columns start with an empty paragraph. */
export function wrapInColumns(doc: BlockDocument, id: string, n: 2 | 3 | 4): BlockDocument {
  const loc = findBlock(doc, id);
  if (!loc || loc.parent.kind !== 'root' || loc.block.type === 'columns') return doc;
  const ratio = 1 / n;
  const columns: BlockColumn[] = [{ ratio, blocks: [loc.block] }];
  for (let i = 1; i < n; i++) columns.push({ ratio, blocks: [createBlock('paragraph')] });
  const next = doc.blocks.slice();
  next[loc.index] = { id: newId(), type: 'columns', columns };
  return { ...doc, blocks: next };
}

/** Append a column (max 4); existing ratios shrink proportionally. */
export function addColumn(doc: BlockDocument, columnsId: string): BlockDocument {
  const host = doc.blocks.find((b) => b.id === columnsId);
  if (host?.type !== 'columns' || host.columns.length >= 4) return doc;
  const n = host.columns.length;
  const columns = host.columns.map((c) => ({ ...c, ratio: (c.ratio * n) / (n + 1) }));
  columns.push({ ratio: 1 / (n + 1), blocks: [createBlock('paragraph')] });
  return {
    ...doc,
    blocks: doc.blocks.map((b) => (b.id === columnsId ? { ...b, columns } : b)),
  };
}

/** Remove a column; its blocks splice into the neighbour. Removing the
    second-to-last column unwraps the survivor's blocks to the root. */
export function removeColumn(doc: BlockDocument, columnsId: string, col: number): BlockDocument {
  const idx = doc.blocks.findIndex((b) => b.id === columnsId);
  const host = doc.blocks[idx];
  if (host?.type !== 'columns' || col >= host.columns.length) return doc;

  if (host.columns.length <= 2) {
    const keep = host.columns[col === 0 ? 1 : 0]!;
    const next = doc.blocks.slice();
    next.splice(idx, 1, ...(keep.blocks.length ? keep.blocks : [createBlock('paragraph')]));
    return { ...doc, blocks: next };
  }

  const removed = host.columns[col]!;
  const into = col === 0 ? 0 : col - 1;
  const remaining = host.columns.filter((_, i) => i !== col);
  const extra = removed.ratio / remaining.length;
  const columns = remaining.map((c, i) => ({
    ...c,
    ratio: c.ratio + extra,
    blocks: i === into ? [...c.blocks, ...removed.blocks] : c.blocks,
  }));
  return {
    ...doc,
    blocks: doc.blocks.map((b) => (b.id === columnsId ? { ...b, columns } : b)),
  };
}

/** Set column ratios (normalized against their sum, min 10% each). */
export function setColumnRatios(
  doc: BlockDocument,
  columnsId: string,
  ratios: number[],
): BlockDocument {
  const host = doc.blocks.find((b) => b.id === columnsId);
  if (
    host?.type !== 'columns' ||
    ratios.length !== host.columns.length ||
    ratios.some((r) => !Number.isFinite(r) || r <= 0)
  )
    return doc;
  const sum = ratios.reduce((a, b) => a + b, 0);
  const columns = host.columns.map((c, i) => ({ ...c, ratio: Math.max(0.1, ratios[i]! / sum) }));
  return {
    ...doc,
    blocks: doc.blocks.map((b) => (b.id === columnsId ? { ...b, columns } : b)),
  };
}

/* ---------------- Tables ------------------------------------------------------ */

type TableBlock = Extract<Block, { type: 'table' }>;

function patchTable(
  doc: BlockDocument,
  id: string,
  fn: (t: TableBlock) => Partial<TableBlock> | null,
): BlockDocument {
  const loc = findBlock(doc, id);
  if (loc?.block.type !== 'table') return doc;
  const patch = fn(loc.block);
  return patch ? updateBlock(doc, id, patch) : doc;
}

/** Set one cell's inline-md source; row -1 addresses the header. */
export function tableSetCell(
  doc: BlockDocument,
  id: string,
  row: number,
  col: number,
  md: string,
): BlockDocument {
  return patchTable(doc, id, (t) => {
    if (row === -1) {
      if (col >= t.header.length || t.header[col] === md) return null;
      const header = t.header.slice();
      header[col] = md;
      return { header };
    }
    if (row >= t.rows.length || col >= (t.rows[row]?.length ?? 0)) return null;
    if (t.rows[row]![col] === md) return null;
    const rows = t.rows.slice();
    rows[row] = rows[row]!.slice();
    rows[row]![col] = md;
    return { rows };
  });
}

export function tableInsertRow(doc: BlockDocument, id: string, at: number): BlockDocument {
  return patchTable(doc, id, (t) => {
    const rows = t.rows.slice();
    rows.splice(Math.min(at, rows.length), 0, new Array(t.header.length).fill(''));
    return { rows };
  });
}

export function tableRemoveRow(doc: BlockDocument, id: string, at: number): BlockDocument {
  return patchTable(doc, id, (t) => {
    if (t.rows.length <= 1 || at >= t.rows.length) return null;
    return { rows: t.rows.filter((_, i) => i !== at) };
  });
}

export function tableInsertCol(doc: BlockDocument, id: string, at: number): BlockDocument {
  return patchTable(doc, id, (t) => {
    const pos = Math.min(at, t.header.length);
    const header = t.header.slice();
    header.splice(pos, 0, '');
    const rows = t.rows.map((r) => {
      const row = r.slice();
      row.splice(Math.min(pos, row.length), 0, '');
      return row;
    });
    return { header, rows };
  });
}

export function tableRemoveCol(doc: BlockDocument, id: string, at: number): BlockDocument {
  return patchTable(doc, id, (t) => {
    if (t.header.length <= 1 || at >= t.header.length) return null;
    return {
      header: t.header.filter((_, i) => i !== at),
      rows: t.rows.map((r) => r.filter((_, i) => i !== at)),
    };
  });
}
