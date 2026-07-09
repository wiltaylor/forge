import { createResource, createSignal, For, Show } from 'solid-js';
import { Alert, Button, Card, Input, Modal, PageHead, Table, toast } from '@forge/ui';
import { api } from '../../api';
import ConfirmModal from '../../components/ConfirmModal';

export default function RolesPage() {
  const [roles, { refetch }] = createResource(() => api.get('/api/admin/roles'));
  const [creating, setCreating] = createSignal(false);
  const [deleting, setDeleting] = createSignal(null);
  const [name, setName] = createSignal('');
  const [description, setDescription] = createSignal('');
  const [error, setError] = createSignal(null);

  const create = async () => {
    setError(null);
    try {
      await api.post('/api/admin/roles', { name: name(), description: description() || null });
      toast('Role created');
      setCreating(false);
      setName('');
      setDescription('');
      refetch();
    } catch (err) {
      setError(err?.message ?? String(err));
    }
  };

  const remove = async () => {
    try {
      await api.del(`/api/admin/roles/${deleting().id}`);
      toast('Role deleted');
      setDeleting(null);
      refetch();
    } catch (err) {
      toast(`Delete failed: ${err?.message ?? err}`);
    }
  };

  return (
    <>
      <PageHead
        title="Roles"
        sub="Assigned to users and encoded into JWT role claims"
        actions={<Button variant="primary" onClick={() => setCreating(true)}>New role</Button>}
      />
      <Card>
        <Table>
          <thead>
            <tr><th>Name</th><th>Description</th><th /></tr>
          </thead>
          <tbody>
            <For each={roles() ?? []}>
              {(role) => (
                <tr>
                  <td>{role.name}</td>
                  <td>{role.description ?? '—'}</td>
                  <td style={{ 'text-align': 'right' }}>
                    <Button size="sm" variant="danger" onClick={() => setDeleting(role)}>Delete</Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </Table>
      </Card>

      <Modal
        open={creating()}
        onClose={() => setCreating(false)}
        title="New role"
        footer={
          <div style={{ display: 'flex', gap: 'var(--sp-3)', 'justify-content': 'flex-end' }}>
            <Button onClick={() => setCreating(false)}>Cancel</Button>
            <Button variant="primary" onClick={create} disabled={!name()}>Create</Button>
          </div>
        }
      >
        <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
          <Input label="Name" value={name()} onInput={(e) => setName(e.currentTarget.value)} />
          <Input label="Description" value={description()} onInput={(e) => setDescription(e.currentTarget.value)} />
          <Show when={error()}>
            <Alert tone="danger">{error()}</Alert>
          </Show>
        </div>
      </Modal>
      <ConfirmModal
        open={!!deleting()}
        title={`Delete role ${deleting()?.name}?`}
        onCancel={() => setDeleting(null)}
        onConfirm={remove}
      >
        The role is removed from every user that has it.
      </ConfirmModal>
    </>
  );
}
