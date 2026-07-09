import { For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { PageHead, Card, Grid, Badge, Button, Logs, LogLine } from '@forge/ui';
import { api } from '../api';

/* Live events over both transports: the demo backends publish a `ticks`
   event every 2 s; the publish button round-trips through the `publish`
   action → event bus → SSE + WS. */
export default function LiveDemo() {
  const [sse, setSse] = createSignal([]);
  const [ws, setWs] = createSignal([]);
  const [wsState, setWsState] = createSignal('connecting');
  const stamp = () => new Date().toTimeString().slice(0, 8);
  const push = (set, entry) => set((xs) => [...xs.slice(-19), entry]);

  onMount(() => {
    const offSse = api.events.on('ticks', (data) =>
      push(setSse, { time: stamp(), msg: JSON.stringify(data) }));

    const socket = api.ws.connect();
    const offOpen = socket.on('open', () => setWsState('open'));
    const offEvent = socket.onEvent('ticks', (data) =>
      push(setWs, { time: stamp(), msg: JSON.stringify(data) }));
    socket.subscribe(['ticks']);

    onCleanup(() => {
      offSse();
      offOpen?.();
      offEvent();
      socket.close();
    });
  });

  const publish = () =>
    api.actions.call('publish', { topic: 'ticks', data: { manual: true, at: stamp() } })
      .catch((e) => console.warn('publish failed', e));

  return (
    <>
      <PageHead title="Live events" sub="SSE + WebSocket fan-out from the backend event bus"
                actions={<Button variant="primary" onClick={publish}>Publish event</Button>} />
      <Grid>
        <Card title="Server-sent events" action={<Badge tone="accent">/api/events</Badge>}>
          <Feed items={sse()} empty="Waiting for events…" />
        </Card>
        <Card title="WebSocket" action={<Badge tone={wsState() === 'open' ? 'success' : 'warning'}>/api/ws · {wsState()}</Badge>}>
          <Feed items={ws()} empty="Waiting for frames…" />
        </Card>
      </Grid>
    </>
  );
}

function Feed(props) {
  return (
    <Show when={props.items.length} fallback={<small>{props.empty}</small>}>
      <Logs style={{ height: '200px' }}>
        <For each={props.items}>
          {(e) => <LogLine time={e.time} level="info">{e.msg}</LogLine>}
        </For>
      </Logs>
    </Show>
  );
}
