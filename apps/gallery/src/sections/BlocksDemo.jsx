import { Show, createSignal } from 'solid-js';
import { PageHead, Card, Toggle, Button, Modal, Stat } from '@forge/ui';
import { CodeEditor } from '@forge/code';
import { BlockEditor, fromMarkdown, toMarkdown, newId } from '@forge/blocks';

/* The same sample content every platform demos (forge-blocks sample.rs). */
const SAMPLE = {
  version: 1,
  blocks: [
    {
      id: newId(),
      type: 'chapter_header',
      title: 'Forge Blocks',
      kicker: 'Design system',
      reading_time: '4 min',
      updated: '2026-07-18',
      version: 'v1',
    },
    { id: newId(), type: 'heading', level: 1, md: 'Forge Blocks :rocket:' },
    {
      id: newId(),
      type: 'paragraph',
      md: 'A **block-based** page editor with *inline markdown*, `code`, [links](https://example.com), ~~regrets~~, and :sparkles: emoji. Focus a block to edit its raw source; press `/` on an empty block for the block palette.',
    },
    { id: newId(), type: 'heading', level: 2, md: 'Typography' },
    { id: newId(), type: 'list_item', style: 'bullet', indent: 0, md: 'Bullet lists with **bold** entries' },
    { id: newId(), type: 'list_item', style: 'bullet', indent: 1, md: 'nested by indent' },
    { id: newId(), type: 'list_item', style: 'number', indent: 0, md: 'Numbered items' },
    { id: newId(), type: 'list_item', style: 'todo', checked: true, indent: 0, md: 'Ship the schema' },
    { id: newId(), type: 'list_item', style: 'todo', checked: false, indent: 0, md: 'Ship the editors' },
    { id: newId(), type: 'quote', md: 'Blocks all the way down.' },
    { id: newId(), type: 'divider' },
    { id: newId(), type: 'heading', level: 2, md: 'Code' },
    { id: newId(), type: 'code', lang: 'ts', code: "export function hello(): string {\n  return 'blocks';\n}" },
    { id: newId(), type: 'heading', level: 2, md: 'Data' },
    {
      id: newId(),
      type: 'table',
      header: ['Kit', 'Language', 'Status'],
      rows: [
        ['web', 'SolidJS', ':white_check_mark: shipped'],
        ['tui', '**Rust**', ':white_check_mark: shipped'],
        ['egui', '**Rust**', ':hourglass: rolling'],
      ],
    },
    { id: newId(), type: 'admonition', tone: 'warning', title: 'Careful', md: 'Admonitions carry a tone, a title, and an **inline-markdown** body.' },
    { id: newId(), type: 'admonition', tone: 'info', title: 'Tip', md: 'Type `:::danger` at the start of a paragraph to convert it.' },
    { id: newId(), type: 'heading', level: 2, md: 'Columns' },
    {
      id: newId(),
      type: 'columns',
      columns: [
        {
          ratio: 0.5,
          blocks: [
            { id: newId(), type: 'heading', level: 3, md: 'Left' },
            { id: newId(), type: 'paragraph', md: 'Columns split content side by side.' },
          ],
        },
        {
          ratio: 0.5,
          blocks: [
            { id: newId(), type: 'heading', level: 3, md: 'Right' },
            { id: newId(), type: 'paragraph', md: 'Each cell holds its own block list.' },
          ],
        },
      ],
    },
    { id: newId(), type: 'custom', kind: 'stat', data: { label: 'Requests', value: '1.2k', delta: 4.2 } },
    { id: newId(), type: 'heading', level: 2, md: 'Media' },
    {
      id: newId(),
      type: 'image',
      src: 'https://picsum.photos/seed/forge/640/360',
      alt: 'A random landscape',
      width: 640,
      height: 360,
    },
    { id: newId(), type: 'video', src: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ', title: 'Launch demo' },
    { id: newId(), type: 'math', tex: '\\frac{1}{N}\\sum_{i=1}^{N} (y_i - \\hat{y}_i)^2' },
    { id: newId(), type: 'heading', level: 2, md: 'Charts' },
    {
      id: newId(),
      type: 'bar_chart',
      title: 'Quarterly revenue ($k)',
      categories: ['Q1', 'Q2', 'Q3', 'Q4'],
      series: [
        { name: 'North', values: [42, 55, 61, 78] },
        { name: 'South', values: [30, 48, 52, 66] },
      ],
      y_min: 0,
      y_max: 100,
    },
    {
      id: newId(),
      type: 'line_chart',
      title: 'Latency p95 (ms)',
      categories: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri'],
      series: [
        { name: 'api', values: [120, 132, 101, 134, 90] },
        { name: 'web', values: [220, 182, 191, 234, 150] },
      ],
    },
    {
      id: newId(),
      type: 'pie_chart',
      title: 'Traffic by source',
      slices: [
        { label: 'Search', value: 46 },
        { label: 'Direct', value: 32 },
        { label: 'Referral', value: 14 },
        { label: 'Social', value: 8 },
      ],
    },
    { id: newId(), type: 'heading', level: 2, md: 'Diagrams' },
    {
      id: newId(),
      type: 'diagram',
      nodes: [
        { id: 'start', kind: 'terminator', text: 'Start' },
        { id: 'build', kind: 'process', text: 'Build' },
        { id: 'ok', kind: 'decision', text: 'Tests pass?' },
        { id: 'ship', kind: 'terminator', text: 'Ship' },
      ],
      edges: [
        { from: 'start', to: 'build' },
        { from: 'build', to: 'ok' },
        { from: 'ok', to: 'ship', label: 'yes' },
        { from: 'ok', to: 'build', label: 'no' },
      ],
    },
    {
      id: newId(),
      type: 'sequence_diagram',
      participants: [
        { id: 'cli', name: 'Client' },
        { id: 'api', name: 'API' },
        { id: 'db', name: 'DB' },
      ],
      messages: [
        { from: 'cli', to: 'api', text: 'POST /login' },
        { from: 'api', to: 'db', text: 'SELECT user', kind: 'sync' },
        { from: 'db', to: 'api', text: 'row', kind: 'reply' },
        { from: 'api', to: 'cli', text: '200 + token', kind: 'reply' },
      ],
    },
    {
      id: newId(),
      type: 'state_diagram',
      states: [
        { id: 'idle', name: 'Idle', initial: true },
        { id: 'running', name: 'Running' },
        { id: 'done', name: 'Done', final: true },
      ],
      transitions: [
        { from: 'idle', to: 'running', trigger: 'start' },
        { from: 'running', to: 'done', trigger: 'finish', guard: 'ok' },
        { from: 'running', to: 'idle', trigger: 'abort' },
      ],
    },
    {
      id: newId(),
      type: 'node_table',
      title: 'users',
      rows: [
        { key: 'id', md: '`id` **uuid** pk' },
        { key: 'email', md: '`email` **text** unique' },
        { md: '`created_at` **timestamptz**' },
      ],
    },
    {
      id: newId(),
      type: 'tree',
      nodes: [
        {
          title: 'src',
          children: [
            { title: 'lib.rs' },
            { title: 'widgets', children: [{ title: 'blocks' }] },
          ],
        },
      ],
    },
    { id: newId(), type: 'heading', level: 2, md: 'Timeline' },
    {
      id: newId(),
      type: 'timeline',
      title: 'Release plan',
      phases: [
        { label: 'Alpha', from: '2026-01-01', to: '2026-03-01' },
        { label: 'Beta', from: '2026-03-01', to: '2026-06-01' },
      ],
      items: [
        { label: 'Kickoff', on: '2026-01-01' },
        { label: 'Feature freeze', on: '2026-04-15' },
        { label: 'GA', on: '2026-06-01' },
      ],
    },
    {
      id: newId(),
      type: 'paragraph',
      md: 'Footnotes get inline references[^spec] that link to definitions.',
    },
    { id: newId(), type: 'footnote', label: 'spec', md: 'See the frozen JSON fixtures in `tests/schema.rs`.' },
  ],
};

/* A custom block proving the BlockDef extension contract. */
const CUSTOM_BLOCKS = {
  stat: {
    label: 'Stat card',
    create: () => ({ label: 'Metric', value: '0', delta: 0 }),
    render: (props) => (
      <div style={{ 'max-width': '260px' }}>
        <Stat label={props.data?.label ?? ''} value={props.data?.value ?? ''} delta={props.data?.delta} />
      </div>
    ),
    edit: (props) => (
      <div style={{ display: 'flex', gap: '8px', 'align-items': 'center' }}>
        <input
          class="fbk-admtitle"
          style={{ flex: '0 0 140px', border: '1px solid var(--border-default)', 'border-radius': '4px', padding: '4px 8px' }}
          value={props.data?.label ?? ''}
          onInput={(e) => props.onChange({ ...props.data, label: e.currentTarget.value })}
        />
        <input
          class="fbk-admtitle"
          style={{ flex: '0 0 100px', border: '1px solid var(--border-default)', 'border-radius': '4px', padding: '4px 8px' }}
          value={props.data?.value ?? ''}
          onInput={(e) => props.onChange({ ...props.data, value: e.currentTarget.value })}
        />
        <span style={{ color: 'var(--fg-2)', 'font-size': '12px' }}>stat block — label / value</span>
      </div>
    ),
  },
};

export default function BlocksDemo() {
  const [doc, setDoc] = createSignal(SAMPLE);
  const [readOnly, setReadOnly] = createSignal(false);
  const [exportOpen, setExportOpen] = createSignal(false);
  const [importOpen, setImportOpen] = createSignal(false);
  let importText = '';

  return (
    <>
      <PageHead
        title="Block editor"
        sub="Notion-style page building — source-per-block editing, slash commands, emoji, columns, custom blocks. Same document schema as the TUI and egui editors."
      />
      <Card
        title="Page"
        action={
          <div style={{ display: 'flex', gap: '8px', 'align-items': 'center' }}>
            <Toggle checked={readOnly()} onChange={setReadOnly}>Read-only</Toggle>
            <Button size="sm" variant="secondary" onClick={() => setExportOpen(true)}>
              Export markdown
            </Button>
            <Button size="sm" variant="secondary" onClick={() => setImportOpen(true)}>
              Import
            </Button>
          </div>
        }
      >
        <BlockEditor
          document={doc()}
          onChange={setDoc}
          customBlocks={CUSTOM_BLOCKS}
          readOnly={readOnly()}
          placeholder="Type '/' for blocks"
        />
      </Card>

      <Modal open={exportOpen()} onClose={() => setExportOpen(false)} title="Markdown export" size="lg">
        <Show when={exportOpen()}>
          <CodeEditor value={toMarkdown(doc())} readOnly height="420px" />
        </Show>
      </Modal>

      <Modal
        open={importOpen()}
        onClose={() => setImportOpen(false)}
        title="Import markdown"
        size="lg"
        footer={
          <Button
            onClick={() => {
              setDoc(fromMarkdown(importText));
              setImportOpen(false);
            }}
          >
            Import
          </Button>
        }
      >
        <textarea
          class="fbk-ta"
          style={{
            width: '100%', height: '320px', border: '1px solid var(--border-default)',
            'border-radius': '6px', padding: '10px', overflow: 'auto', resize: 'vertical',
          }}
          placeholder="Paste markdown…"
          onInput={(e) => (importText = e.currentTarget.value)}
        />
      </Modal>
    </>
  );
}
