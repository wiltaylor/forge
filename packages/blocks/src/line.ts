/* Line-start markdown shortcuts and block → markdown-line rendering.
   Mirrors crates/forge-blocks ops::line_start_shortcut / convert.rs. */
import type { AdmonitionTone, Block, BlockData } from './types';

export interface ShortcutHit {
  /** Replacement block (without id — the caller keeps the block's id). */
  block: BlockData;
  /** Bytes consumed by the prefix; the caret moves back by this much. */
  prefixLen: number;
}

const TONES: AdmonitionTone[] = ['info', 'success', 'warning', 'danger'];

/** Detect a markdown shortcut typed at the start of a paragraph:
    `# `..`#### `, `- `/`* `, `1. `/`1) `, `- [ ] `/`- [x] `/`[] `, `> `,
    ``` ```lang ```, `---`, `:::info` (and the other tones). */
export function detectShortcut(text: string): ShortcutHit | null {
  for (const [p, checked] of [['- [ ] ', false], ['- [x] ', true], ['[] ', false]] as const) {
    if (text.startsWith(p))
      return {
        block: { type: 'list_item', style: 'todo', checked, indent: 0, md: text.slice(p.length) },
        prefixLen: p.length,
      };
  }
  const heading = /^(#{1,4}) /.exec(text);
  if (heading)
    return {
      block: {
        type: 'heading',
        level: heading[1]!.length as 1 | 2 | 3 | 4,
        md: text.slice(heading[0].length),
      },
      prefixLen: heading[0].length,
    };
  for (const p of ['- ', '* ']) {
    if (text.startsWith(p))
      return {
        block: { type: 'list_item', style: 'bullet', indent: 0, md: text.slice(p.length) },
        prefixLen: p.length,
      };
  }
  for (const p of ['1. ', '1) ']) {
    if (text.startsWith(p))
      return {
        block: { type: 'list_item', style: 'number', indent: 0, md: text.slice(p.length) },
        prefixLen: p.length,
      };
  }
  if (text.startsWith('> '))
    return { block: { type: 'quote', md: text.slice(2) }, prefixLen: 2 };
  if (text.startsWith('```') && /^[a-z0-9]*$/i.test(text.slice(3)))
    return { block: { type: 'code', lang: text.slice(3), code: '' }, prefixLen: text.length };
  if (text === '---') return { block: { type: 'divider' }, prefixLen: 3 };
  if (text.startsWith(':::')) {
    const tone = text.slice(3) as AdmonitionTone;
    if (TONES.includes(tone))
      return {
        block: { type: 'admonition', tone, title: '', md: '' },
        prefixLen: text.length,
      };
  }
  return null;
}

const TONE_TAG: Record<AdmonitionTone, string> = {
  info: 'INFO',
  success: 'SUCCESS',
  warning: 'WARNING',
  danger: 'DANGER',
};

export function toneFromTag(tag: string): AdmonitionTone | null {
  switch (tag.toUpperCase()) {
    case 'INFO':
    case 'NOTE':
    case 'TIP':
      return 'info';
    case 'SUCCESS':
      return 'success';
    case 'WARNING':
    case 'IMPORTANT':
      return 'warning';
    case 'DANGER':
    case 'CAUTION':
      return 'danger';
    default:
      return null;
  }
}

/** One block as markdown lines. `num` numbers ordered-list runs. */
export function blockToMarkdown(b: Block, ctx: { num: number }): string {
  switch (b.type) {
    case 'paragraph':
      return b.md;
    case 'heading':
      return `${'#'.repeat(b.level)} ${b.md}`;
    case 'list_item': {
      const pad = '  '.repeat(b.indent);
      if (b.style === 'todo') return `${pad}- [${b.checked ? 'x' : ' '}] ${b.md}`;
      if (b.style === 'number') return `${pad}${++ctx.num}. ${b.md}`;
      return `${pad}- ${b.md}`;
    }
    case 'quote':
      return b.md
        .split('\n')
        .map((l) => `> ${l}`)
        .join('\n');
    case 'divider':
      return '---';
    case 'code':
      return `\`\`\`${b.lang}\n${b.code}\n\`\`\``;
    case 'table': {
      const row = (cells: string[]) => `| ${cells.join(' | ')} |`;
      return [row(b.header), row(b.header.map(() => '---')), ...b.rows.map(row)].join('\n');
    }
    case 'admonition': {
      const head = `> [!${TONE_TAG[b.tone]}]${b.title ? ` ${b.title}` : ''}`;
      return [head, ...b.md.split('\n').map((l) => `> ${l}`)].join('\n');
    }
    case 'columns':
      // Lossy: columns flatten sequentially.
      return b.columns
        .flatMap((c) => c.blocks)
        .map((child) => blockToMarkdown(child, ctx))
        .join('\n\n');
    case 'custom':
      return `\`\`\`block:${b.kind}\n${JSON.stringify(b.data, null, 2)}\n\`\`\``;
  }
}
