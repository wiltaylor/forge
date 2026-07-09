import { For, Show, createResource, createSignal } from 'solid-js';
import { PageHead, Card, Grid, Button, Input, Textarea, Table, toast } from '@forge/ui';
import { api } from '../api';

/* The JSON document store: save / load / list / delete via @forge/client. */
export default function DataDemo() {
  const [name, setName] = createSignal('scratchpad');
  const [body, setBody] = createSignal('{\n  "hello": "forge"\n}');
  const [docs, { refetch }] = createResource(() => api.data.list().catch(() => []));

  const save = async () => {
    try {
      await api.data.put(name(), JSON.parse(body()));
      toast(`Saved ${name()}`, { tone: 'success' });
      refetch();
    } catch (e) {
      toast(`Save failed: ${e.message}`, { tone: 'danger' });
    }
  };
  const load = async (n) => {
    const doc = await api.data.get(n);
    if (doc === null) return toast(`${n} not found`, { tone: 'warning' });
    setName(n);
    setBody(JSON.stringify(doc, null, 2));
  };
  const del = async (n) => {
    await api.data.del(n);
    toast(`Deleted ${n}`);
    refetch();
  };

  return (
    <>
      <PageHead title="Document store" sub="GET/PUT/DELETE /api/data/{name} — atomic JSON docs" />
      <Grid>
        <Card title="Editor" action={<Button variant="primary" onClick={save}>Save</Button>}>
          <div style={{ display: 'grid', gap: 'var(--sp-3)' }}>
            <Input label="Document name" value={name()} onInput={(e) => setName(e.currentTarget.value)}
                   help="lowercase letters, digits, - and _" />
            <Textarea label="JSON body" rows="8" value={body()} class="mono"
                      onInput={(e) => setBody(e.currentTarget.value)} />
          </div>
        </Card>
        <Card title="Documents" action={<Button size="sm" onClick={refetch}>Refresh</Button>}>
          <Show when={docs()?.length} fallback={<small>No documents yet — save one.</small>}>
            <Table>
              <thead>
                <tr><th>Name</th><th>Bytes</th><th></th></tr>
              </thead>
              <tbody>
                <For each={docs()}>
                  {(d) => (
                    <tr>
                      <td class="mono">{d.name}</td>
                      <td>{d.bytes}</td>
                      <td style={{ 'text-align': 'right' }}>
                        <Button size="sm" onClick={() => load(d.name)}>Load</Button>{' '}
                        <Button size="sm" variant="ghost" onClick={() => del(d.name)}>Delete</Button>
                      </td>
                    </tr>
                  )}
                </For>
              </tbody>
            </Table>
          </Show>
        </Card>
      </Grid>
    </>
  );
}
