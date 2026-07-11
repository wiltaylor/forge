import { For, Show, createMemo, mergeProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { parseMarkdown } from '@forge/ui';
import type { MdBlock, MdInline } from '@forge/ui';

/* ---------------- Markdown --------------------------------------------------- */
/* Standalone rendered-markdown control (also used by chat text blocks).
   Zero-dep subset: headings #–####, paragraphs, fenced code (+lang label),
   ul/ol + task lists, blockquote, hr, pipe tables, images, **bold**, *em*,
   ~~strike~~, `code`, [links](url), bare-URL autolinks. Raw HTML renders as
   literal text; only http/https/mailto URLs become links. No syntax
   highlighting — use @forge/code for that. */
export interface MarkdownProps {
  text: string;
  /** Where links open. Default '_blank'. */
  linkTarget?: '_blank' | '_self';
  class?: string;
}

export function Markdown(props: MarkdownProps) {
  const merged = mergeProps({ linkTarget: '_blank' as const }, props);
  const blocks = createMemo(() => parseMarkdown(merged.text));
  return (
    <div class={`fmd ${merged.class ?? ''}`}>
      <For each={blocks()}>{(b) => renderBlock(b, merged.linkTarget)}</For>
    </div>
  );
}

function renderBlock(b: MdBlock, target: '_blank' | '_self'): JSX.Element {
  switch (b.t) {
    case 'p':
      return <p>{renderInlines(b.children, target)}</p>;
    case 'heading':
      return <Dynamic component={`h${b.level}`}>{renderInlines(b.children, target)}</Dynamic>;
    case 'code':
      return (
        <pre class="fmd-code" data-lang={b.lang || undefined}><code>{b.text}</code></pre>
      );
    case 'list':
      return (
        <Dynamic component={b.ordered ? 'ol' : 'ul'}>
          <For each={b.items}>
            {(item) => (
              <li classList={{ 'fmd-task': !!item.task }}>
                <Show when={item.task}>
                  <input type="checkbox" checked={item.checked} disabled aria-hidden="true" />
                </Show>
                {renderInlines(item.children, target)}
              </li>
            )}
          </For>
        </Dynamic>
      );
    case 'quote':
      return <blockquote class="fmd-quote"><For each={b.children}>{(c) => renderBlock(c, target)}</For></blockquote>;
    case 'hr':
      return <hr class="fmd-hr" />;
    case 'table':
      return (
        <div class="fmd-table-wrap">
          <table class="fmd-table">
            <thead>
              <tr><For each={b.head}>{(cell) => <th>{renderInlines(cell, target)}</th>}</For></tr>
            </thead>
            <tbody>
              <For each={b.rows}>
                {(row) => <tr><For each={row}>{(cell) => <td>{renderInlines(cell, target)}</td>}</For></tr>}
              </For>
            </tbody>
          </table>
        </div>
      );
  }
}

function renderInlines(nodes: MdInline[], target: '_blank' | '_self'): JSX.Element {
  return <For each={nodes}>{(n) => renderInline(n, target)}</For>;
}

function renderInline(n: MdInline, target: '_blank' | '_self'): JSX.Element {
  switch (n.t) {
    case 'text':
      return n.text;
    case 'br':
      return <br />;
    case 'code':
      return <code class="fmd-icode">{n.text}</code>;
    case 'strong':
      return <strong>{renderInlines(n.children, target)}</strong>;
    case 'em':
      return <em>{renderInlines(n.children, target)}</em>;
    case 'strike':
      return <s>{renderInlines(n.children, target)}</s>;
    case 'link':
      return <a href={n.href} target={target} rel="noopener noreferrer">{renderInlines(n.children, target)}</a>;
    case 'image':
      return <img class="fmd-img" src={n.src} alt={n.alt} loading="lazy" />;
  }
}
