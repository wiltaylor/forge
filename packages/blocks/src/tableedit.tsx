/* Focused-table editing: a grid of per-cell inputs over the raw inline-md
   strings, with a small row/column toolbar. Unfocused tables render through
   StaticBlock. */
import { For, createSignal, onMount } from 'solid-js';
import { Button } from '@forge/ui';
import {
  tableInsertCol, tableInsertRow, tableRemoveCol, tableRemoveRow, tableSetCell,
} from './ops';
import { useBlocks } from './context';
import type { Block } from './types';

type TableBlock = Extract<Block, { type: 'table' }>;

export interface TableEditProps {
  block: () => TableBlock;
  /** Cell to focus on entry; row -1 = header. */
  initialCell?: { row: number; col: number };
}

export function TableEdit(props: TableEditProps) {
  const ctx = useBlocks();
  const id = () => props.block().id;
  /** Last focused cell — the row/col the toolbar acts on. */
  const [cell, setCell] = createSignal(props.initialCell ?? { row: -1, col: 0 });
  let root!: HTMLDivElement;

  onMount(() => {
    const c = cell();
    const el = root.querySelector<HTMLInputElement>(
      `input[data-row="${c.row}"][data-col="${c.col}"]`,
    );
    el?.focus();
  });

  const set = (row: number, col: number, value: string) =>
    ctx.dispatch(tableSetCell(ctx.doc(), id(), row, col, value));

  const focusCell = (row: number, col: number) => {
    queueMicrotask(() => {
      root
        .querySelector<HTMLInputElement>(`input[data-row="${row}"][data-col="${col}"]`)
        ?.focus();
    });
  };

  const onKeyDown = (e: KeyboardEvent, row: number, col: number) => {
    const t = props.block();
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.ctrlKey || row === t.rows.length - 1) {
        ctx.dispatch(tableInsertRow(ctx.doc(), id(), row + 1));
      }
      focusCell(row + 1, col);
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      ctx.blur();
      return;
    }
    if (e.key === 'ArrowUp' && row > -1) {
      e.preventDefault();
      focusCell(row - 1, col);
      return;
    }
    if (e.key === 'ArrowDown' && row < t.rows.length - 1) {
      e.preventDefault();
      focusCell(row + 1, col);
    }
  };

  const cellInput = (row: number, col: number, value: string) => (
    <input
      class="fbk-cell"
      classList={{ 'fbk-cell-head': row === -1 }}
      data-row={row}
      data-col={col}
      value={value}
      spellcheck={false}
      onInput={(e) => set(row, col, e.currentTarget.value)}
      onFocus={() => setCell({ row, col })}
      onKeyDown={(e) => onKeyDown(e, row, col)}
    />
  );

  return (
    <div class="fbk-tableedit" ref={root}>
      <div class="fbk-tablegrid" style={{ '--fbk-cols': props.block().header.length }}>
        <For each={props.block().header}>{(h, c) => cellInput(-1, c(), h)}</For>
        <For each={props.block().rows}>
          {(row, r) => <For each={row}>{(v, c) => cellInput(r(), c(), v)}</For>}
        </For>
      </div>
      <div class="fbk-tabletools">
        <Button size="sm" variant="ghost" onClick={() => ctx.dispatch(tableInsertRow(ctx.doc(), id(), cell().row + 1))}>
          + Row
        </Button>
        <Button size="sm" variant="ghost" onClick={() => ctx.dispatch(tableInsertCol(ctx.doc(), id(), cell().col + 1))}>
          + Col
        </Button>
        <Button size="sm" variant="ghost" onClick={() => ctx.dispatch(tableRemoveRow(ctx.doc(), id(), Math.max(0, cell().row)))}>
          − Row
        </Button>
        <Button size="sm" variant="ghost" onClick={() => ctx.dispatch(tableRemoveCol(ctx.doc(), id(), cell().col))}>
          − Col
        </Button>
      </div>
    </div>
  );
}
