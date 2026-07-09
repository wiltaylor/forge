import { Show, createSignal } from 'solid-js';
import { Alert, Button, Card, Input } from '@forge/ui';
import { api } from './api';

export default function LoginPage(props) {
  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [error, setError] = createSignal(null);
  const [busy, setBusy] = createSignal(false);

  const submit = async (e) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await api.auth.login(username(), password());
      props.onLogin?.();
    } catch (err) {
      setError(err?.status === 401 ? 'Invalid username or password.' : `Login failed: ${err?.message ?? err}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ display: 'grid', 'place-items': 'center', height: '100vh', padding: 'var(--sp-4)' }}>
      <div style={{ width: '340px', 'max-width': '100%' }}>
        <Card title="Sign in to Forge">
          <form onSubmit={submit} style={{ display: 'grid', gap: 'var(--sp-4)' }}>
            <Input label="Username" value={username()} autocomplete="username"
                   onInput={(e) => setUsername(e.currentTarget.value)} />
            <Input label="Password" type="password" value={password()} autocomplete="current-password"
                   onInput={(e) => setPassword(e.currentTarget.value)} />
            <Show when={error()}>
              <Alert tone="danger">{error()}</Alert>
            </Show>
            <Button variant="primary" type="submit" disabled={busy() || !username()}>
              {busy() ? 'Signing in…' : 'Sign in'}
            </Button>
            <small>Demo credentials come from the backend .env (FORGE_AUTH_USERS, e.g. admin / admin).</small>
          </form>
        </Card>
      </div>
    </div>
  );
}
