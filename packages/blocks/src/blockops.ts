/* Editor-level compound operations built on the pure ops. */
import { insertAfter, splitTextBlock, updateBlock, removeBlock, findBlock } from './ops';
import type { Block, BlockDocument } from './types';
import { isTextBlock, newId } from './types';

/** Deep-clone a block with fresh ids (columns children included). */
export function cloneBlock(b: Block): Block {
  if (b.type === 'columns') {
    return {
      ...b,
      id: newId(),
      columns: b.columns.map((c) => ({ ...c, blocks: c.blocks.map(cloneBlock) })),
    };
  }
  if (b.type === 'custom') {
    return { ...b, id: newId(), data: structuredClone(b.data) };
  }
  return { ...b, id: newId() };
}

/** Splice parsed blocks (markdown paste) into the document at the caret:
    the current text block splits, parsed blocks land between the halves,
    and empty halves are dropped. Returns the id to focus (last inserted). */
export function insertParsedBlocks(
  doc: BlockDocument,
  id: string,
  offset: number,
  blocks: Block[],
): { doc: BlockDocument; focusId: string } {
  if (!blocks.length) return { doc, focusId: id };
  const loc = findBlock(doc, id);
  if (!loc || !isTextBlock(loc.block)) return { doc, focusId: id };

  const { doc: split, focusId: tailId } = splitTextBlock(doc, id, offset);
  let next = split;
  let anchor = id;
  for (const b of blocks) {
    next = insertAfter(next, anchor, b);
    anchor = b.id;
  }
  // Drop empty halves left by the split.
  const head = findBlock(next, id);
  if (head && isTextBlock(head.block) && head.block.md === '') next = removeBlock(next, id);
  const tail = findBlock(next, tailId);
  if (tail && isTextBlock(tail.block) && tail.block.md === '' && tailId !== id)
    next = removeBlock(next, tailId);
  return { doc: next, focusId: anchor };
}

/** Toggle a todo list item. */
export function toggleTodo(doc: BlockDocument, id: string, checked: boolean): BlockDocument {
  const loc = findBlock(doc, id);
  if (loc?.block.type !== 'list_item') return doc;
  return updateBlock(doc, id, { checked });
}
