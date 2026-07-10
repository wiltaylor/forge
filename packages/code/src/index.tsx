/* Forge code editor — CodeMirror 6 wrappers over the code.css class layer.
   Import '@forge/tokens/tokens.css' then '@forge/code/styles.css' at app entry.

   Exports: CodeEditor, DiffEditor, LANGUAGES, forgeTheme.
   Annotation positions: 1-based line, 0-based col (LSP convention). */

export { forgeTheme } from './theme';
export { LANGUAGES } from './languages';
export type { LanguageName, LanguageInput } from './languages';
export type { AnnotationPos, CodeAnnotation, CodeMenuItem } from './internal';
export { CodeEditor } from './editor';
export type { CodeEditorProps } from './editor';
export { DiffEditor } from './diff';
export type { DiffEditorProps } from './diff';
