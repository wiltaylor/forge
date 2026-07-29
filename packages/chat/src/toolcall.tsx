import { For, Show, createSignal, useContext } from 'solid-js';
import type { JSX } from 'solid-js';
import { Spinner } from '@forge/ui';
import type { ChatToolCallData } from './types';
import { ChevronRightSvg } from './internal/icons';
import { ToolExpansionContext, ToolKeyContext } from './internal/expansion';

/* Collapsible tool-call box for assistant transcripts. Nested `children`
   render recursively behind a left rail.

   Expansion is, in order of precedence: controlled by `open`/`onToggle` when
   both are given; else held by the registry ChatView provides, keyed by
   `tool.key` — that is what survives a transcript update rebuilding the box;
   else component-local, as it has always been. */
export interface ChatToolCallProps {
  tool: ChatToolCallData;
  /** Controlled expansion — pass with `onToggle`, or neither. */
  open?: boolean;
  /** Fires with the requested state when the header is clicked. */
  onToggle?: (open: boolean) => void;
}

export function ChatToolCall(props: ChatToolCallProps): JSX.Element {
  const registry = useContext(ToolExpansionContext);
  const inherited = useContext(ToolKeyContext);
  const [local, setLocal] = createSignal(!!props.tool.defaultOpen);

  const controlled = () => props.open !== undefined && props.onToggle !== undefined;
  /* Own key when the producer stamped one, else the one our parent derived. */
  const key = () => props.tool.key ?? inherited?.();
  const stored = () => {
    const k = key();
    return registry !== undefined && k !== undefined ? k : undefined;
  };

  const open = () => {
    if (controlled()) return !!props.open;
    const k = stored();
    return k === undefined ? local() : registry!.isOpen(k, !!props.tool.defaultOpen);
  };
  const toggle = () => {
    const next = !open();
    if (controlled()) return props.onToggle!(next);
    const k = stored();
    if (k === undefined) setLocal(next);
    else registry!.setOpen(k, next);
  };

  const childKey = (index: number) => {
    const k = key();
    return k === undefined ? undefined : `${k}/${index}`;
  };
  const hasBody = () =>
    props.tool.args !== undefined || props.tool.result !== undefined || !!props.tool.children?.length;

  return (
    <div class="fchat-tool" classList={{ 'is-open': open() }}>
      <button
        type="button"
        class="fchat-tool-head"
        aria-expanded={open()}
        disabled={!hasBody()}
        onClick={toggle}
      >
        <ChevronRightSvg />
        <span class="fchat-tool-name">{props.tool.name}</span>
        <Show when={props.tool.summary}>
          <span class="fchat-tool-summary">{props.tool.summary}</span>
        </Show>
        <span class={`fchat-tool-status is-${props.tool.status}`}>
          <Show when={props.tool.status === 'running'} fallback={<span class="fchat-tool-dot" />}>
            <Spinner size={12} label="Running" />
          </Show>
          {props.tool.status}
        </span>
      </button>
      <Show when={open() && hasBody()}>
        <div class="fchat-tool-body">
          <Show when={props.tool.args !== undefined}>
            <div class="eyebrow">Arguments</div>
            <ToolPayload value={props.tool.args} />
          </Show>
          <Show when={props.tool.result !== undefined}>
            <div class="eyebrow">Result</div>
            <ToolPayload value={props.tool.result} />
          </Show>
          <Show when={props.tool.children?.length}>
            <div class="fchat-tool-children">
              <For each={props.tool.children}>
                {(child, index) => (
                  <ToolKeyContext.Provider value={() => childKey(index())}>
                    <ChatToolCall tool={child} />
                  </ToolKeyContext.Provider>
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}

function ToolPayload(props: { value?: string | JSX.Element }) {
  return (
    <Show when={typeof props.value === 'string'} fallback={props.value}>
      <pre class="fmd-code"><code>{props.value as string}</code></pre>
    </Show>
  );
}
