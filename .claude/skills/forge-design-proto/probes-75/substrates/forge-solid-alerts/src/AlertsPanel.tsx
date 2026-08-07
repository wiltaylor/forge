import { For } from 'solid-js';

const alerts = [
  { id: 1, level: 'critical', text: 'ingest lag above 90s', at: '12:41' },
  { id: 2, level: 'warning', text: 'indexer queue depth rising', at: '12:38' },
  { id: 3, level: 'info', text: 'nightly compaction finished', at: '03:02' },
];

export function AlertsPanel() {
  return (
    <div style={{ background: '#ffffff', border: '1px solid #ccc', padding: '20px', 'border-radius': '10px' }}>
      <h2 style={{ 'font-family': 'Georgia, serif', 'font-size': '22px', color: '#333' }}>Alerts</h2>
      <For each={alerts}>
        {(a) => (
          <div style={{ padding: '14px 0', 'border-bottom': '1px dotted #999' }}>
            <span style={{ color: a.level === 'critical' ? 'red' : a.level === 'warning' ? 'orange' : 'gray',
                           'font-weight': 'bold', 'text-transform': 'uppercase' }}>
              {a.level}
            </span>
            <span style={{ 'margin-left': '12px', color: '#555' }}>{a.text}</span>
            <span style={{ float: 'right', color: '#aaa' }}>{a.at}</span>
          </div>
        )}
      </For>
      <button style={{ 'margin-top': '16px', background: '#0066cc', color: 'white',
                       border: 'none', padding: '10px 18px', 'border-radius': '20px' }}>
        Acknowledge all
      </button>
    </div>
  );
}
