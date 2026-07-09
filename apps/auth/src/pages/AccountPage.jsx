import { createResource, createSignal, For, Show } from 'solid-js';
import { Alert, Badge, Button, Card, Input, PageHead, Table, Tabs, toast } from '@forge/ui';
import { api } from '../api';
import { useSession } from '../session';

export default function AccountPage() {
  const [tab, setTab] = createSignal('profile');
  const { refetch } = useSession();
  const [profile, { refetch: reload }] = createResource(() => api.get('/api/me'));

  const logout = async () => {
    await api.post('/api/logout').catch(() => {});
    refetch();
    window.location.assign('/login');
  };

  return (
    <div style={{ 'max-width': '760px', margin: '0 auto', padding: 'var(--sp-6)', display: 'grid', gap: 'var(--sp-4)' }}>
      <PageHead
        title="Your account"
        sub={profile()?.user?.username}
        actions={
          <div style={{ display: 'flex', gap: 'var(--sp-2)' }}>
            <Show when={profile()?.user?.roles?.includes('admin')}>
              <Button onClick={() => window.location.assign('/admin')}>Admin console</Button>
            </Show>
            <Button onClick={logout}>Sign out</Button>
          </div>
        }
      />
      <Tabs
        tabs={[
          { id: 'profile', label: 'Profile' },
          { id: 'password', label: 'Password' },
          { id: 'identities', label: 'Linked identities' },
          { id: 'sessions', label: 'Sessions' },
        ]}
        active={tab()}
        onChange={setTab}
      />
      <Show when={profile()}>
        <Show when={tab() === 'profile'}>
          <ProfileTab profile={profile()} />
        </Show>
        <Show when={tab() === 'password'}>
          <PasswordTab hasPassword={profile().has_password} onDone={reload} />
        </Show>
        <Show when={tab() === 'identities'}>
          <IdentitiesTab profile={profile()} onDone={reload} />
        </Show>
        <Show when={tab() === 'sessions'}>
          <SessionsTab />
        </Show>
      </Show>
    </div>
  );
}

function ProfileTab(props) {
  const user = props.profile.user;
  return (
    <Card title="Profile">
      <div style={{ display: 'grid', gap: 'var(--sp-3)' }}>
        <div>Username: <strong>{user.username}</strong></div>
        <div>Display name: {user.display_name ?? '—'}</div>
        <div>Email: {user.email ?? '—'}</div>
        <div>
          Roles: <For each={user.roles}>{(r) => <Badge>{r}</Badge>}</For>
        </div>
      </div>
    </Card>
  );
}

function PasswordTab(props) {
  const [current, setCurrent] = createSignal('');
  const [next, setNext] = createSignal('');
  const [error, setError] = createSignal(null);

  const submit = async (e) => {
    e.preventDefault();
    setError(null);
    try {
      await api.post('/api/me/password', {
        current_password: props.hasPassword ? current() : undefined,
        new_password: next(),
      });
      toast('Password updated');
      setCurrent('');
      setNext('');
      props.onDone?.();
    } catch (err) {
      setError(err?.message ?? String(err));
    }
  };

  return (
    <Card title={props.hasPassword ? 'Change password' : 'Set a password'}>
      <form onSubmit={submit} style={{ display: 'grid', gap: 'var(--sp-4)', 'max-width': '360px' }}>
        <Show when={props.hasPassword}>
          <Input label="Current password" type="password" value={current()}
                 autocomplete="current-password"
                 onInput={(e) => setCurrent(e.currentTarget.value)} />
        </Show>
        <Input label="New password (min 8 characters)" type="password" value={next()}
               autocomplete="new-password"
               onInput={(e) => setNext(e.currentTarget.value)} />
        <Show when={error()}>
          <Alert tone="danger">{error()}</Alert>
        </Show>
        <Button variant="primary" type="submit" disabled={next().length < 8}>
          Update password
        </Button>
      </form>
    </Card>
  );
}

function IdentitiesTab(props) {
  const unlink = async (identity) => {
    try {
      await api.del(`/api/me/identities/${identity.id}`);
      toast('Identity unlinked');
      props.onDone?.();
    } catch (err) {
      toast(`Unlink failed: ${err?.message ?? err}`, { tone: 'danger' });
    }
  };
  return (
    <Card title="Linked identities">
      <Show when={props.profile.identities.length} fallback={<p>No external identities linked.</p>}>
        <Table>
          <thead>
            <tr><th>Provider</th><th>Subject</th><th>Email</th><th /></tr>
          </thead>
          <tbody>
            <For each={props.profile.identities}>
              {(identity) => (
                <tr>
                  <td>{identity.provider ?? identity.provider_slug}</td>
                  <td>{identity.subject}</td>
                  <td>{identity.email ?? '—'}</td>
                  <td>
                    <Button size="sm" variant="danger" onClick={() => unlink(identity)}>
                      Unlink
                    </Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </Table>
      </Show>
    </Card>
  );
}

function SessionsTab() {
  const [sessions, { refetch }] = createResource(() => api.get('/api/me/sessions'));
  const revoke = async (s) => {
    await api.del(`/api/me/sessions/${s.id}`).catch((err) => toast(`${err?.message ?? err}`, { tone: 'danger' }));
    refetch();
  };
  const when = (ts) => new Date(ts * 1000).toLocaleString();
  return (
    <Card title="Active sessions">
      <Table>
        <thead>
          <tr><th>Signed in</th><th>Last seen</th><th>Method</th><th /></tr>
        </thead>
        <tbody>
          <For each={sessions() ?? []}>
            {(s) => (
              <tr>
                <td>{when(s.created_at)} <Show when={s.current}><Badge tone="success">this session</Badge></Show></td>
                <td>{when(s.last_seen)}</td>
                <td>{(s.amr ?? []).join(', ')}</td>
                <td>
                  <Show when={!s.current}>
                    <Button size="sm" variant="danger" onClick={() => revoke(s)}>Revoke</Button>
                  </Show>
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </Table>
    </Card>
  );
}
