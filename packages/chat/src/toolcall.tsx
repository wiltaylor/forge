import { For, Show, createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { Spinner } from '@forge/ui';
import type { ChatToolCallData } from './types';
import { ChevronRightSvg } from './internal/icons';

/* Collapsible tool-call box for assistant transcripts. Nested `children`
   render recursively behind a left rail. */
export interface ChatToolCallProps {
  tool: ChatToolCallData;
}

export function ChatToolCall(props: ChatToolCallProps) {
  const [open, setOpen] = createSignal(!!props.tool.defaultOpen);
  const hasBody = () =>
    props.tool.args !== undefined || props.tool.result !== undefined || !!props.tool.children?.length;

  return (
    <div class="fchat-tool" classList={{ 'is-open': open() }}>
      <button
        type="button"
        class="fchat-tool-head"
        aria-expanded={open()}
        disabled={!hasBody()}
        onClick={() => setOpen((o) => !o)}
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
              <For each={props.tool.children}>{(child) => <ChatToolCall tool={child} />}</For>
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
