import { describe, it, expect } from 'vitest';
import { parseMarkdown, parseInline, safeUrl, type MdBlock } from '../src/md';

const p = (src: string) => parseMarkdown(src);

describe('safeUrl', () => {
  it('allows http/https/mailto and relative urls', () => {
    expect(safeUrl('https://a.dev/x')).toBe('https://a.dev/x');
    expect(safeUrl('http://a.dev')).toBe('http://a.dev');
    expect(safeUrl('mailto:a@b.c')).toBe('mailto:a@b.c');
    expect(safeUrl('/local/path.png')).toBe('/local/path.png');
  });
  it('rejects javascript:, data: and vbscript:', () => {
    expect(safeUrl('javascript:alert(1)')).toBeNull();
    expect(safeUrl('data:text/html,<script>')).toBeNull();
    expect(safeUrl('vbscript:x')).toBeNull();
    expect(safeUrl('JAVASCRIPT:alert(1)')).toBeNull();
  });
});

describe('inline parsing', () => {
  it('parses bold, italic, strike, code', () => {
    expect(parseInline('a **b** *c* ~~d~~ `e`')).toEqual([
      { t: 'text', text: 'a ' },
      { t: 'strong', children: [{ t: 'text', text: 'b' }] },
      { t: 'text', text: ' ' },
      { t: 'em', children: [{ t: 'text', text: 'c' }] },
      { t: 'text', text: ' ' },
      { t: 'strike', children: [{ t: 'text', text: 'd' }] },
      { t: 'text', text: ' ' },
      { t: 'code', text: 'e' },
    ]);
  });
  it('nests emphasis and caps recursion depth', () => {
    expect(parseInline('**a *b* c**')).toEqual([
      { t: 'strong', children: [
        { t: 'text', text: 'a ' },
        { t: 'em', children: [{ t: 'text', text: 'b' }] },
        { t: 'text', text: ' c' },
      ] },
    ]);
    // depth cap: no infinite descent on pathological nesting
    const deep = parseInline('*'.repeat(64) + 'x' + '*'.repeat(64));
    expect(JSON.stringify(deep).length).toBeLessThan(4000);
  });
  it('does not emphasize snake_case identifiers', () => {
    expect(parseInline('use snake_case_name here')).toEqual([
      { t: 'text', text: 'use snake_case_name here' },
    ]);
  });
  it('keeps markdown inside inline code literal', () => {
    expect(parseInline('`**not bold**`')).toEqual([{ t: 'code', text: '**not bold**' }]);
  });
  it('parses links and rejects unsafe hrefs to plain text', () => {
    expect(parseInline('[ok](https://a.dev)')).toEqual([
      { t: 'link', href: 'https://a.dev', children: [{ t: 'text', text: 'ok' }] },
    ]);
    expect(parseInline('[bad](javascript:alert(1))')).toEqual([
      { t: 'text', text: 'bad' },
    ]);
  });
  it('autolinks bare urls and trims trailing punctuation', () => {
    expect(parseInline('see https://a.dev/x, ok')).toEqual([
      { t: 'text', text: 'see ' },
      { t: 'link', href: 'https://a.dev/x', children: [{ t: 'text', text: 'https://a.dev/x' }] },
      { t: 'text', text: ', ok' },
    ]);
  });
  it('parses images and rejects unsafe srcs to alt text', () => {
    expect(parseInline('![cat](https://a.dev/c.png)')).toEqual([
      { t: 'image', src: 'https://a.dev/c.png', alt: 'cat' },
    ]);
    expect(parseInline('![cat](javascript:x)')).toEqual([{ t: 'text', text: 'cat' }]);
  });
  it('never interprets raw html', () => {
    expect(parseInline('<script>alert(1)</script>')).toEqual([
      { t: 'text', text: '<script>alert(1)</script>' },
    ]);
  });
});

describe('block parsing', () => {
  it('separates paragraphs on blank lines, soft-breaks within', () => {
    expect(p('a\nb\n\nc')).toEqual([
      { t: 'p', children: [{ t: 'text', text: 'a' }, { t: 'br' }, { t: 'text', text: 'b' }] },
      { t: 'p', children: [{ t: 'text', text: 'c' }] },
    ]);
  });
  it('parses headings h1-h4 only', () => {
    expect(p('# a')[0]).toEqual({ t: 'heading', level: 1, children: [{ t: 'text', text: 'a' }] });
    expect(p('#### a')[0]).toMatchObject({ t: 'heading', level: 4 });
    expect(p('##### a')[0]).toMatchObject({ t: 'p' }); // beyond subset → paragraph
  });
  it('parses fenced code with lang; unclosed fence consumes the rest', () => {
    expect(p('```js\nconst a = 1;\n```')).toEqual([
      { t: 'code', lang: 'js', text: 'const a = 1;' },
    ]);
    expect(p('```\n# not a heading\n**not bold**')).toEqual([
      { t: 'code', lang: '', text: '# not a heading\n**not bold**' },
    ]);
  });
  it('parses lists, ordered lists and task lists', () => {
    expect(p('- a\n- b')[0]).toMatchObject({ t: 'list', ordered: false });
    expect(p('1. a\n2. b')[0]).toMatchObject({ t: 'list', ordered: true });
    const task = p('- [ ] todo\n- [x] done')[0] as Extract<MdBlock, { t: 'list' }>;
    expect(task.items).toEqual([
      { task: true, checked: false, children: [{ t: 'text', text: 'todo' }] },
      { task: true, checked: true, children: [{ t: 'text', text: 'done' }] },
    ]);
  });
  it('distinguishes hr from list bullets', () => {
    expect(p('---')[0]).toEqual({ t: 'hr' });
    expect(p('- item')[0]).toMatchObject({ t: 'list' });
  });
  it('parses blockquotes recursively', () => {
    expect(p('> a\n> - b')).toEqual([
      { t: 'quote', children: [
        { t: 'p', children: [{ t: 'text', text: 'a' }] },
        { t: 'list', ordered: false, items: [{ children: [{ t: 'text', text: 'b' }] }] },
      ] },
    ]);
  });
  it('parses pipe tables with separator row', () => {
    const t = p('| a | b |\n|---|---|\n| 1 | 2 |')[0] as Extract<MdBlock, { t: 'table' }>;
    expect(t.head).toEqual([[{ t: 'text', text: 'a' }], [{ t: 'text', text: 'b' }]]);
    expect(t.rows).toEqual([[[{ t: 'text', text: '1' }], [{ t: 'text', text: '2' }]]]);
  });
  it('treats pipe lines without a separator as plain text', () => {
    expect(p('a | b')[0]).toMatchObject({ t: 'p' });
  });
});
