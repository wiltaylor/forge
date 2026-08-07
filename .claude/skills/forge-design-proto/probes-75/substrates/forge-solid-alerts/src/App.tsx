import { For } from 'solid-js';
import { Button } from './forge/Button';
import { AlertsPanel } from './AlertsPanel';

const services = [
  { name: 'ingest', region: 'us-east-1', status: 'ok' },
  { name: 'indexer', region: 'eu-west-1', status: 'degraded' },
];

export default function App() {
  return (
    <div class="app-shell">
      <header class="app-bar">
        <span class="eyebrow">opsview</span>
        <Button variant="primary">Deploy</Button>
      </header>
      <main class="page">
        <h1 class="page-title">Services</h1>
        <ul class="stack">
          <For each={services}>
            {(s) => (
              <li class="row">
                <span>{s.name}</span>
                <span class="muted">{s.region}</span>
                <span class={`fbadge fbadge-${s.status === 'ok' ? 'ok' : 'warn'}`}>{s.status}</span>
              </li>
            )}
          </For>
        </ul>
        <AlertsPanel />
      </main>
    </div>
  );
}
