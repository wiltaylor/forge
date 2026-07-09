/* Shared private helpers for CodeEditor/DiffEditor: language resolution,
   annotation → diagnostic mapping, and the Forge context menu. */

import { Show, For, createEffect, onCleanup } from 'solid-js';
import type { Accessor, JSX, Setter } from 'solid-js';
import { Portal } from 'solid-js/web';
import type { EditorState, Extension } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import type { Diagnostic } from '@codemirror/lint';
import { LANGUAGES } from './languages';

export const resolveLanguage = (lang: string | undefined): Extension => {
  const factory = lang ? (LANGUAGES as Record<string, (() => Extension) | undefined>)[lang] : undefined;
  return factory ? factory() : [];
};

/* ---------------- annotation mapping ---------------------------------------- */
/** A position in the document: 1-based line, 0-based col (LSP convention). */
export interface AnnotationPos {
  line: number;
  col?: number;
}

/** LSP-style annotation rendered as a lint diagnostic. */
export interface CodeAnnotation {
  from: AnnotationPos;
  /** Omitted = a 1-character range starting at `from`. */
  to?: AnnotationPos;
  /** Default 'info'. */
  severity?: 'error' | 'warning' | 'info' | 'hint';
  message: string;
  source?: string;
}

export const toDiagnostics = (state: EditorState, anns: CodeAnnotation[] | undefined): Diagnostic[] => {
  const doc = state.doc;
  const pos = (p: AnnotationPos) => {
    const ln = doc.line(Math.min(Math.max(1, p.line), doc.lines));
    return Math.min(ln.from + Math.max(0, p.col ?? 0), ln.to);
  };
  return (anns ?? []).map((a) => {
    const from = pos(a.from);
    const to = a.to ? Math.max(pos(a.to), from) : Math.min(from + 1, doc.length);
    return { from, to, severity: a.severity ?? 'info', message: a.message, source: a.source };
  });
};

/** Item shape for CodeEditor's `contextMenuItems`. */
export interface CodeMenuItem {
  label?: JSX.Element;
  kbd?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  onSelect?: (view: EditorView | undefined) => void;
}

export interface MenuPos {
  x: number;
  y: number;
}

export interface CodeMenuProps {
  pos: Accessor<MenuPos | null>;
  setPos: Setter<MenuPos | null>;
  items?: CodeMenuItem[];
  view: () => EditorView | undefined;
}

/* Forge context menu rendered at the pointer (uses round-4 .fmenu classes). */
export function CodeMenu(props: CodeMenuProps) {
  createEffect(() => {
    if (!props.pos()) return;
    const close = () => props.setPos(null);
    const onDown = (e: PointerEvent) => { if (!(e.target as Element).closest('.fcode-menu')) close(); };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };
    document.addEventListener('pointerdown', onDown);
    document.addEventListener('keydown', onKey);
    onCleanup(() => {
      document.removeEventListener('pointerdown', onDown);
      document.removeEventListener('keydown', onKey);
    });
  });
  return (
    <Show when={props.pos()}>
      <Portal>
        <div class="fpop fmenu-pop fcode-menu" role="menu"
             style={{
               left: `${Math.min(props.pos()!.x, window.innerWidth - 200)}px`,
               top: `${Math.min(props.pos()!.y, window.innerHeight - 40 * (props.items?.length ?? 1))}px`,
             }}>
          <For each={props.items}>
            {(item) => (
              <Show when={!item.separator} fallback={<div class="fmenu-sep" role="separator" />}>
                <button type="button" class="fmenu-item" role="menuitem" disabled={item.disabled}
                        classList={{ 'is-danger': !!item.danger, 'is-disabled': !!item.disabled }}
                        onClick={() => { props.setPos(null); item.onSelect?.(props.view()); }}>
                  <span class="fmenu-label">{item.label}</span>
                  <Show when={item.kbd}>
                    <span class="fmenu-kbd">{item.kbd}</span>
                  </Show>
                </button>
              </Show>
            )}
          </For>
        </div>
      </Portal>
    </Show>
  );
}
