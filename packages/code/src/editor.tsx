/* ---------------- CodeEditor ------------------------------------------------- */
import { createSignal, createEffect, onMount, onCleanup } from 'solid-js';
import type { JSX } from 'solid-js';
import { EditorState, Compartment } from '@codemirror/state';
import {
  EditorView, keymap, lineNumbers as lineNumbersExt, placeholder as placeholderExt,
  highlightActiveLine, highlightActiveLineGutter, drawSelection,
} from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching } from '@codemirror/language';
import { setDiagnostics, lintGutter } from '@codemirror/lint';
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
import { forgeTheme } from './theme';
import type { LanguageInput } from './languages';
import { resolveLanguage, toDiagnostics, CodeMenu } from './internal';
import type { CodeAnnotation, CodeMenuItem, MenuPos } from './internal';

export interface CodeEditorProps {
  value?: string;
  /** Omitted (with readOnly unset) = read-only editor. */
  onChange?: (value: string) => void;
  language?: LanguageInput;
  readOnly?: boolean;
  /** Default true. */
  lineNumbers?: boolean;
  placeholder?: string;
  /** Soft-wrap long lines. */
  wrap?: boolean;
  /** LSP-style annotations: 1-based line, 0-based col. */
  annotations?: CodeAnnotation[];
  /** Enables the Forge context menu when non-empty. */
  contextMenuItems?: CodeMenuItem[];
  /** CSS height (default '240px'). */
  height?: string;
  class?: string;
  style?: JSX.CSSProperties;
}

export function CodeEditor(props: CodeEditorProps) {
  let host!: HTMLDivElement;
  let view: EditorView | undefined;
  const lang = new Compartment(), editable = new Compartment(), wrapping = new Compartment();
  const [menu, setMenu] = createSignal<MenuPos | null>(null);
  const isRO = () => props.readOnly || !props.onChange;
  const roExt = () => [EditorState.readOnly.of(isRO()), EditorView.editable.of(!isRO())];

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: props.value ?? '',
        extensions: [
          ...forgeTheme,
          props.lineNumbers !== false ? [lineNumbersExt(), highlightActiveLineGutter()] : [],
          highlightActiveLine(), drawSelection(), bracketMatching(),
          lintGutter(), history(), highlightSelectionMatches(),
          keymap.of([...defaultKeymap, ...searchKeymap, ...historyKeymap, indentWithTab]),
          props.placeholder ? placeholderExt(props.placeholder) : [],
          lang.of(resolveLanguage(props.language)),
          editable.of(roExt()),
          wrapping.of(props.wrap ? EditorView.lineWrapping : []),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) props.onChange?.(u.state.doc.toString());
          }),
          EditorView.domEventHandlers({
            contextmenu: (e) => {
              if (!props.contextMenuItems?.length) return false;
              e.preventDefault();
              setMenu({ x: e.clientX, y: e.clientY });
              return true;
            },
          }),
        ],
      }),
    });
  });
  onCleanup(() => view?.destroy());

  /* External value → view; compare-before-dispatch keeps the loop safe. */
  createEffect(() => {
    const next = props.value ?? '';
    if (view && next !== view.state.doc.toString())
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } });
  });
  /* Annotations → lint diagnostics (re-mapped when the doc swaps). */
  createEffect(() => {
    const anns = props.annotations ?? [];
    props.value;
    if (view) view.dispatch(setDiagnostics(view.state, toDiagnostics(view.state, anns)));
  });
  createEffect(() => view?.dispatch({ effects: editable.reconfigure(roExt()) }));
  createEffect(() => view?.dispatch({ effects: lang.reconfigure(resolveLanguage(props.language)) }));
  createEffect(() => view?.dispatch({ effects: wrapping.reconfigure(props.wrap ? EditorView.lineWrapping : []) }));

  return (
    <div class={`fcode ${props.class ?? ''}`}
         style={{ height: props.height ?? '240px', ...props.style }} ref={host}>
      <CodeMenu pos={menu} setPos={setMenu} items={props.contextMenuItems} view={() => view} />
    </div>
  );
}
