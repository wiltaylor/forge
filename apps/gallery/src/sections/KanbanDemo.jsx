import { createSignal } from 'solid-js';
import { PageHead, Card, Badge } from '@forge/ui';
import { KanbanBoard } from '@forge/kanban';

let nextId = 1;

const FIELDS = [
  { key: 'assignee', label: 'Assignee', type: 'select', placeholder: 'Unassigned', options: [
    { value: 'ada', label: 'Ada' }, { value: 'grace', label: 'Grace' },
    { value: 'linus', label: 'Linus' }, { value: 'margaret', label: 'Margaret' },
  ] },
  { key: 'priority', label: 'Priority', type: 'select', options: [
    { value: 'low', label: 'Low' }, { value: 'medium', label: 'Medium' }, { value: 'high', label: 'High' },
  ] },
  { key: 'priority', label: 'Level', type: 'badge', tones: { high: 'danger', medium: 'warning', low: 'neutral' } },
  { key: 'due', label: 'Due', type: 'date' },
  { key: 'estimate', label: 'Estimate (days)', type: 'slider', min: 1, max: 8, showValue: true },
  { key: 'done', label: 'Ready to ship', type: 'toggle' },
];

export default function KanbanDemo() {
  const [columns, setColumns] = createSignal([
    { id: 'backlog', title: 'Backlog' },
    { id: 'progress', title: 'In progress' },
    { id: 'review', title: 'Review', collapsed: true },
    { id: 'done', title: 'Done' },
  ]);
  const [cards, setCards] = createSignal([
    { id: 'c1', column: 'backlog', title: 'Auth tokens rotate', data: { assignee: 'ada', priority: 'high', due: '2026-07-20', estimate: 3, done: false } },
    { id: 'c2', column: 'backlog', title: 'Dark mode audit', data: { assignee: 'grace', priority: 'low', estimate: 1, done: false } },
    { id: 'c3', column: 'progress', title: 'Streaming exports', data: { assignee: 'linus', priority: 'medium', due: '2026-07-14', estimate: 5, done: false } },
    { id: 'c4', column: 'review', title: 'Billing webhooks', data: { assignee: 'margaret', priority: 'high', due: '2026-07-10', estimate: 2, done: true } },
    { id: 'c5', column: 'done', title: 'CLI login flow', data: { assignee: 'ada', priority: 'medium', estimate: 4, done: true } },
  ]);

  const changeCard = (id, data) => setCards((cs) => cs.map((c) => (c.id === id ? { ...c, data } : c)));
  const addCard = (columnId) =>
    setCards((cs) => [...cs, {
      id: `card-${nextId++}`, column: columnId, title: `New card ${nextId - 1}`,
      data: { priority: 'medium', estimate: 1, done: false },
    }]);
  const removeCard = (id) => setCards((cs) => cs.filter((c) => c.id !== id));
  const toggleColumn = (id, collapsed) => setColumns((cols) => cols.map((c) => (c.id === id ? { ...c, collapsed } : c)));

  return (
    <>
      <PageHead title="Kanban board" sub="Drag cards between columns; the controls on each card come from a field schema and write straight back into the card's data" />
      <Card padded={false} title="Sprint board" action={<Badge tone="accent">schema-driven cards</Badge>}>
        <div style={{ padding: '12px', height: '560px', 'box-sizing': 'border-box' }}>
          <KanbanBoard
            style={{ height: '100%' }}
            columns={columns()}
            cards={cards()}
            fields={FIELDS}
            onCardsChange={setCards}
            onCardChange={changeCard}
            onCardAdd={addCard}
            onCardRemove={removeCard}
            onColumnToggle={toggleColumn}
          />
        </div>
      </Card>
      <Card title="Cards (moves commit once per drop; edits per change)">
        <pre style={{ margin: 0, 'font-size': '11px', 'overflow-x': 'auto' }}>
          {JSON.stringify(cards(), null, 2)}
        </pre>
      </Card>
    </>
  );
}
