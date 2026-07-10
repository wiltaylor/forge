/* ---------------- Languages ------------------------------------------------- */
import type { Extension } from '@codemirror/state';
import { StreamLanguage } from '@codemirror/language';
import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { json } from '@codemirror/lang-json';
import { css } from '@codemirror/lang-css';
import { html } from '@codemirror/lang-html';
import { shell } from '@codemirror/legacy-modes/mode/shell';

/** Built-in language keys for CodeEditor/DiffEditor's `language` prop. */
export type LanguageName = 'js' | 'jsx' | 'ts' | 'tsx' | 'python' | 'json' | 'css' | 'html' | 'shell';

/** What the `language` prop accepts: a built-in key, or any CodeMirror
    language `Extension` (e.g. a `StreamLanguage.define(...)` for a custom
    DSL). Unknown strings fall back to plain text. */
export type LanguageInput = LanguageName | (string & {}) | Extension;

export const LANGUAGES: Record<LanguageName, () => Extension> = {
  js: () => javascript(),
  jsx: () => javascript({ jsx: true }),
  ts: () => javascript({ typescript: true }),
  tsx: () => javascript({ typescript: true, jsx: true }),
  python: () => python(),
  json: () => json(),
  css: () => css(),
  html: () => html(),
  shell: () => StreamLanguage.define(shell),
};
