import { AppShell, NavLink, NavSection } from '@forge/ui';
import { useLocation, useNavigate } from '@solidjs/router';
import { KeyRound, MonitorSmartphone, Network, Shield, Users } from 'lucide-solid';
import { api } from '../../api';

const NAV = [
  { href: '/admin/users', label: 'Users', icon: Users },
  { href: '/admin/roles', label: 'Roles', icon: Shield },
  { href: '/admin/clients', label: 'Clients', icon: KeyRound },
  { href: '/admin/providers', label: 'Providers', icon: Network },
  { href: '/admin/sessions', label: 'Sessions', icon: MonitorSmartphone },
];

export default function AdminLayout(props) {
  const location = useLocation();
  const navigate = useNavigate();

  const go = (e, href) => {
    e.preventDefault();
    navigate(href);
  };

  const logout = async () => {
    await api.post('/api/logout').catch(() => {});
    window.location.assign('/login');
  };

  return (
    <AppShell
      topbar={
        <div style={{ display: 'flex', 'justify-content': 'space-between', 'align-items': 'center', width: '100%' }}>
          <strong>forge-auth</strong>
          <div style={{ display: 'flex', gap: 'var(--sp-3)' }}>
            <a href="/account" onClick={(e) => go(e, '/account')}>Account</a>
            <a href="/login" onClick={(e) => { e.preventDefault(); logout(); }}>Sign out</a>
          </div>
        </div>
      }
      sidebar={
        <NavSection title="Identity">
          {NAV.map((item) => (
            <NavLink
              href={item.href}
              icon={item.icon}
              active={location.pathname.startsWith(item.href)}
              onClick={(e) => go(e, item.href)}
            >
              {item.label}
            </NavLink>
          ))}
        </NavSection>
      }
    >
      <div style={{ padding: 'var(--sp-5)', display: 'grid', gap: 'var(--sp-4)' }}>{props.children}</div>
    </AppShell>
  );
}
