import { createSignal, For } from 'solid-js';
import { PageHead, Card, Badge, IconButton } from '@forge/ui';
import { X } from 'lucide-solid';
import { BlockGrid, GridDndProvider, GridPalette, PaletteItem } from '@forge/grid';

let nextId = 1;

const TEMPLATES = [
  { w: 1, h: 1, label: 'Stat' },
  { w: 2, h: 1, label: 'Chart' },
  { w: 2, h: 2, label: 'Table' },
  { w: 3, h: 1, label: 'Wide' },
];

export default function GridDemo() {
  const [layout, setLayout] = createSignal([
    { id: 'cpu', x: 0, y: 0, w: 1, h: 1, label: 'CPU' },
    { id: 'mem', x: 1, y: 0, w: 1, h: 1, label: 'Memory' },
    { id: 'reqs', x: 2, y: 0, w: 2, h: 1, label: 'Requests' },
    { id: 'logs', x: 0, y: 1, w: 2, h: 2, label: 'Logs' },
    { id: 'latency', x: 2, y: 1, w: 4, h: 2, label: 'Latency' },
  ]);

  // Preserve untouched block objects so DOM (and settle transitions) survive commits.
  const commit = (next) =>
    setLayout((prev) =>
      next.map((nb) => {
        const old = prev.find((b) => b.id === nb.id);
        return old && old.x === nb.x && old.y === nb.y && old.w === nb.w && old.h === nb.h ? old : nb;
      }),
    );
  const addBlock = (pos, template) =>
    setLayout((prev) => [...prev, { id: `b${nextId++}`, ...pos, label: template.label }]);
  const removeBlock = (id) => setLayout((prev) => prev.filter((b) => b.id !== id));

  return (
    <>
      <PageHead title="Dashboard grid" sub="Drag blocks to rearrange — overlapped blocks are pushed aside; drag a corner to resize; drag a palette chip onto the grid to add a block" />
      <GridDndProvider>
      <Card title="Palette" action={<Badge tone="accent">drag onto the grid</Badge>}>
        <GridPalette>
          <For each={TEMPLATES}>
            {(t) => <PaletteItem template={t}>{t.label} {t.w}×{t.h}</PaletteItem>}
          </For>
        </GridPalette>
      </Card>
      <Card padded={false} title="Dashboard">
        <BlockGrid cols={6} rowHeight={90} layout={layout()} onLayoutChange={commit} onBlockAdd={addBlock}>
          {(block) => (
            <div style={{ display: 'flex', 'align-items': 'flex-start', 'justify-content': 'space-between', gap: '6px' }}>
              <span style={{ 'font-weight': 600 }}>{block.label}</span>
              <IconButton icon={X} label={`Remove ${block.label}`} data-no-drag onClick={() => removeBlock(block.id)} />
            </div>
          )}
        </BlockGrid>
      </Card>
      </GridDndProvider>
      <Card title="Layout (commits once per drop)">
        <pre style={{ margin: 0, 'font-size': '11px', 'overflow-x': 'auto' }}>
          {JSON.stringify(layout().map(({ id, x, y, w, h }) => ({ id, x, y, w, h })), null, 2)}
        </pre>
      </Card>
    </>
  );
}
