/* Column layout editing: side-by-side nested block lists with draggable
   ratio grips (pointer-capture, the grid palette pattern) and per-column
   add/remove controls. */
import { For, Show } from 'solid-js';
import { addColumn, removeColumn, setColumnRatios } from './ops';
import { useBlocks } from './context';
import { EditableBlockList } from './editor';
import type { Block } from './types';

type ColumnsBlock = Extract<Block, { type: 'columns' }>;

export function ColumnsEdit(props: { block: () => ColumnsBlock }) {
  const ctx = useBlocks();
  const id = () => props.block().id;
  let root!: HTMLDivElement;

  const startDrag = (grip: number, down: PointerEvent) => {
    down.preventDefault();
    const target = down.currentTarget as HTMLElement;
    target.setPointerCapture(down.pointerId);
    const ratios = props.block().columns.map((c) => c.ratio);
    const total = root.offsetWidth;

    const move = (e: PointerEvent) => {
      const delta = (e.clientX - down.clientX) / total;
      const next = ratios.slice();
      next[grip] = Math.max(0.1, ratios[grip]! + delta);
      next[grip + 1] = Math.max(0.1, ratios[grip + 1]! - delta);
      ctx.dispatch(setColumnRatios(ctx.doc(), id(), next));
    };
    const up = () => {
      target.removeEventListener('pointermove', move);
      target.removeEventListener('pointerup', up);
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', up);
  };

  return (
    <div class="fbk-cols fbk-cols-edit" ref={root}>
      <For each={props.block().columns}>
        {(col, i) => (
          <>
            <Show when={i() > 0}>
              <div
                class="fbk-colgrip"
                role="separator"
                aria-orientation="vertical"
                onPointerDown={(e) => startDrag(i() - 1, e)}
              />
            </Show>
            <div class="fbk-col" style={{ 'flex-grow': col.ratio * 1000 }}>
              <div class="fbk-coltools">
                <button
                  type="button"
                  class="fbk-iconbtn"
                  title="Remove column"
                  aria-label="Remove column"
                  onClick={() => ctx.dispatch(removeColumn(ctx.doc(), id(), i()))}
                >
                  ×
                </button>
              </div>
              <EditableBlockList blocks={col.blocks} />
            </div>
          </>
        )}
      </For>
      <Show when={props.block().columns.length < 4}>
        <button
          type="button"
          class="fbk-coladd"
          title="Add column"
          onClick={() => ctx.dispatch(addColumn(ctx.doc(), id()))}
        >
          +
        </button>
      </Show>
    </div>
  );
}
