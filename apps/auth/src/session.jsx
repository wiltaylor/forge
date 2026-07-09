import { createContext, createResource, useContext, Show } from 'solid-js';
import { Navigate, useLocation } from '@solidjs/router';
import { Spinner } from '@forge/ui';
import { api, onUnauthorized } from './api';

const SessionContext = createContext();

export function SessionProvider(props) {
  const [session, { refetch }] = createResource(() =>
    api.get('/api/session').catch(() => ({ authenticated: false })),
  );
  onUnauthorized(() => refetch());
  return (
    <SessionContext.Provider value={{ session, refetch }}>
      {props.children}
    </SessionContext.Provider>
  );
}

export function useSession() {
  return useContext(SessionContext);
}

function Gate(props) {
  const { session } = useSession();
  const location = useLocation();
  const loginUrl = () => `/login?return_to=${encodeURIComponent(location.pathname)}`;
  return (
    <Show
      when={!session.loading}
      fallback={
        <div style={{ display: 'grid', 'place-items': 'center', height: '100vh' }}>
          <Spinner />
        </div>
      }
    >
      <Show when={session()?.authenticated} fallback={<Navigate href={loginUrl()} />}>
        <Show when={!props.role || session()?.user?.roles?.includes(props.role)}
              fallback={<Navigate href="/account" />}>
          {props.children}
        </Show>
      </Show>
    </Show>
  );
}

export function RequireAuth(props) {
  return <Gate>{props.children}</Gate>;
}

export function RequireAdmin(props) {
  return <Gate role="admin">{props.children}</Gate>;
}
