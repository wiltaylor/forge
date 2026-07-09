/* ---------------- Forge theme (every value a var(--token)) ------------------ */
import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

const forgeEditorTheme = EditorView.theme({
  '&': { background: 'var(--bg-0)', color: 'var(--fg-0)', fontSize: '12px', height: '100%' },
  '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.6' },
  '.cm-content': { caretColor: 'var(--accent)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--accent)' },
  '&.cm-focused': { outline: 'none' },
  '.cm-gutters': {
    background: 'var(--bg-0)', color: 'var(--fg-3)',
    borderRight: '1px solid var(--border-subtle)',
  },
  '.cm-activeLine': { background: 'var(--bg-1)' },
  '.cm-activeLineGutter': { background: 'var(--bg-1)', color: 'var(--fg-1)' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection':
    { background: 'var(--accent-bg) !important' },
  '.cm-selectionMatch': { background: 'var(--accent-bg)' },
  '.cm-matchingBracket': { background: 'var(--bg-3)', outline: '1px solid var(--border-strong)' },
  '.cm-panels': { background: 'var(--bg-1)', color: 'var(--fg-0)', border: '1px solid var(--border)' },
  '.cm-searchMatch': { background: 'var(--warning-bg)' },
  '.cm-searchMatch-selected': { background: 'var(--accent-bg)' },
  '.cm-tooltip': {
    background: 'var(--bg-4)', color: 'var(--fg-0)',
    border: '1px solid var(--border-strong)', borderRadius: 'var(--r-md)', fontSize: '12px',
  },
  '.cm-lintRange-error': { textDecoration: 'underline wavy var(--danger)' },
  '.cm-lintRange-warning': { textDecoration: 'underline wavy var(--warning)' },
  '.cm-lintRange-info': { textDecoration: 'underline wavy var(--info)' },
  '.cm-lintRange-hint': { textDecoration: 'underline dotted var(--fg-3)' },
  '.cm-lint-marker-error': { content: 'none', background: 'var(--danger)', borderRadius: '999px', width: '8px', height: '8px' },
  '.cm-lint-marker-warning': { content: 'none', background: 'var(--warning)', borderRadius: '999px', width: '8px', height: '8px' },
  '.cm-lint-marker-info': { content: 'none', background: 'var(--info)', borderRadius: '999px', width: '8px', height: '8px' },
  /* merge view */
  '.cm-mergeView': { background: 'var(--bg-0)' },
  '.cm-merge-a .cm-changedLine, .cm-deletedChunk': { background: 'var(--danger-bg)' },
  '.cm-merge-b .cm-changedLine, .cm-insertedLine': { background: 'var(--success-bg)' },
  '.cm-changedText': { background: 'color-mix(in oklab, var(--success) 30%, transparent)' },
  '.cm-deletedText': { background: 'color-mix(in oklab, var(--danger) 30%, transparent)' },
});

const forgeHighlight = HighlightStyle.define([
  { tag: [t.keyword, t.moduleKeyword, t.operatorKeyword], color: 'var(--accent-fg)' },
  { tag: [t.string, t.special(t.string)], color: 'var(--success-fg)' },
  { tag: [t.number, t.bool, t.atom, t.null], color: 'var(--info-fg)' },
  { tag: [t.comment, t.docComment], color: 'var(--fg-3)' },
  { tag: [t.typeName, t.className, t.namespace], color: 'var(--warning-fg)' },
  { tag: [t.propertyName, t.attributeName], color: 'var(--info-fg)' },
  { tag: [t.regexp, t.escape], color: 'var(--warning-fg)' },
  { tag: t.tagName, color: 'var(--accent-fg)' },
  { tag: t.variableName, color: 'var(--fg-0)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: 'var(--fg-0)', fontWeight: '500' },
  { tag: [t.meta, t.punctuation, t.operator], color: 'var(--fg-2)' },
  { tag: t.invalid, color: 'var(--danger-fg)' },
  { tag: t.heading, color: 'var(--fg-0)', fontWeight: '700' },
  { tag: [t.link, t.url], color: 'var(--accent-fg)', textDecoration: 'underline' },
]);

/** The Forge editor theme: pass to any CodeMirror EditorState's extensions. */
export const forgeTheme: Extension[] = [forgeEditorTheme, syntaxHighlighting(forgeHighlight)];
