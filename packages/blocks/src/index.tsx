/* @forge/blocks — Notion-style block page editor for the Forge design system.
   CSS import order (app entry): @forge/tokens/tokens.css → @forge/tokens/base.css
   → @forge/ui/styles.css → @forge/code/styles.css → @forge/blocks/styles.css.

   The document JSON is the cross-platform interchange shared with
   crates/forge-blocks (TUI + egui editors) — see that crate's tests for the
   frozen fixtures. */
export * from './types';
export * from './ops';
export { cloneBlock, insertParsedBlocks, toggleTodo } from './blockops';
export { detectShortcut, blockToMarkdown } from './line';
export type { ShortcutHit } from './line';
export { toMarkdown, fromMarkdown } from './serialize';
export { EMOJI, resolveEmoji, searchEmoji } from './emoji';
export { InlineMd } from './inline';
export type { InlineMdProps } from './inline';
export { BlockRenderer, StaticBlock, codeLanguage } from './render';
export type { BlockRendererProps, RenderCtx } from './render';
export { BlockEditor } from './editor';
export type { BlockEditorProps } from './editor';
