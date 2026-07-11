/* The shared text-block editor: an auto-growing textarea showing the raw
   inline-markdown source, carrying the whole keyboard model (split/merge,
   block navigation, shortcuts, slash + emoji popups). Autosize uses the CSS
   grid-replica trick (.fbk-grow::after mirrors the value) — no JS resize. */
import { Show, createEffect } from 'solid-js';
import { detectShortcut } from './line';
import {
  mergeWithPrevious, moveBlock, nextEditable, prevEditable,
  replaceBlock, setListIndent, splitTextBlock, updateBlock,
} from './ops';
import { fromMarkdown } from './serialize';
import { insertParsedBlocks } from './blockops';
import { searchEmoji } from './emoji';
import { useBlocks } from './context';
import { applySlashItem, slashItems, slashQuery } from './slash';
import type { TextBlock } from './types';
import { isTextBlock } from './types';

export interface TextBlockEditProps {
  block: () => TextBlock;
  /** Ordinal for number-style list items. */
  num?: number;
  placeholder?: string;
}

export function TextBlockEdit(props: TextBlockEditProps) {
  const ctx = useBlocks();
  let ta!: HTMLTextAreaElement;
  let grow!: HTMLDivElement;
  const id = () => props.block().id;

  /* External value → textarea; compare-before-write keeps typing safe. */
  createEffect(() => {
    const md = props.block().md;
    if (ta.value !== md) ta.value = md;
    grow.dataset.value = md;
  });

  const commit = (value: string) => {
    ctx.dispatch(updateBlock(ctx.doc(), id(), { md: value }));
  };

  const trackEmoji = (value: string, caret: number) => {
    const before = value.slice(0, caret);
    const m = /:([a-z0-9_+-]{2,})$/.exec(before);
    if (m && !ctx.slash()) {
      ctx.setEmojiPop({
        blockId: id(),
        query: m[1]!,
        colonIdx: caret - m[0].length,
        index: 0,
      });
    } else if (ctx.emojiPop()?.blockId === id()) {
      ctx.setEmojiPop(null);
    }
  };

  const onInput = () => {
    const value = ta.value;
    grow.dataset.value = value;
    const b = props.block();

    // Line-start shortcuts convert paragraphs only (Notion behavior).
    if (b.type === 'paragraph' && !ctx.slash()) {
      const hit = detectShortcut(value);
      if (hit) {
        ctx.dispatch(replaceBlock(ctx.doc(), id(), { id: id(), ...hit.block } as TextBlock));
        const caret = Math.max(0, ta.selectionStart - hit.prefixLen);
        ctx.focusBlock(id(), caret);
        return;
      }
    }
    commit(value);

    // '/' on an (previously) empty paragraph opens the slash menu.
    if (b.type === 'paragraph' && value === '/' && !ctx.slash()) {
      ctx.setSlash({ blockId: id(), index: 0 });
      ctx.setEmojiPop(null);
      return;
    }
    // Slash menu tracks the query typed after '/'; leaving the pattern closes it.
    if (ctx.slash()?.blockId === id()) {
      if (!value.startsWith('/')) ctx.setSlash(null);
      else ctx.setSlash({ blockId: id(), index: 0 });
      return;
    }
    trackEmoji(value, ta.selectionStart);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.isComposing) return;
    const doc = ctx.doc();
    const sel = ta.selectionStart;
    const collapsed = ta.selectionStart === ta.selectionEnd;

    // Popup navigation eats keys first.
    const slash = ctx.slash();
    if (slash?.blockId === id()) {
      const items = slashItems(ctx, id());
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const d = e.key === 'ArrowDown' ? 1 : -1;
        ctx.setSlash({ ...slash, index: (slash.index + d + items.length) % Math.max(1, items.length) });
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        const item = items[slash.index];
        if (item) applySlashItem(ctx, id(), item);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        ctx.setSlash(null);
        return;
      }
    }
    const pop = ctx.emojiPop();
    if (pop?.blockId === id()) {
      const hits = searchEmoji(pop.query, ctx.emoji());
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const d = e.key === 'ArrowDown' ? 1 : -1;
        ctx.setEmojiPop({ ...pop, index: (pop.index + d + hits.length) % Math.max(1, hits.length) });
        return;
      }
      if ((e.key === 'Enter' || e.key === 'Tab') && hits.length) {
        e.preventDefault();
        const hit = hits[pop.index] ?? hits[0]!;
        const value = ta.value;
        const end = pop.colonIdx + 1 + pop.query.length;
        const next = `${value.slice(0, pop.colonIdx)}:${hit.code}:${value.slice(end)}`;
        ctx.dispatch(updateBlock(doc, id(), { md: next }));
        ctx.setEmojiPop(null);
        ctx.focusBlock(id(), pop.colonIdx + hit.code.length + 2);
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        ctx.setEmojiPop(null);
        return;
      }
    }

    // Alt+arrows move the block within its sibling list.
    if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      e.preventDefault();
      ctx.dispatch(moveBlock(doc, id(), e.key === 'ArrowUp' ? -1 : 1));
      ctx.focusBlock(id(), sel);
      return;
    }

    switch (e.key) {
      case 'Enter': {
        if (e.shiftKey) return; // native newline = soft break
        e.preventDefault();
        const { doc: next, focusId } = splitTextBlock(doc, id(), sel);
        ctx.dispatch(next);
        ctx.setSlash(null);
        ctx.setEmojiPop(null);
        ctx.focusBlock(focusId, 0);
        return;
      }
      case 'Backspace': {
        if (sel !== 0 || !collapsed) return;
        e.preventDefault();
        const b = props.block();
        if (b.type !== 'paragraph') {
          // The shared rule: first Backspace turns the block into a paragraph.
          ctx.dispatch(replaceBlock(doc, id(), { id: id(), type: 'paragraph', md: b.md }));
          ctx.focusBlock(id(), 0);
          return;
        }
        const r = mergeWithPrevious(doc, id());
        if (r) {
          ctx.dispatch(r.doc);
          ctx.focusBlock(r.focusId, r.caret);
        }
        return;
      }
      case 'Delete': {
        if (!collapsed || sel !== ta.value.length) return;
        const next = nextEditable(doc, id());
        if (next?.block.type === 'paragraph') {
          e.preventDefault();
          const r = mergeWithPrevious(doc, next.block.id);
          if (r) {
            ctx.dispatch(r.doc);
            ctx.focusBlock(r.focusId, r.caret);
          }
        }
        return;
      }
      case 'ArrowUp': {
        if (ta.value.lastIndexOf('\n', sel - 1) !== -1) return; // not on first line
        const prev = prevEditable(doc, id());
        if (!prev) return;
        e.preventDefault();
        ctx.focusBlock(prev.block.id, isTextBlock(prev.block) ? 'end' : undefined);
        return;
      }
      case 'ArrowDown': {
        if (ta.value.indexOf('\n', sel) !== -1) return; // not on last line
        const next = nextEditable(doc, id());
        if (!next) return;
        e.preventDefault();
        ctx.focusBlock(next.block.id, isTextBlock(next.block) ? 0 : undefined);
        return;
      }
      case 'Tab': {
        if (props.block().type === 'list_item') {
          e.preventDefault();
          ctx.dispatch(setListIndent(doc, id(), e.shiftKey ? -1 : 1));
        }
        return;
      }
      case 'Escape': {
        ctx.setSlash(null);
        ctx.setEmojiPop(null);
        ta.blur();
        ctx.blur();
        return;
      }
    }
  };

  /* Multi-block markdown paste splices parsed blocks at the caret. */
  const onPaste = (e: ClipboardEvent) => {
    const text = e.clipboardData?.getData('text/plain') ?? '';
    if (!text.includes('\n')) return; // plain inline paste stays native
    const parsed = fromMarkdown(text);
    if (parsed.blocks.length === 1 && parsed.blocks[0]!.type === 'paragraph') return;
    e.preventDefault();
    const { doc: next, focusId } = insertParsedBlocks(
      ctx.doc(),
      id(),
      ta.selectionStart,
      parsed.blocks,
    );
    ctx.dispatch(next);
    ctx.focusBlock(focusId, 'end');
  };

  const b = props.block;
  return (
    <div
      class="fbk-textedit"
      classList={{
        'fbk-textedit-quote': b().type === 'quote',
        'fbk-textedit-li': b().type === 'list_item',
      }}
    >
      <Show when={b().type === 'list_item' && b()}>
        {(li) => {
          const item = li as () => Extract<TextBlock, { type: 'list_item' }>;
          return (
            <span class="fbk-li-marker" style={{ '--fbk-indent': item().indent }}>
              <Show
                when={item().style === 'todo'}
                fallback={item().style === 'number' ? `${props.num ?? 1}.` : '•'}
              >
                <input
                  type="checkbox"
                  checked={item().checked}
                  onChange={(ev) =>
                    ctx.dispatch(updateBlock(ctx.doc(), id(), { checked: ev.currentTarget.checked }))
                  }
                />
              </Show>
            </span>
          );
        }}
      </Show>
      <div class="fbk-grow" ref={grow} data-value={b().md}>
        <textarea
          ref={(el) => {
            ta = el;
            ctx.registerRef(id(), el);
          }}
          class={taClass(b())}
          rows="1"
          spellcheck={false}
          placeholder={props.placeholder}
          value={b().md}
          onInput={onInput}
          onKeyDown={onKeyDown}
          onPaste={onPaste}
          onBlur={() => {
            // Popups own their dismissal; plain blur just closes them.
            queueMicrotask(() => {
              if (ctx.slash()?.blockId === id()) ctx.setSlash(null);
              if (ctx.emojiPop()?.blockId === id()) ctx.setEmojiPop(null);
            });
          }}
        />
      </div>
    </div>
  );
}

function taClass(b: TextBlock): string {
  let cls = 'fbk-ta';
  if (b.type === 'heading') cls += ` fbk-ta-h${b.level}`;
  return cls;
}
