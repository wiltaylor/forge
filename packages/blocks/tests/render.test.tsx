/* The render arms of the read-only block renderer, one suite per arm.

   Each case mounts <BlockRenderer> — the package's public interface, and the
   same component the editor shows for an unfocused block — with an authored
   block, and asserts the element shape the arm must produce: the tag, the
   stylesheet class, and the content that must land inside it.

   `docs/web-testing.md` says to query by role and accessible name. These
   suites are a documented exception, like the corpus driver: a render arm's
   contract *is* its markup — `styles/blocks.css` selects on these classes —
   so here the tag and the class are the behaviour, not the implementation.
   The last suite closes that loop: every `fbk-` class the renderer emits
   must have a rule in the stylesheet. */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render } from '@solidjs/testing-library';
import { BlockRenderer, DOC_VERSION, createBlock } from '../src';
import type { Block, BlockData, BlockDef, BlockRendererProps, BlockType } from '../src';
import { loadRegistryKinds } from './kinds';

let nextId = 0;
/** An authored block: the payload under test plus a unique id. */
function blk(data: BlockData): Block {
  nextId += 1;
  return { id: `blk-${nextId}`, ...data } as Block;
}

type Ctx = Omit<BlockRendererProps, 'document'>;

function mount(blocks: Block[], ctx: Ctx = {}): HTMLElement {
  const { container } = render(() => (
    <BlockRenderer document={{ version: DOC_VERSION, blocks }} {...ctx} />
  ));
  return container;
}

/** The one element the selector must match. */
function one<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelectorAll<T>(selector);
  expect(found.length, `expected exactly one "${selector}"`).toBe(1);
  return found[0]!;
}

describe('paragraph', () => {
  it('renders <p class="fbk-p"> with its inline markdown', () => {
    const c = mount([blk({ type: 'paragraph', md: 'plain **bold**' })]);
    const p = one(c, 'p.fbk-p');
    expect(one(p, 'strong').textContent).toBe('bold');
    expect(p.textContent).toBe('plain bold');
  });
});

describe('heading', () => {
  for (const level of [1, 2, 3, 4] as const) {
    it(`level ${level} renders <h${level} class="fbk-h fbk-h${level}">`, () => {
      const c = mount([blk({ type: 'heading', level, md: `Title ${level}` })]);
      const h = one(c, `h${level}.fbk-h.fbk-h${level}`);
      expect(h.textContent).toBe(`Title ${level}`);
    });
  }
});

describe('list_item', () => {
  it('a bullet item renders its marker and its indent variable', () => {
    const c = mount([blk({ type: 'list_item', style: 'bullet', indent: 2, md: 'point' })]);
    const li = one(c, '.fbk-li');
    expect(one(li, '.fbk-li-marker').textContent).toBe('•');
    expect(li.getAttribute('style')).toContain('--fbk-indent: 2');
    expect(li.textContent).toContain('point');
  });

  it('numbered items count up and reset after a non-list block', () => {
    const c = mount([
      blk({ type: 'list_item', style: 'number', indent: 0, md: 'one' }),
      blk({ type: 'list_item', style: 'number', indent: 0, md: 'two' }),
      blk({ type: 'paragraph', md: 'break' }),
      blk({ type: 'list_item', style: 'number', indent: 0, md: 'anew' }),
    ]);
    const markers = [...c.querySelectorAll('.fbk-li-marker')].map((m) => m.textContent);
    expect(markers).toEqual(['1.', '2.', '1.']);
  });

  it('a todo item renders a checkbox, disabled without a toggle handler', () => {
    const c = mount([
      blk({ type: 'list_item', style: 'todo', checked: false, indent: 0, md: 'task' }),
    ]);
    const box = one<HTMLInputElement>(c, '.fbk-li input[type="checkbox"]');
    expect(box.checked).toBe(false);
    expect(box.disabled).toBe(true);
    expect(c.querySelector('.fbk-li-done')).toBeNull();
  });

  it('a checked todo item ticks the box and strikes the text', () => {
    const c = mount([
      blk({ type: 'list_item', style: 'todo', checked: true, indent: 0, md: 'done' }),
    ]);
    expect(one<HTMLInputElement>(c, '.fbk-li input[type="checkbox"]').checked).toBe(true);
    expect(one(c, '.fbk-li-done').textContent).toBe('done');
  });

  it('a click on the checkbox reports through onToggleTodo', () => {
    const onToggleTodo = vi.fn();
    const todo = blk({ type: 'list_item', style: 'todo', checked: false, indent: 0, md: 'task' });
    const c = mount([todo], { onToggleTodo });
    const box = one<HTMLInputElement>(c, '.fbk-li input[type="checkbox"]');
    expect(box.disabled).toBe(false);
    fireEvent.click(box);
    expect(onToggleTodo).toHaveBeenCalledWith(todo.id, true);
  });
});

describe('quote', () => {
  it('renders <blockquote class="fbk-quote">', () => {
    const c = mount([blk({ type: 'quote', md: 'said *so*' })]);
    const q = one(c, 'blockquote.fbk-quote');
    expect(one(q, 'em').textContent).toBe('so');
  });
});

describe('divider', () => {
  it('renders <hr class="fbk-hr">', () => {
    one(mount([blk({ type: 'divider' })]), 'hr.fbk-hr');
  });
});

describe('code', () => {
  it('renders a read-only code editor with the language on the wrapper', () => {
    const c = mount([blk({ type: 'code', lang: 'ts', code: 'const one = 1;' })]);
    const wrap = one(c, '.fbk-code');
    expect(wrap.getAttribute('data-lang')).toBe('ts');
    const content = one(wrap, '[role="textbox"]');
    expect(content.getAttribute('aria-readonly')).toBe('true');
    expect(content.textContent).toContain('const one = 1;');
  });

  it('leaves the language attribute off when the block has none', () => {
    const c = mount([blk({ type: 'code', lang: '', code: 'x' })]);
    expect(one(c, '.fbk-code').getAttribute('data-lang')).toBeNull();
  });
});

describe('table', () => {
  it('renders header and body cells with inline markdown inside', () => {
    const c = mount([
      blk({ type: 'table', header: ['Name', 'Role'], rows: [['Ada', '*lead*'], ['Bo', 'dev']] }),
    ]);
    const table = one(c, '.fbk-tablewrap table.fbk-table');
    const heads = [...table.querySelectorAll('thead th')].map((th) => th.textContent);
    expect(heads).toEqual(['Name', 'Role']);
    const bodyRows = table.querySelectorAll('tbody tr');
    expect(bodyRows.length).toBe(2);
    expect([...bodyRows[0]!.querySelectorAll('td')].map((td) => td.textContent)).toEqual([
      'Ada', 'lead',
    ]);
    expect(one(bodyRows[0]!, 'em').textContent).toBe('lead');
  });
});

describe('admonition', () => {
  it('renders an alert in the block tone with title and body', () => {
    const c = mount([
      blk({ type: 'admonition', tone: 'warning', title: 'Careful', md: 'the **body**' }),
    ]);
    const alert = one(c, '.fbk-adm [role="alert"]');
    expect(alert.classList.contains('falert-warning')).toBe(true);
    expect(alert.textContent).toContain('Careful');
    expect(one(alert, 'strong').textContent).toBe('body');
  });
});

describe('columns', () => {
  it('renders one weighted column per cell with the cell blocks inside', () => {
    const c = mount([
      blk({
        type: 'columns',
        columns: [
          { ratio: 0.3, blocks: [blk({ type: 'paragraph', md: 'left' })] },
          { ratio: 0.7, blocks: [blk({ type: 'paragraph', md: 'right' })] },
        ],
      }),
    ]);
    const cols = [...one(c, '.fbk-cols').querySelectorAll('.fbk-col')];
    expect(cols.length).toBe(2);
    expect(cols[0]!.getAttribute('style')).toContain('flex-grow: 300');
    expect(one(cols[0]!, 'p.fbk-p').textContent).toBe('left');
    expect(one(cols[1]!, 'p.fbk-p').textContent).toBe('right');
  });
});

describe('custom', () => {
  const gizmo: BlockDef = {
    label: 'Gizmo',
    create: () => ({ n: 0 }),
    render: (p) => <span class="gizmo-view">{JSON.stringify(p.data)}</span>,
  };

  it('renders a registered kind through its BlockDef', () => {
    const c = mount(
      [blk({ type: 'custom', kind: 'gizmo', data: { n: 2 } })],
      { customBlocks: { gizmo } },
    );
    expect(one(c, '.fbk-custom .gizmo-view').textContent).toBe('{"n":2}');
  });

  it('renders a warning for an unregistered kind', () => {
    const c = mount([blk({ type: 'custom', kind: 'gizmo', data: null })]);
    const alert = one(c, '[role="alert"]');
    expect(alert.classList.contains('falert-warning')).toBe(true);
    expect(alert.textContent).toContain('Unknown block “gizmo”');
  });
});

describe('image', () => {
  it('renders a figure with the image and its alt text as caption', () => {
    const c = mount([
      blk({ type: 'image', src: '/pic.png', alt: 'A pic', width: 640, height: 480 }),
    ]);
    const img = one<HTMLImageElement>(c, 'figure.fbk-img img');
    expect(img.getAttribute('src')).toBe('/pic.png');
    expect(img.getAttribute('alt')).toBe('A pic');
    expect(img.getAttribute('width')).toBe('640');
    expect(img.getAttribute('height')).toBe('480');
    expect(one(c, 'figcaption').textContent).toBe('A pic');
  });

  it('drops the caption when the alt text is empty', () => {
    const c = mount([blk({ type: 'image', src: '/pic.png', alt: '' })]);
    expect(c.querySelector('figcaption')).toBeNull();
  });
});

describe('video', () => {
  it('shows a titled facade, then a plain <video> for a file source', () => {
    const c = mount([blk({ type: 'video', src: '/clip.mp4', title: 'Intro' })]);
    const facade = one<HTMLButtonElement>(c, '.fbk-video button.fbk-video-facade');
    expect(one(facade, '.fbk-video-title').textContent).toBe('Intro');
    fireEvent.click(facade);
    const video = one<HTMLVideoElement>(c, '.fbk-video video');
    expect(video.getAttribute('src')).toBe('/clip.mp4');
    expect(video.hasAttribute('controls')).toBe(true);
  });

  it('embeds a YouTube source as a nocookie iframe after the facade', () => {
    const c = mount([blk({ type: 'video', src: 'https://youtu.be/dQw4w9WgXcQ' })]);
    fireEvent.click(one(c, '.fbk-video-facade'));
    const frame = one<HTMLIFrameElement>(c, '.fbk-video iframe');
    expect(frame.getAttribute('src')).toBe(
      'https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?autoplay=1',
    );
  });
});

describe('math', () => {
  it('shows the TeX source when no typesetter is given', () => {
    const c = mount([blk({ type: 'math', tex: '\\frac{1}{2}' })]);
    expect(one(c, '.fbk-math code').textContent).toBe('\\frac{1}{2}');
  });

  it('hands the TeX source to renderMath when one is given', () => {
    const c = mount(
      [blk({ type: 'math', tex: 'x^2' })],
      { renderMath: (tex) => <var class="typeset">{tex}</var> },
    );
    expect(one(c, '.fbk-math var.typeset').textContent).toBe('x^2');
    expect(c.querySelector('.fbk-math code')).toBeNull();
  });
});

describe('bar_chart', () => {
  it('renders a chart figure with title, bars and axis labels', () => {
    const c = mount([
      blk({
        type: 'bar_chart',
        title: 'Sales',
        x_label: 'Quarter',
        y_label: 'Units',
        categories: ['A', 'B', 'C'],
        series: [{ name: 'S1', values: [3, 5, 4] }],
      }),
    ]);
    const fig = one(c, 'figure.fbk-chart');
    expect(one(fig, 'figcaption.fbk-chart-title').textContent).toBe('Sales');
    one(fig, 'svg[aria-label="Bar chart"]');
    expect(fig.querySelectorAll('.fbar').length).toBe(3);
    const axes = one(fig, '.fbk-chart-axes');
    expect(axes.textContent).toContain('↑ Units');
    expect(axes.textContent).toContain('→ Quarter');
  });
});

describe('line_chart', () => {
  it('renders a line chart and lists its annotated points', () => {
    const c = mount([
      blk({
        type: 'line_chart',
        categories: ['A', 'B', 'C'],
        series: [{ name: 'S1', values: [3, 5, 4] }],
        points: [{ label: 'peak', category: 1, value: 5 }],
      }),
    ]);
    const fig = one(c, 'figure.fbk-chart');
    one(fig, 'svg[aria-label="Line chart"]');
    one(fig, 'polyline');
    expect(one(fig, '.fbk-chart-points').textContent).toBe('peak: B → 5');
  });
});

describe('pie_chart', () => {
  it('renders one slice per datum and a labelled legend', () => {
    const c = mount([
      blk({ type: 'pie_chart', slices: [{ label: 'A', value: 3 }, { label: 'B', value: 5 }] }),
    ]);
    const fig = one(c, 'figure.fbk-chart');
    one(fig, 'svg[aria-label="Pie chart"]');
    expect(fig.querySelectorAll('.fslice').length).toBe(2);
    expect(one(fig, '.fchart-legend').textContent).toContain('A');
  });
});

describe('diagram', () => {
  it('renders a flowchart node per node and an edge per edge', () => {
    const c = mount([
      blk({
        type: 'diagram',
        nodes: [
          { id: 's', kind: 'terminator', text: 'Start' },
          { id: 'w', kind: 'process', text: 'Work' },
        ],
        edges: [{ from: 's', to: 'w' }],
      }),
    ]);
    const box = one(c, '.fbk-diagram');
    const nodes = [...box.querySelectorAll('.fflow-node')];
    expect(nodes.map((el) => el.textContent)).toEqual(['Start', 'Work']);
    expect(nodes[0]!.classList.contains('tone-terminator')).toBe(true);
    expect(nodes[1]!.classList.contains('tone-process')).toBe(false);
    expect(box.querySelectorAll('.fgraph-edge').length).toBe(1);
  });
});

describe('state_diagram', () => {
  it('marks initial and final states and labels guarded transitions', () => {
    const c = mount([
      blk({
        type: 'state_diagram',
        states: [
          { id: 'idle', initial: true, name: 'Idle' },
          { id: 'done', final: true, name: 'Done' },
        ],
        transitions: [{ from: 'idle', to: 'done', trigger: 'finish', guard: 'ok' }],
      }),
    ]);
    const box = one(c, '.fbk-diagram');
    const nodes = [...box.querySelectorAll('.fflow-node')];
    expect(nodes[0]!.textContent).toBe('● Idle');
    expect(nodes[0]!.classList.contains('tone-initial')).toBe(true);
    expect(nodes[1]!.textContent).toBe('◉ Done');
    expect(nodes[1]!.classList.contains('tone-final')).toBe(true);
    expect(one(box, '.fflow-edge-label').textContent).toBe('finish [ok]');
  });
});

describe('sequence_diagram', () => {
  it('renders lifelines, messages, a dashed reply and an anchored note', () => {
    const c = mount([
      blk({
        type: 'sequence_diagram',
        participants: [{ id: 'a', name: 'Client' }, { id: 'b', name: 'Server' }],
        messages: [
          { from: 'a', to: 'b', text: 'request' },
          { from: 'b', to: 'a', kind: 'reply', text: 'response' },
        ],
        notes: [{ at: 0, text: 'over TLS' }],
      }),
    ]);
    const svg = one(c, '.fbk-seq svg[aria-label="Sequence diagram"]');
    expect(svg.querySelectorAll('.fbk-seq-lifeline').length).toBe(2);
    expect([...svg.querySelectorAll('.fbk-seq-name')].map((t) => t.textContent)).toEqual([
      'Client', 'Server',
    ]);
    const msgs = [...svg.querySelectorAll('.fbk-seq-msg')];
    expect(msgs.length).toBe(2);
    expect(msgs[0]!.classList.contains('is-dashed')).toBe(false);
    expect(msgs[1]!.classList.contains('is-dashed')).toBe(true);
    expect(svg.querySelectorAll('.fbk-seq-arrow').length).toBe(2);
    expect(one(svg, '.fbk-seq-note').textContent).toContain('over TLS');
  });
});

describe('node_table', () => {
  it('renders a titled row table with inline markdown per row', () => {
    const c = mount([
      blk({ type: 'node_table', title: 'Users', rows: [{ md: '**id** int' }, { md: 'name' }] }),
    ]);
    const table = one(c, '.fbk-ntable');
    expect(one(table, '.fbk-ntable-title').textContent).toBe('Users');
    const rows = table.querySelectorAll('.fbk-ntable-row');
    expect(rows.length).toBe(2);
    expect(one(rows[0]!, 'strong').textContent).toBe('id');
  });

  it('renders headerless when the title is empty', () => {
    const c = mount([blk({ type: 'node_table', title: '', rows: [{ md: 'row' }] })]);
    expect(c.querySelector('.fbk-ntable-title')).toBeNull();
  });
});

describe('tree', () => {
  it('renders guide-prefixed rows depth-first with optional icons', () => {
    const c = mount([
      blk({
        type: 'tree',
        nodes: [{ title: 'root', icon: '📁', children: [{ title: 'a' }, { title: 'b' }] }],
      }),
    ]);
    const rows = [...one(c, '.fbk-tree').querySelectorAll('.fbk-tree-row')];
    expect(rows.map((r) => r.textContent)).toEqual(['└─ 📁 root', '   ├─ a', '   └─ b']);
    one(rows[0]!, '.fbk-tree-icon');
  });
});

describe('timeline', () => {
  it('sorts items by date and tags each with the phase it falls in', () => {
    const c = mount([
      blk({
        type: 'timeline',
        title: 'Roadmap',
        direction: 'horizontal',
        phases: [{ label: 'Alpha', from: '2026-01-01', to: '2026-02-01' }],
        items: [
          { label: 'Later', on: '2026-03-01' },
          { label: 'First', on: '2026-01-05' },
        ],
      }),
    ]);
    const tl = one(c, '.fbk-timeline');
    expect(tl.classList.contains('is-horizontal')).toBe(true);
    expect(one(tl, '.fbk-timeline-title').textContent).toBe('Roadmap');
    expect(one(tl, '.fbk-timeline-phase').textContent).toContain('Alpha');
    const items = [...tl.querySelectorAll('.fbk-timeline-item')];
    expect(items.map((i) => one(i, '.fbk-timeline-label').textContent)).toEqual([
      'First', 'Later',
    ]);
    expect(one(items[0]!, '.fbk-timeline-date').textContent).toContain('· Alpha');
    expect(one(items[1]!, '.fbk-timeline-date').textContent).not.toContain('Alpha');
  });
});

describe('chapter_header', () => {
  it('renders kicker, inline-markdown title and dot-joined meta', () => {
    const c = mount([
      blk({
        type: 'chapter_header',
        title: 'The **Plan**',
        kicker: 'Chapter 1',
        reading_time: '5 min',
        updated: '2026-08-01',
        version: 'v2',
      }),
    ]);
    const header = one(c, 'header.fbk-chapter');
    expect(one(header, '.fbk-chapter-kicker').textContent).toBe('Chapter 1');
    expect(one(header, 'h1.fbk-chapter-title strong').textContent).toBe('Plan');
    expect(one(header, '.fbk-chapter-meta').textContent).toBe('5 min · 2026-08-01 · v2');
  });

  it('drops kicker and meta when the block has none', () => {
    const c = mount([blk({ type: 'chapter_header', title: 'Bare' })]);
    expect(c.querySelector('.fbk-chapter-kicker')).toBeNull();
    expect(c.querySelector('.fbk-chapter-meta')).toBeNull();
  });
});

describe('footnote', () => {
  it('renders an addressable definition with its label and body', () => {
    const c = mount([blk({ type: 'footnote', label: 'src-1', md: 'See **this**' })]);
    const note = one(c, '.fbk-footnote');
    expect(note.id).toBe('fn-src-1');
    expect(one(note, 'sup.fbk-footnote-label').textContent).toBe('[src-1]');
    expect(one(note, 'strong').textContent).toBe('this');
  });
});

describe('the stylesheet', () => {
  it('has a rule for every fbk- class the renderer emits', () => {
    const css = readFileSync(resolve(import.meta.dirname, '../styles/blocks.css'), 'utf8');

    /* Every kind's starter, plus the variants whose classes only appear with
       the right payload (a checked todo, a titled facade, chart trimmings,
       a note, phases, a kicker …). */
    const starters = loadRegistryKinds().map((k) => createBlock(k.type as BlockType));
    const variants: Block[] = [
      blk({ type: 'list_item', style: 'todo', checked: true, indent: 0, md: 'done' }),
      blk({ type: 'video', src: '/clip.mp4', title: 'Intro' }),
      blk({
        type: 'bar_chart', title: 'T', x_label: 'x', y_label: 'y',
        categories: ['A'], series: [{ name: 'S', values: [1] }],
      }),
      blk({
        type: 'line_chart', categories: ['A'], series: [{ name: 'S', values: [1] }],
        points: [{ label: 'p', category: 0, value: 1 }],
      }),
      blk({
        type: 'sequence_diagram',
        participants: [{ id: 'a' }, { id: 'b' }],
        messages: [{ from: 'a', to: 'b', text: 'hi' }],
        notes: [{ at: 0, text: 'note' }],
      }),
      blk({ type: 'tree', nodes: [{ title: 'root', icon: '📁' }] }),
      blk({
        type: 'timeline', title: 'T',
        phases: [{ label: 'P', from: '2026-01-01', to: '2026-02-01' }],
        items: [{ label: 'i', on: '2026-01-05' }],
      }),
      blk({ type: 'chapter_header', title: 'T', kicker: 'K', version: 'v1' }),
    ];
    const gizmo: BlockDef = {
      label: 'Gizmo',
      create: () => null,
      render: () => <span>g</span>,
    };
    const c = mount([...starters, ...variants], { customBlocks: { '': gizmo } });

    const emitted = new Set<string>();
    for (const el of c.querySelectorAll('*')) {
      for (const cls of el.classList) if (/^fbk-/.test(cls)) emitted.add(cls);
    }
    // A near-empty set means the render produced nothing, not a clean pass.
    expect(emitted.size).toBeGreaterThan(30);

    const missing = [...emitted].filter(
      (cls) => !new RegExp(`\\.${cls}(?![\\w-])`).test(css),
    );
    expect(missing, 'classes the renderer emits with no stylesheet rule').toEqual([]);
  });
});
