/* The ':xx' emoji autocomplete popup. Completion inserts the full
   `:shortcode:` text — storage stays shortcode-canonical; rendering resolves
   to unicode. */
import { For, Show, createMemo } from 'solid-js';
import { searchEmoji } from './emoji';
import { updateBlock, findBlock } from './ops';
import { useBlocks } from './context';

export function EmojiPopup(props: { blockId: string }) {
  const ctx = useBlocks();
  const state = () => ctx.emojiPop();
  const hits = createMemo(() => {
    const s = state();
    return s && s.blockId === props.blockId ? searchEmoji(s.query, ctx.emoji()) : [];
  });

  const apply = (code: string) => {
    const s = state();
    if (!s) return;
    const loc = findBlock(ctx.doc(), props.blockId);
    if (!loc || !('md' in loc.block)) return;
    const md = loc.block.md;
    const end = s.colonIdx + 1 + s.query.length;
    const next = `${md.slice(0, s.colonIdx)}:${code}:${md.slice(end)}`;
    ctx.dispatch(updateBlock(ctx.doc(), props.blockId, { md: next }));
    ctx.setEmojiPop(null);
    ctx.focusBlock(props.blockId, s.colonIdx + code.length + 2);
  };

  return (
    <Show when={state()?.blockId === props.blockId && hits().length > 0}>
      <div class="fpop fbk-pop" role="listbox">
        <For each={hits()}>
          {(hit, i) => (
            <button
              type="button"
              class="fbk-pop-item"
              classList={{ 'is-active': i() === state()!.index }}
              role="option"
              aria-selected={i() === state()!.index}
              onMouseDown={(e) => {
                e.preventDefault();
                apply(hit.code);
              }}
              onMouseEnter={() => ctx.setEmojiPop({ ...state()!, index: i() })}
            >
              <span class="fbk-pop-emoji">{hit.char}</span>
              <span class="fbk-pop-hint">:{hit.code}:</span>
            </button>
          )}
        </For>
      </div>
    </Show>
  );
}
