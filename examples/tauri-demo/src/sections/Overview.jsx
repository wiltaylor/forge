import { createSignal, onMount, onCleanup, For } from 'solid-js';
import { PageHead, Card, Stat, Button, Textarea, Logs, LogLine } from '@forge/ui';
import { api } from '../api';

/* The data plane over IPC: health via the generic request command, notes
   persisted through the doc store (putDebounced → survives restarts in
   <app_data_dir>/data/notes.json), and the EventBus bridged to the single
   `forge://event` Tauri event. */
export default function Overview() {
  const [health, setHealth] = createSignal(null);
  const [notes, setNotes] = createSignal('');
  const [log, setLog] = createSignal([]);

  const append = (topic) => (data) =>
    setLog((l) => [...l.slice(-49), { t: new Date().toLocaleTimeString(), topic, data }]);

  onMount(async () => {
    setHealth(await api.health());
    const doc = await api.data.get('notes');
    if (doc?.text) setNotes(doc.text);
    const offTicks = api.events.on('ticks', append('ticks'));
    const offDemo = api.events.on('demo', append('demo'));
    onCleanup(() => {
      offTicks();
      offDemo();
    });
  });

  const editNotes = (e) => {
    setNotes(e.currentTarget.value);
    api.data.putDebounced('notes', { text: e.currentTarget.value });
  };

  const publish = () =>
    api.actions.call('publish', { topic: 'demo', data: { hello: 'from the webview', at: Date.now() } });

  return (
    <div style={{ display: 'grid', gap: 'var(--sp-5)' }}>
      <PageHead title="Overview" sub="Forge contract over pure Tauri IPC — no HTTP server in this app" />

      <div style={{ display: 'grid', gap: 'var(--sp-4)', 'grid-template-columns': 'repeat(auto-fit, minmax(160px, 1fr))' }}>
        <Stat label="App" value={health()?.app ?? '…'} />
        <Stat label="Version" value={health()?.version ?? '…'} />
        <Stat label="Uptime" value={health() ? `${health().uptime_s}s` : '…'} />
        <Stat label="Actions" value={health()?.actions?.join(', ') ?? '…'} />
      </div>

      <Card title="Notes" sub="Debounced doc-store writes; persists across app restarts">
        <Textarea
          value={notes()}
          rows={4}
          placeholder="Type here — saved to <app_data_dir>/data/notes.json after 500 ms of quiet."
          onInput={editNotes}
        />
      </Card>

      <Card
        title="Live events"
        sub="EventBus → forge://event: a 2 s backend ticker plus the publish action"
        action={<Button variant="primary" onClick={publish}>Publish event</Button>}
      >
        <Logs style={{ 'max-height': '240px', 'overflow-y': 'auto' }}>
          <For each={[...log()].reverse()} fallback={<LogLine level="debug">waiting for events…</LogLine>}>
            {(e) => (
              <LogLine time={e.t} level={e.topic === 'demo' ? 'warn' : 'info'}>
                {e.topic}: {JSON.stringify(e.data)}
              </LogLine>
            )}
          </For>
        </Logs>
      </Card>
    </div>
  );
}
