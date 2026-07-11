import { describe, it, expect } from 'vitest';
import { fromMarkdown, toMarkdown } from '../src/serialize';
import { detectShortcut } from '../src/line';
import type { Block, BlockDocument } from '../src/types';
import { DOC_VERSION } from '../src/types';

const doc = (...blocks: Block[]): BlockDocument => ({ version: DOC_VERSION, blocks });
let n = 0;
const id = () => `t${n++}`;

describe('detectShortcut', () => {
  it('hits every prefix', () => {
    expect(detectShortcut('# T')!.block).toMatchObject({ type: 'heading', level: 1, md: 'T' });
    expect(detectShortcut('#### T')!.block).toMatchObject({ type: 'heading', level: 4 });
    expect(detectShortcut('- x')!.block).toMatchObject({ type: 'list_item', style: 'bullet' });
    expect(detectShortcut('* x')!.block).toMatchObject({ type: 'list_item', style: 'bullet' });
    expect(detectShortcut('1. x')!.block).toMatchObject({ type: 'list_item', style: 'number' });
    expect(detectShortcut('- [ ] x')!.block).toMatchObject({
      type: 'list_item', style: 'todo', checked: false,
    });
    expect(detectShortcut('- [x] x')!.block).toMatchObject({ checked: true });
    expect(detectShortcut('> q')!.block).toMatchObject({ type: 'quote', md: 'q' });
    expect(detectShortcut('```rust')!.block).toMatchObject({ type: 'code', lang: 'rust' });
    expect(detectShortcut('---')!.block).toMatchObject({ type: 'divider' });
    expect(detectShortcut(':::warning')!.block).toMatchObject({
      type: 'admonition', tone: 'warning',
    });
  });
  it('rejects non-shortcuts', () => {
    for (const s of ['#x', '-x', '##### five', ':::nope', 'plain', '--', '```rust x'])
      expect(detectShortcut(s)).toBeNull();
  });
  it('caret math: prefixLen strips the prefix', () => {
    const hit = detectShortcut('## Title here')!;
    expect(hit.prefixLen).toBe(3);
    expect((hit.block as { md: string }).md).toBe('Title here');
  });
});

describe('markdown round-trip', () => {
  const rich = doc(
    { id: id(), type: 'heading', level: 1, md: 'Title :rocket:' },
    { id: id(), type: 'paragraph', md: 'Body with **bold** and\na soft break' },
    { id: id(), type: 'list_item', style: 'bullet', indent: 0, md: 'one' },
    { id: id(), type: 'list_item', style: 'bullet', indent: 1, md: 'nested' },
    { id: id(), type: 'list_item', style: 'number', indent: 0, md: 'first' },
    { id: id(), type: 'list_item', style: 'number', indent: 0, md: 'second' },
    { id: id(), type: 'list_item', style: 'todo', checked: true, indent: 0, md: 'done' },
    { id: id(), type: 'quote', md: 'wisdom' },
    { id: id(), type: 'divider' },
    { id: id(), type: 'code', lang: 'ts', code: 'const a = 1;\nconst b = 2;' },
    { id: id(), type: 'table', header: ['A', 'B'], rows: [['1', '**2**']] },
    { id: id(), type: 'admonition', tone: 'danger', title: 'Stop', md: 'reason' },
    { id: id(), type: 'custom', kind: 'stat', data: { label: 'Reqs', value: 42 } },
  );

  it('round-trips structure and content', () => {
    const back = fromMarkdown(toMarkdown(rich));
    expect(back.blocks.map((b) => b.type)).toEqual(rich.blocks.map((b) => b.type));
    for (let i = 0; i < rich.blocks.length; i++) {
      const { id: _a, ...want } = rich.blocks[i]! as Block & { id: string };
      const { id: _b, ...got } = back.blocks[i]! as Block & { id: string };
      expect(got).toEqual(want);
    }
  });

  it('numbered runs restart after non-list blocks', () => {
    const md = toMarkdown(
      doc(
        { id: id(), type: 'list_item', style: 'number', indent: 0, md: 'a' },
        { id: id(), type: 'list_item', style: 'number', indent: 0, md: 'b' },
        { id: id(), type: 'paragraph', md: 'gap' },
        { id: id(), type: 'list_item', style: 'number', indent: 0, md: 'c' },
      ),
    );
    expect(md).toContain('1. a');
    expect(md).toContain('2. b');
    expect(md.trimEnd().endsWith('1. c')).toBe(true);
  });

  it('columns flatten (documented lossy)', () => {
    const d = doc({
      id: id(),
      type: 'columns',
      columns: [
        { ratio: 0.5, blocks: [{ id: id(), type: 'paragraph', md: 'left' }] },
        { ratio: 0.5, blocks: [{ id: id(), type: 'paragraph', md: 'right' }] },
      ],
    });
    const back = fromMarkdown(toMarkdown(d));
    expect(back.blocks.map((b) => b.type)).toEqual(['paragraph', 'paragraph']);
  });

  it('github alert tags map to tones', () => {
    const back = fromMarkdown('> [!NOTE] Heads\n> body\n\n> [!CAUTION]\n> boom\n');
    expect(back.blocks[0]).toMatchObject({ type: 'admonition', tone: 'info', title: 'Heads' });
    expect(back.blocks[1]).toMatchObject({ type: 'admonition', tone: 'danger', title: '' });
  });

  it('plain quotes stay quotes', () => {
    const back = fromMarkdown('> just quoting\n> two lines\n');
    expect(back.blocks[0]).toMatchObject({ type: 'quote', md: 'just quoting\ntwo lines' });
  });

  it('malformed custom payloads degrade to null data', () => {
    const back = fromMarkdown('```block:widget\nnot json\n```\n');
    expect(back.blocks[0]).toMatchObject({ type: 'custom', kind: 'widget', data: null });
  });

  it('empty input yields one empty paragraph', () => {
    const back = fromMarkdown('');
    expect(back.blocks).toHaveLength(1);
    expect(back.blocks[0]).toMatchObject({ type: 'paragraph', md: '' });
  });
});
