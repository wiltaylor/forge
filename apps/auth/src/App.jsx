import { lazy } from 'solid-js';
import { Navigate, Route, Router } from '@solidjs/router';
import { Toaster } from '@forge/ui';
import { RequireAdmin, RequireAuth, SessionProvider } from './session';
import LoginPage from './pages/LoginPage';
import ConsentPage from './pages/ConsentPage';

// Login/consent stay in the entry chunk (latency-critical); the rest is lazy.
const AccountPage = lazy(() => import('./pages/AccountPage'));
const AdminLayout = lazy(() => import('./pages/admin/AdminLayout'));
const UsersPage = lazy(() => import('./pages/admin/UsersPage'));
const RolesPage = lazy(() => import('./pages/admin/RolesPage'));
const ClientsPage = lazy(() => import('./pages/admin/ClientsPage'));
const ProvidersPage = lazy(() => import('./pages/admin/ProvidersPage'));
const SessionsPage = lazy(() => import('./pages/admin/SessionsPage'));

function Root(props) {
  return (
    <SessionProvider>
      {props.children}
      <Toaster />
    </SessionProvider>
  );
}

export default function App() {
  return (
    <Router root={Root}>
      <Route path="/login" component={LoginPage} />
      <Route path="/consent" component={ConsentPage} />
      <Route
        path="/account"
        component={() => (
          <RequireAuth>
            <AccountPage />
          </RequireAuth>
        )}
      />
      <Route
        path="/admin"
        component={(props) => (
          <RequireAdmin>
            <AdminLayout>{props.children}</AdminLayout>
          </RequireAdmin>
        )}
      >
        <Route path="/" component={() => <Navigate href="/admin/users" />} />
        <Route path="/users" component={UsersPage} />
        <Route path="/roles" component={RolesPage} />
        <Route path="/clients" component={ClientsPage} />
        <Route path="/providers" component={ProvidersPage} />
        <Route path="/sessions" component={SessionsPage} />
      </Route>
      <Route path="*" component={() => <Navigate href="/account" />} />
    </Router>
  );
}
