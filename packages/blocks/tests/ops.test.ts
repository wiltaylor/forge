/* Pure document operations.

   What a keypress does with these — splitting, merging, the demote-before-merge
   rule, indent clamping, block moves, the line-start shortcut grammar and the
   table keys — is the block key corpus's, not this file's:
   `contract/blocks/corpus.json`, driven by `block_corpus.test.tsx` here and by
   `crates/forge-tui/tests/block_corpus.rs` and its egui sibling over there. Two
   languages ran near-duplicate hand-written assertions on that model; the corpus
   replaced them, so adding a case now covers both.

   What stays here is what no key reaches: the identity discipline the editor's
   rendering depends on, column ratios, and the table operations the web kit
   drives from its toolbar. */
import { describe, it, expect } from 'vitest';
import {
  findBlock, insertAfter, iterBlocks, moveBlock, removeBlock,
  replaceBlock, setColumnRatios, setListIndent, tableInsertCol,
  tableInsertRow, tableRemoveCol, tableRemoveRow, tableSetCell, updateBlock,
  wrapInColumns, addColumn, removeColumn, prevEditable, nextEditable,
} from '../src/ops';
import type { Block, BlockDocument } from '../src/types';
import { DOC_VERSION } from '../src/types';

let n = 0;
const p = (md: string, id = `p${n++}`): Block => ({ id, type: 'paragraph', md });
const doc = (...blocks: Block[]): BlockDocument => ({ version: DOC_VERSION, blocks });

describe('identity discipline', () => {
  it('no-ops return the same reference', () => {
    const d = doc(p('a', 'a'));
    expect(updateBlock(d, 'missing', { md: 'x' })).toBe(d);
    expect(moveBlock(d, 'a', -1)).toBe(d);
    expect(removeBlock(d, 'missing')).toBe(d);
    expect(setListIndent(d, 'a', 1)).toBe(d);
  });
  it('untouched blocks keep identity', () => {
    const d = doc(p('a', 'a'), p('b', 'b'));
    const next = updateBlock(d, 'b', { md: 'B' });
    expect(next.blocks[0]).toBe(d.blocks[0]);
    expect(next.blocks[1]).not.toBe(d.blocks[1]);
  });
});

describe('columns', () => {
  it('wraps, addresses, and unwraps', () => {
    let d = doc(p('a', 'a'), p('b', 'b'));
    d = wrapInColumns(d, 'a', 2);
    const cols = d.blocks[0] as Extract<Block, { type: 'columns' }>;
    expect(cols.columns).toHaveLength(2);
    expect(findBlock(d, 'a')!.parent).toEqual({
      kind: 'column', columnsId: cols.id, col: 0,
    });

    // Navigation flattens through columns: a, empty col paragraph, b.
    expect([...iterBlocks(d)]).toHaveLength(3);
    expect(nextEditable(d, 'a')!.block.type).toBe('paragraph');
    expect(prevEditable(d, 'b')!.parent.kind).toBe('column');

    // No nested columns.
    const before = d;
    expect(wrapInColumns(d, 'a', 2)).toBe(before);
    expect(replaceBlock(d, 'a', { id: 'a', type: 'columns', columns: [] })).toBe(before);

    d = addColumn(d, cols.id);
    expect((d.blocks[0] as typeof cols).columns).toHaveLength(3);
    d = removeColumn(d, cols.id, 2);
    d = removeColumn(d, cols.id, 1);
    expect(d.blocks[0]!.type).toBe('paragraph');
    expect((d.blocks[0] as { md: string }).md).toBe('a');
  });

  it('normalizes ratios with a floor', () => {
    let d = wrapInColumns(doc(p('a', 'a')), 'a', 2);
    const id = d.blocks[0]!.id;
    d = setColumnRatios(d, id, [3, 1]);
    const cols = d.blocks[0] as Extract<Block, { type: 'columns' }>;
    expect(cols.columns[0]!.ratio).toBeCloseTo(0.75);
    expect(cols.columns[1]!.ratio).toBeCloseTo(0.25);
    expect(setColumnRatios(d, id, [1])).toBe(d);
    expect(setColumnRatios(d, id, [-1, 1])).toBe(d);
  });
});

describe('tables', () => {
  const table = (): Block => ({
    id: 't', type: 'table', header: ['A', 'B'], rows: [['1', '2']],
  });

  it('ignores a cell outside the table', () => {
    const d = doc(table());
    expect(tableSetCell(d, 't', 5, 0, 'y')).toBe(d);
    expect(tableSetCell(d, 't', 0, 9, 'y')).toBe(d);
  });

  // Reached from the table toolbar, not from a key — so not the corpus's.
  it('inserts and removes rows/cols with floors', () => {
    let d = doc(table());
    d = tableInsertRow(d, 't', 1);
    d = tableInsertCol(d, 't', 2);
    let t = d.blocks[0] as Extract<Block, { type: 'table' }>;
    expect(t.rows).toHaveLength(2);
    expect(t.header).toHaveLength(3);
    expect(t.rows.every((r) => r.length === 3)).toBe(true);
    d = tableRemoveCol(d, 't', 2);
    d = tableRemoveRow(d, 't', 1);
    d = tableRemoveRow(d, 't', 0); // floor: last row stays
    t = d.blocks[0] as Extract<Block, { type: 'table' }>;
    expect(t.rows).toHaveLength(1);
  });
});

describe('misc ops', () => {
  it('removeBlock refills an emptied document', () => {
    const d = removeBlock(doc(p('only', 'a')), 'a');
    expect(d.blocks).toHaveLength(1);
    expect((d.blocks[0] as { md: string }).md).toBe('');
  });

  it('insertAfter lands in the same sibling list', () => {
    let d = wrapInColumns(doc(p('a', 'a')), 'a', 2);
    d = insertAfter(d, 'a', p('new', 'n'));
    expect(findBlock(d, 'n')!.parent.kind).toBe('column');
    expect(findBlock(d, 'n')!.index).toBe(1);
  });

  it('arrow navigation skips dividers', () => {
    const d = doc(p('a', 'a'), { id: 'd', type: 'divider' }, p('b', 'b'));
    expect(nextEditable(d, 'a')!.block.id).toBe('b');
    expect(prevEditable(d, 'b')!.block.id).toBe('a');
  });
});
