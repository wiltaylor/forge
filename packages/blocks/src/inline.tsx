/* Inline markdown rendering: parseInline (from @forge/ui) → emoji resolution
   on text nodes only (never code spans) → JSX. XSS-safe by construction:
   raw HTML stays literal text and URLs pass safeUrl or degrade. */
import { For, createMemo } from 'solid-js';
import type { JSX } from 'solid-js';
import { parseInline } from '@forge/ui';
import type { MdInline } from '@forge/ui';
import { resolveEmoji } from './emoji';

export interface InlineMdProps {
  md: string;
  emoji?: Record<string, string>;
  /** Where links open. Default '_blank'. */
  linkTarget?: '_blank' | '_self';
}

/** One text block's inline content. `\n` renders as a soft break. */
export function InlineMd(props: InlineMdProps) {
  const nodes = createMemo(() => {
    const out: MdInline[] = [];
    props.md.split('\n').forEach((line, i) => {
      if (i) out.push({ t: 'br' });
      out.push(...parseInline(line));
    });
    return withEmoji(out, props.emoji);
  });
  return renderInlines(nodes(), props.linkTarget ?? '_blank');
}

function withEmoji(nodes: MdInline[], extra?: Record<string, string>): MdInline[] {
  return nodes.map((n) => {
    switch (n.t) {
      case 'text':
        return { ...n, text: resolveEmoji(n.text, extra) };
      case 'strong':
      case 'em':
      case 'strike':
        return { ...n, children: withEmoji(n.children, extra) };
      case 'link':
        return { ...n, children: withEmoji(n.children, extra) };
      default:
        return n; // code spans and images stay untouched
    }
  });
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
      return <code class="fbk-icode">{n.text}</code>;
    case 'strong':
      return <strong>{renderInlines(n.children, target)}</strong>;
    case 'em':
      return <em>{renderInlines(n.children, target)}</em>;
    case 'strike':
      return <s>{renderInlines(n.children, target)}</s>;
    case 'link':
      return (
        <a href={n.href} target={target} rel="noopener noreferrer">
          {renderInlines(n.children, target)}
        </a>
      );
    case 'image':
      return <img class="fbk-img" src={n.src} alt={n.alt} loading="lazy" />;
  }
}
