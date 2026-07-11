/* <BlockEditor> — the fully-controlled block page editor. The parent owns
   the document: every op computes a fresh document and reports it through
   onChange (the kanban idiom). Block lists render with <Index> (not <For>):
   ops rebuild the edited block object each keystroke and <For> would remount
   the row, destroying the focused textarea. */
import { Index, Show, createSignal } from 'solid-js';
import { BlocksContext } from './context';
import type { EditorCtx, EmojiPopState, FocusState, SlashState } from './context';
import { toggleTodo } from './blockops';
import { insertAfter } from './ops';
import { BlockRenderer, listNumbers } from './render';
import { BlockRow } from './blockrow';
import type { Block, BlockDef, BlockDocument } from './types';
import { createBlock } from './types';

export interface BlockEditorProps {
  document: BlockDocument;
  onChange: (next: BlockDocument) => void;
  customBlocks?: Record<string, BlockDef>;
  /** Extra emoji shortcodes merged over the builtin table. */
  emoji?: Record<string, string>;
  readOnly?: boolean;
  /** Empty-document hint. */
  placeholder?: string;
  class?: string;
}

export function BlockEditor(props: BlockEditorProps) {
  const [focus, setFocus] = createSignal<FocusState | null>(null);
  const [slash, setSlash] = createSignal<SlashState | null>(null);
  const [emojiPop, setEmojiPop] = createSignal<EmojiPopState | null>(null);
  const refs = new Map<string, HTMLTextAreaElement>();

  const dispatch = (next: BlockDocument) => {
    if (next !== props.document) props.onChange(next);
  };

  const focusBlock = (id: string, caret?: number | 'end') => {
    setFocus({ id, caret: caret ?? 'end' });
    // The textarea may only exist after Solid applies the focus/doc change.
    const place = () => {
      const el = refs.get(id);
      if (!el) return false;
      el.focus();
      if (caret !== undefined) {
        const at = caret === 'end' ? el.value.length : Math.min(caret, el.value.length);
        el.setSelectionRange(at, at);
      }
      return true;
    };
    queueMicrotask(() => {
      if (!place()) requestAnimationFrame(place);
    });
  };

  const ctx: EditorCtx = {
    doc: () => props.document,
    dispatch,
    focus,
    focusBlock,
    blur: () => {
      setFocus(null);
      setSlash(null);
      setEmojiPop(null);
    },
    registerRef: (id, el) => {
      if (el) refs.set(id, el);
      else refs.delete(id);
    },
    refOf: (id) => refs.get(id),
    customBlocks: () => props.customBlocks ?? {},
    emoji: () => props.emoji,
    render: {
      get customBlocks() {
        return props.customBlocks;
      },
      get emoji() {
        return props.emoji;
      },
      onToggleTodo: (id, checked) => dispatch(toggleTodo(props.document, id, checked)),
    },
    slash,
    setSlash,
    emojiPop,
    setEmojiPop,
    placeholder: () => props.placeholder ?? "Type '/' for blocks",
  };

  /** Click in the trailing space appends a paragraph (or reuses a trailing
      empty one). */
  const appendParagraph = () => {
    const blocks = props.document.blocks;
    const last = blocks[blocks.length - 1];
    if (last && last.type === 'paragraph' && last.md === '') {
      focusBlock(last.id, 0);
      return;
    }
    const p = createBlock('paragraph');
    if (last) dispatch(insertAfter(props.document, last.id, p));
    else dispatch({ ...props.document, blocks: [p] });
    focusBlock(p.id, 0);
  };

  return (
    <Show
      when={!props.readOnly}
      fallback={
        <BlockRenderer
          document={props.document}
          customBlocks={props.customBlocks}
          emoji={props.emoji}
          class={props.class}
        />
      }
    >
      <BlocksContext.Provider value={ctx}>
        <div class={`fbk fbk-editor ${props.class ?? ''}`}>
          <EditableBlockList blocks={props.document.blocks} placeholder={ctx.placeholder()} />
          <div class="fbk-append" onClick={appendParagraph} aria-hidden="true" />
        </div>
      </BlocksContext.Provider>
    </Show>
  );
}

/** A sibling list of editable rows — also hosts each column's blocks. */
export function EditableBlockList(props: { blocks: Block[]; placeholder?: string }) {
  const numbers = () => listNumbers(props.blocks);
  const showPlaceholder = () =>
    props.placeholder !== undefined &&
    props.blocks.length === 1 &&
    props.blocks[0]!.type === 'paragraph' &&
    props.blocks[0]!.md === '';
  return (
    <Index each={props.blocks}>
      {(block) => (
        <BlockRow
          block={block}
          num={numbers().get(block().id)}
          placeholder={showPlaceholder() ? props.placeholder : undefined}
        />
      )}
    </Index>
  );
}
