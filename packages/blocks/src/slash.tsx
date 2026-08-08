/* The '/' block palette: a small anchored .fpop list (not the full-screen
   Command palette). Query = the text typed after '/' in the block itself. */
import { For, Show, createMemo } from 'solid-js';
import type { JSX } from 'solid-js';
import { findBlock, replaceBlock, insertAfter, wrapInColumns } from './ops';
import { useBlocks } from './context';
import type { EditorCtx } from './context';
import type { Block, BlockData } from './types';
import { newId } from './types';
import { SLASH_BUILTINS } from './slash.gen';

export interface SlashItem {
  label: string;
  hint?: string;
  /** Builtin block to convert into / insert, or a wrap-in-columns action. */
  make?: () => BlockData;
  columns?: 2 | 3;
}

/** The filtered item list for the block's current query.

    The built-in rows are generated from the Rust kind registry, so this kit
    offers the same list in the same order as the TUI and egui editors. What is
    decided here is context: a block already inside a column cannot hold more
    columns, and only this kit knows which custom kinds the host registered. */
export function slashItems(ctx: EditorCtx, blockId: string): SlashItem[] {
  const q = slashQuery(ctx, blockId).toLowerCase();
  const loc = findBlock(ctx.doc(), blockId);
  const inColumn = loc?.parent.kind === 'column';

  const items: SlashItem[] = SLASH_BUILTINS.filter((b) => !(inColumn && b.columns)).map((b) => ({
    label: b.label,
    hint: b.hint,
    make: b.make,
    columns: b.columns,
  }));
  for (const [kind, def] of Object.entries(ctx.customBlocks())) {
    items.push({
      label: def.label,
      hint: kind,
      make: () => ({ type: 'custom', kind, data: def.create() }),
    });
  }
  return q ? items.filter((i) => i.label.toLowerCase().includes(q)) : items;
}

export function slashQuery(ctx: EditorCtx, blockId: string): string {
  const loc = findBlock(ctx.doc(), blockId);
  const md = loc && 'md' in loc.block ? loc.block.md : '';
  return md.startsWith('/') ? md.slice(1) : '';
}

/** Convert the (slash-typed) paragraph in place; columns wrap it instead. */
export function applySlashItem(ctx: EditorCtx, blockId: string, item: SlashItem): void {
  let doc = ctx.doc();
  ctx.setSlash(null);
  if (item.columns) {
    doc = replaceBlock(doc, blockId, { id: blockId, type: 'paragraph', md: '' });
    doc = wrapInColumns(doc, blockId, item.columns);
    ctx.dispatch(doc);
    ctx.focusBlock(blockId, 0);
    return;
  }
  if (!item.make) return;
  const made = item.make();
  const loc = findBlock(doc, blockId);
  if (loc && loc.block.type === 'paragraph') {
    ctx.dispatch(replaceBlock(doc, blockId, { id: blockId, ...made } as Block));
    ctx.focusBlock(blockId, 0);
  } else {
    const block = { id: newId(), ...made } as Block;
    ctx.dispatch(insertAfter(doc, blockId, block));
    ctx.focusBlock(block.id, 0);
  }
}

/** The anchored popup itself; rendered inside the focused block's row. */
export function SlashMenu(props: { blockId: string }): JSX.Element {
  const ctx = useBlocks();
  const state = () => ctx.slash();
  const items = createMemo(() => slashItems(ctx, props.blockId));
  return (
    <Show when={state()?.blockId === props.blockId && items().length > 0}>
      <div class="fpop fbk-pop" role="listbox">
        <For each={items()}>
          {(item, i) => (
            <button
              type="button"
              class="fbk-pop-item"
              classList={{ 'is-active': i() === state()!.index }}
              role="option"
              aria-selected={i() === state()!.index}
              onMouseDown={(e) => {
                e.preventDefault(); // keep the textarea focused
                applySlashItem(ctx, props.blockId, item);
              }}
              onMouseEnter={() => ctx.setSlash({ blockId: props.blockId, index: i() })}
            >
              <span>{item.label}</span>
              <Show when={item.hint}>
                <span class="fbk-pop-hint">{item.hint}</span>
              </Show>
            </button>
          )}
        </For>
      </div>
    </Show>
  );
}
