import { createContext } from 'solid-js';
import type { Context } from 'solid-js';

/* Expand/collapse state for tool-call boxes, hoisted out of the box itself.
   `ChatToolCall` keeps its own signal when nothing provides this — but a
   transcript that rebuilds its rows (a new `items` array, a patched message)
   destroys the box and with it that signal, so `ChatView` owns a registry
   keyed by `ChatToolCallData.key` and the boxes read it through this
   context. */
export interface ToolExpansion {
  /** Current state for `key`; `defaultOpen` until the user touches it. */
  isOpen: (key: string, defaultOpen: boolean) => boolean;
  /** Record an explicit user toggle. */
  setOpen: (key: string, open: boolean) => void;
}

export const ToolExpansionContext: Context<ToolExpansion | undefined> =
  createContext<ToolExpansion>();

/* A nested call has no key of its own — its parent hands one down, derived
   from its own key and the child's index. */
export const ToolKeyContext: Context<(() => string | undefined) | undefined> = createContext<
  () => string | undefined
>();
