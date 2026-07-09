import { createResource, createSignal, For } from 'solid-js';
import { Badge, Button, Card, PageHead, Table, toast } from '@forge/ui';
import { api } from '../../api';
import ConfirmModal from '../../components/ConfirmModal';

export default function SessionsPage() {
  const [sessions, { refetch }] = createResource(() => api.get('/api/admin/sessions'));
  const [revoking, setRevoking] = createSignal(null);
  const when = (ts) => new Date(ts * 1000).toLocaleString();

  const revoke = async () => {
    try {
      await api.del(`/api/admin/sessions/${revoking().id}`);
      toast('Session revoked');
      setRevoking(null);
      refetch();
    } catch (err) {
      toast(`Revoke failed: ${err?.message ?? err}`);
    }
  };

  return (
    <>
      <PageHead title="Sessions" sub="Active browser sessions across all users" />
      <Card>
        <Table>
          <thead>
            <tr><th>User</th><th>Method</th><th>Signed in</th><th>Last seen</th><th>Expires</th><th /></tr>
          </thead>
          <tbody>
            <For each={sessions() ?? []}>
              {(s) => (
                <tr>
                  <td>{s.username ?? s.user_id}</td>
                  <td><For each={s.amr ?? []}>{(m) => <Badge>{m}</Badge>}</For></td>
                  <td>{when(s.created_at)}</td>
                  <td>{when(s.last_seen)}</td>
                  <td>{when(s.expires_at)}</td>
                  <td style={{ 'text-align': 'right' }}>
                    <Button size="sm" variant="danger" onClick={() => setRevoking(s)}>Revoke</Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </Table>
      </Card>
      <ConfirmModal
        open={!!revoking()}
        title={`Revoke session for ${revoking()?.username}?`}
        confirmLabel="Revoke"
        onCancel={() => setRevoking(null)}
        onConfirm={revoke}
      >
        The user is signed out of that browser immediately.
      </ConfirmModal>
    </>
  );
}
