/* Focused editing for data blocks: the block's fields as raw JSON source in
   a CodeEditor (the source-per-block model applied to structured kinds).
   Parse-on-commit — invalid JSON keeps the draft and shows an error without
   ever clobbering the block. */
import { Show, createEffect, createSignal, on } from 'solid-js';
import type { JSX } from 'solid-js';
import { CodeEditor } from '@forge/code';
import { useBlocks } from './context';
import { replaceBlock } from './ops';
import type { Block, DataBlock } from './types';

export function JsonBlockEdit(props: { block: () => DataBlock }): JSX.Element {
  const ctx = useBlocks();
  const id = () => props.block().id;
  const source = () => {
    const { id: _id, type: _type, ...fields } = props.block();
    return JSON.stringify(fields, null, 2);
  };
  const [draft, setDraft] = createSignal(source());
  const [err, setErr] = createSignal<string | null>(null);

  // Rows render via <Index> and are reused across reorders — reset the draft
  // when this row starts showing a different block.
  createEffect(
    on(id, () => {
      setDraft(source());
      setErr(null);
    }, { defer: true }),
  );

  const commit = (): boolean => {
    try {
      const fields: unknown = JSON.parse(draft());
      if (!fields || typeof fields !== 'object' || Array.isArray(fields)) {
        throw new Error('expected a JSON object');
      }
      ctx.dispatch(
        replaceBlock(ctx.doc(), id(), {
          ...(fields as object),
          id: id(),
          type: props.block().type,
        } as Block),
      );
      setErr(null);
      return true;
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
      return false;
    }
  };

  return (
    <div
      class="fbk-code fbk-jsonedit"
      onKeyDown={(e) => {
        if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
          e.preventDefault();
          if (commit()) {
            (document.activeElement as HTMLElement | null)?.blur();
            ctx.blur();
          }
        }
      }}
      onFocusOut={(e) => {
        // Clicking elsewhere commits when valid; an invalid draft stays put.
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) commit();
      }}
    >
      <div class="fbk-codehead">
        <span class="fbk-jsonkind">{props.block().type}</span>
        <Show when={err()}>
          <span class="fbk-jsonerr">{err()}</span>
        </Show>
      </div>
      <CodeEditor
        value={draft()}
        onChange={setDraft}
        language="json"
        lineNumbers={false}
        height="auto"
      />
    </div>
  );
}
