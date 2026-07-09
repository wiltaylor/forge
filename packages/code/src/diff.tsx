/* ---------------- DiffEditor ------------------------------------------------- */
import { createEffect, onMount, onCleanup } from 'solid-js';
import type { JSX } from 'solid-js';
import { EditorState, Compartment } from '@codemirror/state';
import { EditorView, lineNumbers as lineNumbersExt } from '@codemirror/view';
import { setDiagnostics } from '@codemirror/lint';
import { MergeView, unifiedMergeView } from '@codemirror/merge';
import { forgeTheme } from './theme';
import type { LanguageName } from './languages';
import { resolveLanguage, toDiagnostics } from './internal';
import type { CodeAnnotation } from './internal';

export interface DiffEditorProps {
  original?: string;
  modified?: string;
  /** Omitted = read-only modified side. */
  onChange?: (value: string) => void;
  language?: LanguageName | (string & {});
  /** Default true. */
  lineNumbers?: boolean;
  /** Soft-wrap long lines. */
  wrap?: boolean;
  /** Single-pane unified diff instead of side-by-side. */
  unified?: boolean;
  /** LSP-style annotations on the modified side. */
  annotations?: CodeAnnotation[];
  /** CSS height (default '280px'). */
  height?: string;
  class?: string;
  style?: JSX.CSSProperties;
}

export function DiffEditor(props: DiffEditorProps) {
  let host!: HTMLDivElement;
  let mv: MergeView | null = null;
  let uview: EditorView | null = null;

  const shared = () => [
    ...forgeTheme,
    props.lineNumbers !== false ? lineNumbersExt() : [],
    props.wrap ? EditorView.lineWrapping : [],
    lang0.of(resolveLanguage(props.language)),
  ];
  const lang0 = new Compartment();

  const build = () => {
    mv?.destroy(); uview?.destroy(); mv = uview = null;
    host.textContent = '';
    if (props.unified) {
      uview = new EditorView({
        parent: host,
        state: EditorState.create({
          doc: props.modified ?? '',
          extensions: [
            ...shared(),
            unifiedMergeView({ original: props.original ?? '' }),
            EditorState.readOnly.of(!props.onChange),
            EditorView.editable.of(!!props.onChange),
            EditorView.updateListener.of((u) => {
              if (u.docChanged) props.onChange?.(u.state.doc.toString());
            }),
          ],
        }),
      });
    } else {
      mv = new MergeView({
        parent: host,
        a: {
          doc: props.original ?? '',
          extensions: [...shared(), EditorState.readOnly.of(true), EditorView.editable.of(false)],
        },
        b: {
          doc: props.modified ?? '',
          extensions: [
            ...shared(),
            EditorState.readOnly.of(!props.onChange),
            EditorView.editable.of(!!props.onChange),
            EditorView.updateListener.of((u) => {
              if (u.docChanged) props.onChange?.(u.state.doc.toString());
            }),
          ],
        },
      });
    }
  };

  onMount(() => {
    build();
    /* rebuild when the layout mode flips */
    createEffect((prev: boolean | undefined) => {
      const mode = !!props.unified;
      if (prev !== undefined && mode !== prev) build();
      return mode;
    });
    /* external doc updates, compare-before-dispatch */
    createEffect(() => {
      const a = props.original ?? '';
      const av = mv?.a;
      if (av && a !== av.state.doc.toString())
        av.dispatch({ changes: { from: 0, to: av.state.doc.length, insert: a } });
    });
    createEffect(() => {
      const b = props.modified ?? '';
      const bv = mv?.b ?? uview;
      if (bv && b !== bv.state.doc.toString())
        bv.dispatch({ changes: { from: 0, to: bv.state.doc.length, insert: b } });
    });
    /* annotations on the modified side */
    createEffect(() => {
      const anns = props.annotations ?? [];
      const bv = mv?.b ?? uview;
      if (bv) bv.dispatch(setDiagnostics(bv.state, toDiagnostics(bv.state, anns)));
    });
  });
  onCleanup(() => { mv?.destroy(); uview?.destroy(); });

  return (
    <div class={`fcode fcode-diff ${props.class ?? ''}`}
         style={{ height: props.height ?? '280px', ...props.style }} ref={host} />
  );
}
