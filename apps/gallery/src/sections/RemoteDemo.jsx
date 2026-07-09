import { For, Match, Switch, createResource, createSignal } from 'solid-js';
import { PageHead, Card, Grid, Alert, Badge, Logs, LogLine } from '@forge/ui';
import { loadRemote, Remote } from '@forge/remote';
import { api } from '../api';

/* Component federation: fetch this backend's /api/components manifest
   (JWT-authenticated), blob-import the bundle, mount its web components.
   Toggle the theme in the top bar — the remotes recolor live because design
   tokens inherit through the shadow boundary. */
export default function RemoteDemo() {
  const [events, setEvents] = createSignal([]);
  const log = (kind, detail) =>
    setEvents((xs) => [...xs.slice(-9), { time: new Date().toTimeString().slice(0, 8), kind, detail: JSON.stringify(detail) }]);

  const [handle] = createResource(() =>
    loadRemote('/api/components', { headers: api.auth.header() }));

  const series = [12, 18, 9, 24, 31, 22, 27, 35, 30, 42];

  return (
    <>
      <PageHead title="Remote components" sub="Web-component bundle fetched from /api/components and mounted in shadow DOM" />
      <Switch>
        <Match when={handle.error}>
          <Alert tone="warning" title="No remote bundle">
            The backend has no components dir configured (or the fetch failed:
            {' '}{String(handle.error?.message ?? handle.error)}). Run the demo backends —
            they serve the apps/remote-widgets bundle.
          </Alert>
        </Match>
        <Match when={handle()}>
          <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
            <div>
              <Badge tone="accent">app: {handle().manifest.app}</Badge>{' '}
              <Badge>{handle().manifest.components.length} components</Badge>
            </div>
            <Grid>
              <Remote
                tag={handle().get('status-card')?.tag}
                props={{ title: 'Remote status card', status: 'success', message: 'Served by another app' }}
                on={{ refresh: (e) => log('refresh', e.detail) }}
              />
              <Remote
                tag={handle().get('metrics-panel')?.tag}
                props={{ title: 'Remote metrics', series, unit: 'ms' }}
                on={{ select: (e) => log('select', e.detail) }}
              />
            </Grid>
            <Card title="CustomEvents from the remotes">
              <Logs style={{ height: '140px' }}>
                <For each={events()} fallback={<LogLine time="—" level="debug">interact with the widgets above</LogLine>}>
                  {(e) => <LogLine time={e.time} level="info">{e.kind}: {e.detail}</LogLine>}
                </For>
              </Logs>
            </Card>
          </div>
        </Match>
      </Switch>
    </>
  );
}
