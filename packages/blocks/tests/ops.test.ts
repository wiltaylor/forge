import { describe, it, expect } from 'vitest';
import {
  findBlock, insertAfter, iterBlocks, mergeWithPrevious, moveBlock, removeBlock,
  replaceBlock, setColumnRatios, setListIndent, splitTextBlock, tableInsertCol,
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

describe('split / merge', () => {
  it('splits a paragraph at the caret and merge restores it', () => {
    const d = doc(p('hello world', 'a'));
    const { doc: after, focusId } = splitTextBlock(d, 'a', 5);
    expect(after.blocks).toHaveLength(2);
    expect((after.blocks[0] as { md: string }).md).toBe('hello');
    expect((after.blocks[1] as { md: string }).md).toBe(' world');
    const merged = mergeWithPrevious(after, focusId)!;
    expect(merged.caret).toBe(5);
    expect((merged.doc.blocks[0] as { md: string }).md).toBe('hello world');
    expect(merged.doc.blocks).toHaveLength(1);
  });

  it('heading tail becomes a paragraph; list continues the list', () => {
    const h: Block = { id: 'h', type: 'heading', level: 2, md: 'ab' };
    const { doc: afterH } = splitTextBlock(doc(h), 'h', 1);
    expect(afterH.blocks[1]!.type).toBe('paragraph');

    const li: Block = { id: 'l', type: 'list_item', style: 'todo', checked: true, indent: 2, md: 'task' };
    const { doc: afterL } = splitTextBlock(doc(li), 'l', 4);
    const tail = afterL.blocks[1] as Extract<Block, { type: 'list_item' }>;
    expect(tail.style).toBe('todo');
    expect(tail.indent).toBe(2);
    expect(tail.checked).toBe(false);
  });

  it('enter on an empty list item converts it in place', () => {
    const li: Block = { id: 'l', type: 'list_item', style: 'bullet', indent: 1, md: '' };
    const { doc: after, focusId } = splitTextBlock(doc(li), 'l', 0);
    expect(focusId).toBe('l');
    expect(after.blocks).toHaveLength(1);
    expect(after.blocks[0]!.type).toBe('paragraph');
  });

  it('merge with a divider deletes the divider', () => {
    const d = doc(p('a', 'a'), { id: 'd', type: 'divider' }, p('b', 'b'));
    const r = mergeWithPrevious(d, 'b')!;
    expect(r.focusId).toBe('b');
    expect(r.caret).toBe(0);
    expect(r.doc.blocks).toHaveLength(2);
  });

  it('first block cannot merge', () => {
    expect(mergeWithPrevious(doc(p('a', 'a')), 'a')).toBeNull();
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

  it('edits cells including the header', () => {
    let d = doc(table());
    d = tableSetCell(d, 't', -1, 0, 'H');
    d = tableSetCell(d, 't', 0, 1, 'x');
    const t = d.blocks[0] as Extract<Block, { type: 'table' }>;
    expect(t.header[0]).toBe('H');
    expect(t.rows[0]![1]).toBe('x');
    expect(tableSetCell(d, 't', 5, 0, 'y')).toBe(d);
  });

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
  it('moveBlock swaps within its list', () => {
    const d = doc(p('a', 'a'), p('b', 'b'));
    const next = moveBlock(d, 'a', 1);
    expect(next.blocks.map((b) => b.id)).toEqual(['b', 'a']);
  });

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

  it('list indent clamps 0..5', () => {
    const li: Block = { id: 'l', type: 'list_item', style: 'bullet', indent: 0, md: 'x' };
    let d = doc(li);
    expect(setListIndent(d, 'l', -1)).toBe(d);
    for (let i = 0; i < 10; i++) d = setListIndent(d, 'l', 1);
    expect((d.blocks[0] as { indent: number }).indent).toBe(5);
  });

  it('arrow navigation skips dividers', () => {
    const d = doc(p('a', 'a'), { id: 'd', type: 'divider' }, p('b', 'b'));
    expect(nextEditable(d, 'a')!.block.id).toBe('b');
    expect(prevEditable(d, 'b')!.block.id).toBe('a');
  });
});
