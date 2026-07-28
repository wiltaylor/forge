/* Whole-document markdown import/export. JSON is the canonical interchange;
   markdown is clipboard interop and export. Lossy: columns flatten, custom
   blocks travel as ```block:<kind> JSON fences. Mirrors forge-blocks
   convert.rs so all platforms exchange the same markdown. */
import type { Block, BlockDocument } from './types';
import { DATA_TYPES, DOC_VERSION, createBlock, newId } from './types';
import { blockToMarkdown, toneFromTag } from './line';

/** Builtin types that may travel as ```forge:<type> fences. */
const FENCE_TYPES = new Set<string>([...DATA_TYPES, 'footnote']);

/** Rebuild a builtin block from a ```forge:<type> fence body; on any
    parse/shape failure degrade to a code block so content is never lost. */
function parseForgeFence(typeName: string, body: string): Block {
  const fallback: Block = { id: newId(), type: 'code', lang: `forge:${typeName}`, code: body };
  if (!FENCE_TYPES.has(typeName)) return fallback;
  try {
    const fields: unknown = JSON.parse(body);
    if (!fields || typeof fields !== 'object' || Array.isArray(fields)) return fallback;
    return { ...(fields as object), id: newId(), type: typeName } as Block;
  } catch {
    return fallback;
  }
}

/** A line that is exactly one `![alt](src)` image reference. */
function imageLine(l: string): Block | null {
  const m = /^!\[([^\]]*)\]\(([^()]*)\)$/.exec(l);
  if (!m) return null;
  return { id: newId(), type: 'image', src: m[2]!, alt: m[1]! };
}

/** A `[^label]: body` footnote definition line. */
function footnoteLine(l: string): Block | null {
  const m = /^\[\^([^\]\s]+)\]: (.*)$/.exec(l);
  if (!m) return null;
  return { id: newId(), type: 'footnote', label: m[1]!, md: m[2]! };
}

export function toMarkdown(doc: BlockDocument): string {
  const out: string[] = [];
  const ctx = { num: 0 };
  let prevList = false;
  for (const b of doc.blocks) {
    const isList = b.type === 'list_item';
    if (!isList) ctx.num = 0;
    const text = blockToMarkdown(b, ctx);
    if (out.length) out.push(prevList && isList ? '\n' : '\n\n');
    out.push(text);
    prevList = isList;
  }
  return out.join('') + '\n';
}

/** Parse markdown into a document — a line scanner that keeps raw inline
    source (the AST parser can't give the source back). Also powers paste. */
export function fromMarkdown(text: string): BlockDocument {
  const lines = text.replace(/\r\n?/g, '\n').split('\n');
  const blocks: Block[] = [];
  let i = 0;

  const heading = (l: string) => /^(#{1,4}) (.*)$/.exec(l);
  const listItem = (l: string): Block | null => {
    const spaces = l.length - l.trimStart().length;
    const indent = Math.min(5, Math.floor(spaces / 2));
    const rest = l.slice(spaces);
    const todo = /^- \[([ xX])\] (.*)$/.exec(rest);
    if (todo)
      return {
        id: newId(),
        type: 'list_item',
        style: 'todo',
        checked: todo[1] !== ' ',
        indent,
        md: todo[2]!,
      };
    const bullet = /^[-*+] (.*)$/.exec(rest);
    if (bullet)
      return { id: newId(), type: 'list_item', style: 'bullet', indent, md: bullet[1]! };
    const num = /^\d+[.)] (.*)$/.exec(rest);
    if (num) return { id: newId(), type: 'list_item', style: 'number', indent, md: num[1]! };
    return null;
  };
  const splitRow = (l: string) =>
    l
      .trim()
      .replace(/^\|/, '')
      .replace(/\|$/, '')
      .split('|')
      .map((c) => c.trim());
  const isSeparator = (l: string) =>
    l.startsWith('|') && /^[|\-: ]+$/.test(l) && l.includes('-');

  while (i < lines.length) {
    const line = lines[i]!;
    const trimmed = line.trimStart();
    if (!trimmed) {
      i++;
      continue;
    }

    // Fenced code (plain or ```block:<kind> custom payload).
    if (trimmed.startsWith('```')) {
      const info = trimmed.slice(3).trim();
      const body: string[] = [];
      i++;
      while (i < lines.length && !lines[i]!.trimStart().startsWith('```')) body.push(lines[i++]!);
      i++; // closing fence
      if (info.startsWith('forge:')) {
        blocks.push(parseForgeFence(info.slice(6), body.join('\n')));
      } else if (info.startsWith('block:')) {
        let data: unknown = null;
        try {
          data = JSON.parse(body.join('\n'));
        } catch {
          /* malformed payload stays null */
        }
        blocks.push({ id: newId(), type: 'custom', kind: info.slice(6), data });
      } else {
        blocks.push({ id: newId(), type: 'code', lang: info, code: body.join('\n') });
      }
      continue;
    }

    // Math ($$ … $$).
    if (trimmed === '$$') {
      const body: string[] = [];
      i++;
      while (i < lines.length && lines[i]!.trim() !== '$$') body.push(lines[i++]!);
      i++; // closing $$
      blocks.push({ id: newId(), type: 'math', tex: body.join('\n') });
      continue;
    }

    const img = imageLine(trimmed);
    if (img) {
      blocks.push(img);
      i++;
      continue;
    }

    const fn = footnoteLine(trimmed);
    if (fn) {
      blocks.push(fn);
      i++;
      continue;
    }

    const h = heading(trimmed);
    if (h) {
      blocks.push({
        id: newId(),
        type: 'heading',
        level: h[1]!.length as 1 | 2 | 3 | 4,
        md: h[2]!,
      });
      i++;
      continue;
    }

    if (/^(?:-{3,}|\*{3,}|_{3,})\s*$/.test(trimmed)) {
      blocks.push({ id: newId(), type: 'divider' });
      i++;
      continue;
    }

    // Blockquote group → admonition (first line `[!TONE] Title`) or quote.
    if (trimmed.startsWith('>')) {
      const body: string[] = [];
      while (i < lines.length && lines[i]!.trimStart().startsWith('>')) {
        body.push(lines[i]!.trimStart().replace(/^> ?/, ''));
        i++;
      }
      const alert = /^\[!([a-zA-Z]+)\]\s*(.*)$/.exec(body[0] ?? '');
      const tone = alert ? toneFromTag(alert[1]!) : null;
      if (alert && tone) {
        blocks.push({
          id: newId(),
          type: 'admonition',
          tone,
          title: alert[2]!,
          md: body.slice(1).join('\n'),
        });
      } else {
        blocks.push({ id: newId(), type: 'quote', md: body.join('\n') });
      }
      continue;
    }

    const li = listItem(line);
    if (li) {
      blocks.push(li);
      i++;
      continue;
    }

    if (trimmed.startsWith('|') && i + 1 < lines.length && isSeparator(lines[i + 1]!.trim())) {
      const header = splitRow(trimmed);
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && lines[i]!.trimStart().startsWith('|'))
        rows.push(splitRow(lines[i++]!));
      blocks.push({ id: newId(), type: 'table', header, rows });
      continue;
    }

    // Paragraph: consecutive plain lines join with soft breaks.
    const para = [line.trimEnd()];
    i++;
    while (i < lines.length) {
      const l = lines[i]!;
      const t = l.trimStart();
      if (
        !t ||
        t.startsWith('```') ||
        t.startsWith('>') ||
        t.startsWith('|') ||
        heading(t) ||
        listItem(l) ||
        /^(?:-{3,}|\*{3,}|_{3,})\s*$/.test(t) ||
        t === '$$' ||
        imageLine(t) ||
        footnoteLine(t)
      )
        break;
      para.push(l.trimEnd());
      i++;
    }
    blocks.push({ id: newId(), type: 'paragraph', md: para.join('\n') });
  }

  return { version: DOC_VERSION, blocks: blocks.length ? blocks : [createBlock('paragraph')] };
}
