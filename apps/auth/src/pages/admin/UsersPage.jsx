import { createResource, createSignal, For, Show } from 'solid-js';
import { Alert, Badge, Button, Card, Input, ListBox, Modal, PageHead, Table, Toggle, toast } from '@forge/ui';
import { api } from '../../api';
import ConfirmModal from '../../components/ConfirmModal';

export default function UsersPage() {
  const [users, { refetch }] = createResource(() => api.get('/api/admin/users'));
  const [roles] = createResource(() => api.get('/api/admin/roles'));
  const [editing, setEditing] = createSignal(null); // null | 'new' | user
  const [deleting, setDeleting] = createSignal(null);

  const remove = async () => {
    try {
      await api.del(`/api/admin/users/${deleting().id}`);
      toast('User deleted');
      setDeleting(null);
      refetch();
    } catch (err) {
      toast(`Delete failed: ${err?.message ?? err}`);
    }
  };

  return (
    <>
      <PageHead
        title="Users"
        sub="Member database — roles here are encoded into issued tokens"
        actions={<Button variant="primary" onClick={() => setEditing('new')}>New user</Button>}
      />
      <Card>
        <Table>
          <thead>
            <tr><th>Username</th><th>Display name</th><th>Email</th><th>Roles</th><th>Status</th><th /></tr>
          </thead>
          <tbody>
            <For each={users() ?? []}>
              {(user) => (
                <tr>
                  <td>{user.username}</td>
                  <td>{user.display_name ?? '—'}</td>
                  <td>{user.email ?? '—'}</td>
                  <td><For each={user.roles}>{(r) => <Badge>{r}</Badge>}</For></td>
                  <td>{user.disabled ? <Badge tone="danger">disabled</Badge> : <Badge tone="success">active</Badge>}</td>
                  <td style={{ 'text-align': 'right' }}>
                    <Button size="sm" onClick={() => setEditing(user)}>Edit</Button>{' '}
                    <Button size="sm" variant="danger" onClick={() => setDeleting(user)}>Delete</Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </Table>
      </Card>

      <Show when={editing()}>
        <UserModal
          user={editing() === 'new' ? null : editing()}
          allRoles={(roles() ?? []).map((r) => r.name)}
          onClose={() => setEditing(null)}
          onSaved={() => { setEditing(null); refetch(); }}
        />
      </Show>
      <ConfirmModal
        open={!!deleting()}
        title={`Delete ${deleting()?.username}?`}
        onCancel={() => setDeleting(null)}
        onConfirm={remove}
      >
        This permanently removes the user, their credentials, sessions and tokens.
      </ConfirmModal>
    </>
  );
}

function UserModal(props) {
  const existing = props.user;
  const [username, setUsername] = createSignal(existing?.username ?? '');
  const [displayName, setDisplayName] = createSignal(existing?.display_name ?? '');
  const [email, setEmail] = createSignal(existing?.email ?? '');
  const [password, setPassword] = createSignal('');
  const [selectedRoles, setSelectedRoles] = createSignal(existing?.roles ?? []);
  const [disabled, setDisabled] = createSignal(existing?.disabled ?? false);
  const [error, setError] = createSignal(null);

  const save = async () => {
    setError(null);
    try {
      if (existing) {
        await api.put(`/api/admin/users/${existing.id}`, {
          display_name: displayName() || null,
          email: email() || null,
          disabled: disabled(),
        });
        await api.put(`/api/admin/users/${existing.id}/roles`, { roles: selectedRoles() });
        if (password()) {
          await api.put(`/api/admin/users/${existing.id}/password`, { password: password() });
        }
        toast('User updated');
      } else {
        await api.post('/api/admin/users', {
          username: username(),
          display_name: displayName() || null,
          email: email() || null,
          password: password() || null,
          roles: selectedRoles(),
        });
        toast('User created');
      }
      props.onSaved();
    } catch (err) {
      setError(err?.message ?? String(err));
    }
  };

  return (
    <Modal
      open
      onClose={props.onClose}
      title={existing ? `Edit ${existing.username}` : 'New user'}
      footer={
        <div style={{ display: 'flex', gap: 'var(--sp-3)', 'justify-content': 'flex-end' }}>
          <Button onClick={props.onClose}>Cancel</Button>
          <Button variant="primary" onClick={save} disabled={!existing && !username()}>
            Save
          </Button>
        </div>
      }
    >
      <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
        <Show when={!existing}>
          <Input label="Username" value={username()} onInput={(e) => setUsername(e.currentTarget.value)} />
        </Show>
        <Input label="Display name" value={displayName()} onInput={(e) => setDisplayName(e.currentTarget.value)} />
        <Input label="Email" type="email" value={email()} onInput={(e) => setEmail(e.currentTarget.value)} />
        <Input
          label={existing ? 'Reset password (leave blank to keep)' : 'Password (optional — can be set later)'}
          type="password"
          value={password()}
          autocomplete="new-password"
          onInput={(e) => setPassword(e.currentTarget.value)}
        />
        <ListBox
          label="Roles"
          multiple
          options={props.allRoles.map((r) => ({ value: r, label: r }))}
          values={selectedRoles()}
          onChange={setSelectedRoles}
        />
        <Show when={existing}>
          <Toggle checked={disabled()} onChange={setDisabled}>Disabled</Toggle>
        </Show>
        <Show when={error()}>
          <Alert tone="danger">{error()}</Alert>
        </Show>
      </div>
    </Modal>
  );
}
