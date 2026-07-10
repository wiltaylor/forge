/* Zero-dep markdown parser — produces a plain AST rendered by markdown.tsx.
   Safe by construction: raw HTML is never interpreted (it stays literal text)
   and link/image URLs pass safeUrl() or degrade to plain text. */

export type MdInline =
  | { t: 'text'; text: string }
  | { t: 'br' }
  | { t: 'strong'; children: MdInline[] }
  | { t: 'em'; children: MdInline[] }
  | { t: 'strike'; children: MdInline[] }
  | { t: 'code'; text: string }
  | { t: 'link'; href: string; children: MdInline[] }
  | { t: 'image'; src: string; alt: string };

export interface MdListItem {
  children: MdInline[];
  task?: boolean;
  checked?: boolean;
}

export type MdBlock =
  | { t: 'p'; children: MdInline[] }
  | { t: 'heading'; level: 1 | 2 | 3 | 4; children: MdInline[] }
  | { t: 'code'; lang: string; text: string }
  | { t: 'list'; ordered: boolean; items: MdListItem[] }
  | { t: 'quote'; children: MdBlock[] }
  | { t: 'hr' }
  | { t: 'table'; head: MdInline[][]; rows: MdInline[][][] };

/** Allow http/https/mailto (relative URLs resolve to https and pass). */
export function safeUrl(raw: string): string | null {
  try {
    const u = new URL(raw, 'https://relative.invalid');
    return u.protocol === 'http:' || u.protocol === 'https:' || u.protocol === 'mailto:' ? raw : null;
  } catch {
    return null;
  }
}

/* ---------------- Inline pass ------------------------------------------------ */
const EM_DEPTH_MAX = 4;

interface InlineMatch {
  idx: number;
  len: number;
  nodes: MdInline[];
}

function earliestMatch(s: string, depth: number): InlineMatch | null {
  let best: InlineMatch | null = null;
  /* First consider() at a given index wins — call order is priority order. */
  const consider = (idx: number, len: number, nodes: MdInline[]) => {
    if (best === null || idx < best.idx) best = { idx, len, nodes };
  };
  let m: RegExpExecArray | null;

  if ((m = /`([^`]+)`/.exec(s)))
    consider(m.index, m[0].length, [{ t: 'code', text: m[1]! }]);
  if ((m = /!\[([^\]]*)\]\(((?:[^()\s]|\([^()\s]*\))+)\)/.exec(s))) {
    const src = safeUrl(m[2]!);
    consider(m.index, m[0].length,
      src ? [{ t: 'image', src, alt: m[1]! }] : [{ t: 'text', text: m[1]! }]);
  }
  if ((m = /\[([^\]]+)\]\(((?:[^()\s]|\([^()\s]*\))+)\)/.exec(s))) {
    const href = safeUrl(m[2]!);
    const children = parseInline(m[1]!, depth + 1);
    consider(m.index, m[0].length,
      href ? [{ t: 'link', href, children }] : children);
  }
  if ((m = /\*\*(.+?)\*\*/.exec(s)))
    consider(m.index, m[0].length, [{ t: 'strong', children: parseInline(m[1]!, depth + 1) }]);
  if ((m = /~~(.+?)~~/.exec(s)))
    consider(m.index, m[0].length, [{ t: 'strike', children: parseInline(m[1]!, depth + 1) }]);
  if ((m = /\*([^*\s](?:[^*]*[^*\s])?)\*|\b_([^_\s](?:[^_]*[^_\s])?)_\b/.exec(s)))
    consider(m.index, m[0].length, [{ t: 'em', children: parseInline((m[1] ?? m[2])!, depth + 1) }]);
  if ((m = /https?:\/\/[^\s<>]+/.exec(s))) {
    const url = m[0].replace(/[.,;:!?)\]'"]+$/, '');
    if (url.length > 'https://'.length)
      consider(m.index, url.length, [{ t: 'link', href: url, children: [{ t: 'text', text: url }] }]);
  }
  return best;
}

export function parseInline(src: string, depth = 0): MdInline[] {
  if (!src) return [];
  if (depth >= EM_DEPTH_MAX) return [{ t: 'text', text: src }];
  const out: MdInline[] = [];
  let rest = src;
  while (rest.length) {
    const m = earliestMatch(rest, depth);
    if (!m) {
      out.push({ t: 'text', text: rest });
      break;
    }
    if (m.idx > 0) out.push({ t: 'text', text: rest.slice(0, m.idx) });
    out.push(...m.nodes);
    rest = rest.slice(m.idx + m.len);
  }
  return out;
}

/* ---------------- Block pass -------------------------------------------------- */
const LIST_RE = /^\s*([-*]|\d+[.)])\s+(.*)$/;

function splitRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|')) s = s.slice(0, -1);
  return s.split('|').map((c) => c.trim());
}

/** Paragraph lines join with soft breaks. */
function inlineLines(lines: string[]): MdInline[] {
  const out: MdInline[] = [];
  lines.forEach((l, i) => {
    if (i) out.push({ t: 'br' });
    out.push(...parseInline(l.trim()));
  });
  return out;
}

function parseBlocks(lines: string[]): MdBlock[] {
  const blocks: MdBlock[] = [];
  let para: string[] = [];
  const flush = () => {
    if (para.length) {
      blocks.push({ t: 'p', children: inlineLines(para) });
      para = [];
    }
  };

  let i = 0;
  while (i < lines.length) {
    const line = lines[i]!;

    const fence = /^```(.*)$/.exec(line);
    if (fence) {
      flush();
      const buf: string[] = [];
      i++;
      while (i < lines.length && !/^```\s*$/.test(lines[i]!)) buf.push(lines[i++]!);
      i++; // closing fence (unclosed: rest of input is code)
      blocks.push({ t: 'code', lang: fence[1]!.trim(), text: buf.join('\n') });
      continue;
    }
    if (!line.trim()) {
      flush();
      i++;
      continue;
    }
    const heading = /^(#{1,4})\s+(.*)$/.exec(line);
    if (heading) {
      flush();
      blocks.push({
        t: 'heading',
        level: heading[1]!.length as 1 | 2 | 3 | 4,
        children: parseInline(heading[2]!.trim()),
      });
      i++;
      continue;
    }
    if (/^(?:-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
      flush();
      blocks.push({ t: 'hr' });
      i++;
      continue;
    }
    if (/^>\s?/.test(line)) {
      flush();
      const buf: string[] = [];
      while (i < lines.length && /^>\s?/.test(lines[i]!)) buf.push(lines[i++]!.replace(/^>\s?/, ''));
      blocks.push({ t: 'quote', children: parseBlocks(buf) });
      continue;
    }
    const li = LIST_RE.exec(line);
    if (li) {
      flush();
      const ordered = /\d/.test(li[1]!.charAt(0));
      const items: MdListItem[] = [];
      while (i < lines.length) {
        const m = LIST_RE.exec(lines[i]!);
        if (!m || /\d/.test(m[1]!.charAt(0)) !== ordered) break;
        const task = !ordered && /^\[([ xX])\]\s+(.*)$/.exec(m[2]!);
        if (task) items.push({ task: true, checked: task[1] !== ' ', children: parseInline(task[2]!) });
        else items.push({ children: parseInline(m[2]!) });
        i++;
      }
      blocks.push({ t: 'list', ordered, items });
      continue;
    }
    if (line.includes('|') && i + 1 < lines.length) {
      const sep = splitRow(lines[i + 1]!);
      if (sep.length > 1 && sep.every((c) => /^:?-+:?$/.test(c))) {
        flush();
        const head = splitRow(line).map((c) => parseInline(c));
        i += 2;
        const rows: MdInline[][][] = [];
        while (i < lines.length && lines[i]!.includes('|'))
          rows.push(splitRow(lines[i++]!).map((c) => parseInline(c)));
        blocks.push({ t: 'table', head, rows });
        continue;
      }
    }
    para.push(line);
    i++;
  }
  flush();
  return blocks;
}

export function parseMarkdown(src: string): MdBlock[] {
  return parseBlocks((src ?? '').replace(/\r\n?/g, '\n').split('\n'));
}
