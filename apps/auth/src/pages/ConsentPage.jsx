import { createResource, createSignal, For, Show } from 'solid-js';
import { useSearchParams } from '@solidjs/router';
import { Alert, Badge, Button, Card, Spinner } from '@forge/ui';
import { api } from '../api';

const SCOPE_DESCRIPTIONS = {
  openid: 'Confirm your identity',
  profile: 'Read your display name',
  email: 'Read your email address',
  roles: 'Read your assigned roles',
  offline_access: 'Stay signed in (refresh tokens)',
};

export default function ConsentPage() {
  const [params] = useSearchParams();
  const [error, setError] = createSignal(null);
  const [busy, setBusy] = createSignal(false);
  const [info] = createResource(() => api.get(`/api/consent/${params.request}`));

  const decide = async (approve) => {
    setBusy(true);
    setError(null);
    try {
      const data = await api.post(`/api/consent/${params.request}`, { approve });
      window.location.assign(data.redirect_to);
    } catch (err) {
      setError(`${err?.message ?? err}`);
      setBusy(false);
    }
  };

  return (
    <div style={{ display: 'grid', 'place-items': 'center', 'min-height': '100vh', padding: 'var(--sp-4)' }}>
      <div style={{ width: '380px', 'max-width': '100%' }}>
        <Show when={!info.loading} fallback={<Spinner />}>
          <Show when={info()} fallback={<Alert tone="danger">This request has expired. Start again from the application.</Alert>}>
            <Card title={`Authorize ${info().client_name}`}>
              <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
                <p>
                  <strong>{info().client_name}</strong> wants to:
                </p>
                <ul style={{ margin: 0, 'padding-left': 'var(--sp-4)', display: 'grid', gap: 'var(--sp-2)' }}>
                  <For each={info().scopes}>
                    {(scope) => (
                      <li>
                        {SCOPE_DESCRIPTIONS[scope] ?? scope} <Badge>{scope}</Badge>
                      </li>
                    )}
                  </For>
                </ul>
                <Show when={error()}>
                  <Alert tone="danger">{error()}</Alert>
                </Show>
                <div style={{ display: 'flex', gap: 'var(--sp-3)', 'justify-content': 'flex-end' }}>
                  <Button disabled={busy()} onClick={() => decide(false)}>
                    Deny
                  </Button>
                  <Button variant="primary" disabled={busy()} onClick={() => decide(true)}>
                    Allow
                  </Button>
                </div>
              </div>
            </Card>
          </Show>
        </Show>
      </div>
    </div>
  );
}
