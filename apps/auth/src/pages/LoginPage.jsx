import { createResource, createSignal, For, Show } from 'solid-js';
import { useSearchParams } from '@solidjs/router';
import { Alert, Button, Card, Input, Separator } from '@forge/ui';
import { api } from '../api';

const ERROR_MESSAGES = {
  upstream_failed: 'Sign-in with the external provider failed. Try again or use a password.',
  link_denied: 'This external account is not linked to a user here.',
  access_denied: 'Access was denied.',
};

export default function LoginPage() {
  const [params] = useSearchParams();
  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [error, setError] = createSignal(params.error ? (ERROR_MESSAGES[params.error] ?? params.error) : null);
  const [busy, setBusy] = createSignal(false);

  // Context: client name + providers for a pending /authorize request, or the
  // plain provider list for a direct visit.
  const [info] = createResource(() =>
    (params.request
      ? api.get(`/api/login/request/${params.request}`)
      : api.get('/api/login/providers')
    ).catch(() => ({ providers: [], dev_login: false })),
  );

  const go = (redirectTo) => {
    // Server-supplied path, or a local return_to (never an absolute URL).
    const returnTo = params.return_to;
    if (!params.request && returnTo && returnTo.startsWith('/') && !returnTo.startsWith('//')) {
      window.location.assign(returnTo);
    } else {
      window.location.assign(redirectTo);
    }
  };

  const submit = async (e) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const data = await api.post('/api/login', {
        username: username(),
        password: password(),
        request_id: params.request,
      });
      go(data.redirect_to);
    } catch (err) {
      setError(err?.status === 401 ? 'Invalid username or password.' : `Login failed: ${err?.message ?? err}`);
      setBusy(false);
    }
  };

  const upstreamUrl = (slug) =>
    `/api/login/upstream/${slug}${params.request ? `?request=${params.request}` : ''}`;

  return (
    <div style={{ display: 'grid', 'place-items': 'center', 'min-height': '100vh', padding: 'var(--sp-4)' }}>
      <div style={{ width: '360px', 'max-width': '100%', display: 'grid', gap: 'var(--sp-4)' }}>
        <Card title={info()?.client_name ? `Sign in to ${info().client_name}` : 'Sign in'}>
          <form onSubmit={submit} style={{ display: 'grid', gap: 'var(--sp-4)' }}>
            <Input
              label="Username"
              value={username()}
              autocomplete="username"
              onInput={(e) => setUsername(e.currentTarget.value)}
            />
            <Input
              label="Password"
              type="password"
              value={password()}
              autocomplete="current-password"
              onInput={(e) => setPassword(e.currentTarget.value)}
            />
            <Show when={error()}>
              <Alert tone="danger">{error()}</Alert>
            </Show>
            <Button variant="primary" type="submit" disabled={busy() || !username()}>
              {busy() ? 'Signing in…' : 'Sign in'}
            </Button>
          </form>
          <Show when={info()?.providers?.length}>
            <Separator />
            <div style={{ display: 'grid', gap: 'var(--sp-2)' }}>
              <For each={info().providers}>
                {(p) => (
                  <Button onClick={() => window.location.assign(upstreamUrl(p.slug))}>
                    Sign in with {p.display_name}
                  </Button>
                )}
              </For>
            </div>
          </Show>
        </Card>
        <Show when={info()?.dev_login}>
          <DevLoginPanel requestId={params.request} onError={setError} />
        </Show>
      </div>
    </div>
  );
}

/// Rendered only when the backend reports dev_login (compile-time feature).
function DevLoginPanel(props) {
  const [users] = createResource(() =>
    api.get('/api/login/dev/users').then((d) => d.users).catch(() => []),
  );
  const loginAs = async (user) => {
    try {
      const data = await api.post('/api/login/dev', {
        user_id: user.id,
        request_id: props.requestId,
      });
      window.location.assign(data.redirect_to);
    } catch (err) {
      props.onError?.(`Dev login failed: ${err?.message ?? err}`);
    }
  };
  return (
    <Card title="Development sign-in">
      <div style={{ display: 'grid', gap: 'var(--sp-3)' }}>
        <Alert tone="warning" title="dev-login build">
          This build has password-less dev login compiled in. Never deploy it to production.
        </Alert>
        <For each={users()}>
          {(u) => (
            <Button onClick={() => loginAs(u)}>
              {u.display_name} ({u.roles.join(', ')})
            </Button>
          )}
        </For>
      </div>
    </Card>
  );
}
