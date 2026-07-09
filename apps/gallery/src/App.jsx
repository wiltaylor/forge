import { Match, Switch, createResource, createSignal, onCleanup } from 'solid-js';
import { Spinner } from '@forge/ui';
import { api } from './api';
import Gallery from './Gallery';
import LoginPage from './LoginPage';

/* Gate: when the backend has auth enabled and we hold no token, show the
   login page. When auth is disabled (or no backend is running — pure
   frontend dev), go straight to the gallery. */
export default function App() {
  const [token, setToken] = createSignal(api.auth.token());
  const [health] = createResource(async () => {
    try {
      return await api.health();
    } catch {
      return null; // no backend — components-only mode
    }
  });

  const offUnauthorized = api.onUnauthorized(() => setToken(null));
  onCleanup(offUnauthorized);

  const needsLogin = () => !!health()?.auth_enabled && !token();

  return (
    <Switch fallback={<Gallery backend={health()} onLogout={() => { api.auth.logout(); setToken(null); }} />}>
      <Match when={health.loading}>
        <div style={{ display: 'grid', 'place-items': 'center', height: '100vh' }}>
          <Spinner size={28} />
        </div>
      </Match>
      <Match when={needsLogin()}>
        <LoginPage onLogin={() => setToken(api.auth.token())} />
      </Match>
    </Switch>
  );
}
