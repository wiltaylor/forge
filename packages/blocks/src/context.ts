/* Editor context shared by the block rows, popups, and nested column lists.
   Kept in its own module so blockrow/columns/editor can import it without
   cycles at module-init time. */
import { createContext, useContext } from 'solid-js';
import type { Accessor } from 'solid-js';
import type { BlockDef, BlockDocument } from './types';
import type { RenderCtx } from './render';

export interface FocusState {
  id: string;
  caret: number | 'end';
}

export interface SlashState {
  blockId: string;
  index: number;
}

export interface EmojiPopState {
  blockId: string;
  /** Shortcode prefix typed after the colon. */
  query: string;
  /** Byte index of the opening ':' in the block's md. */
  colonIdx: number;
  index: number;
}

export interface EditorCtx {
  doc: () => BlockDocument;
  dispatch: (next: BlockDocument) => void;
  focus: Accessor<FocusState | null>;
  /** Focus a block; text blocks also place the caret. */
  focusBlock: (id: string, caret?: number | 'end') => void;
  blur: () => void;
  registerRef: (id: string, el: HTMLTextAreaElement | null) => void;
  refOf: (id: string) => HTMLTextAreaElement | undefined;
  customBlocks: () => Record<string, BlockDef>;
  emoji: () => Record<string, string> | undefined;
  render: RenderCtx;
  slash: Accessor<SlashState | null>;
  setSlash: (s: SlashState | null) => void;
  emojiPop: Accessor<EmojiPopState | null>;
  setEmojiPop: (s: EmojiPopState | null) => void;
  placeholder: () => string;
}

export const BlocksContext = createContext<EditorCtx>();

export function useBlocks(): EditorCtx {
  const ctx = useContext(BlocksContext);
  if (!ctx) throw new Error('useBlocks must be used inside <BlockEditor>');
  return ctx;
}
