import { createResource, createSignal, For, Show } from 'solid-js';
import { Alert, Badge, Button, Card, Input, ListBox, Modal, PageHead, Table, Toggle, toast } from '@forge/ui';
import { api } from '../../api';
import ConfirmModal from '../../components/ConfirmModal';

// Presets fill kind + config skeleton; Entra covers Azure AD, LDAP covers
// on-prem Active Directory.
const PRESETS = {
  oidc: { label: 'Generic OIDC', config: { issuer_url: '', client_id: '', client_secret: '', scopes: 'openid profile email' } },
  google: { label: 'Google', kind: 'oidc', config: { issuer_url: 'https://accounts.google.com', client_id: '', client_secret: '', scopes: 'openid profile email' } },
  microsoft: { label: 'Microsoft Entra ID (Azure AD)', kind: 'oidc', config: { issuer_url: 'https://login.microsoftonline.com/{tenant}/v2.0', client_id: '', client_secret: '', scopes: 'openid profile email' } },
  github: { label: 'GitHub', kind: 'github', config: { client_id: '', client_secret: '' } },
  ldap: {
    label: 'LDAP / Active Directory',
    kind: 'ldap',
    config: {
      url: 'ldaps://dc01.example.lan:636',
      bind_dn: 'CN=svc-forge,OU=Service Accounts,DC=example,DC=lan',
      bind_password: '',
      base_dn: 'DC=example,DC=lan',
      user_filter: '(&(objectClass=user)(sAMAccountName={username}))',
      email_attr: 'mail',
      display_name_attr: 'displayName',
      starttls: false,
    },
  },
};

export default function ProvidersPage() {
  const [providers, { refetch }] = createResource(() => api.get('/api/admin/providers'));
  const [roles] = createResource(() => api.get('/api/admin/roles'));
  const [editing, setEditing] = createSignal(null); // null | preset-key | provider
  const [deleting, setDeleting] = createSignal(null);

  const remove = async () => {
    try {
      await api.del(`/api/admin/providers/${deleting().id}`);
      toast('Provider deleted');
      setDeleting(null);
      refetch();
    } catch (err) {
      toast(`Delete failed: ${err?.message ?? err}`);
    }
  };

  const test = async (provider) => {
    try {
      const data = await api.post(`/api/admin/providers/${provider.id}/test`);
      toast(data.ok ? `OK: ${data.detail}` : `Failed: ${data.detail}`);
    } catch (err) {
      toast(`Test failed: ${err?.message ?? err}`);
    }
  };

  return (
    <>
      <PageHead
        title="Identity providers"
        sub="Upstream login sources: OAuth/OIDC federation and LDAP/Active Directory"
        actions={
          <div style={{ display: 'flex', gap: 'var(--sp-2)', 'flex-wrap': 'wrap' }}>
            <For each={Object.entries(PRESETS)}>
              {([key, preset]) => (
                <Button size="sm" onClick={() => setEditing(key)}>+ {preset.label}</Button>
              )}
            </For>
          </div>
        }
      />
      <Card>
        <Table>
          <thead>
            <tr><th>Name</th><th>Slug</th><th>Kind</th><th>Options</th><th /></tr>
          </thead>
          <tbody>
            <For each={providers() ?? []}>
              {(provider) => (
                <tr>
                  <td>{provider.display_name}</td>
                  <td><code>{provider.slug}</code></td>
                  <td><Badge>{provider.kind}</Badge></td>
                  <td>
                    <Show when={!provider.enabled}><Badge tone="danger">disabled</Badge></Show>{' '}
                    <Show when={provider.allow_signup}><Badge>auto-signup</Badge></Show>{' '}
                    <Show when={provider.link_by_email}><Badge>link-by-email</Badge></Show>
                  </td>
                  <td style={{ 'text-align': 'right', 'white-space': 'nowrap' }}>
                    <Button size="sm" onClick={() => test(provider)}>Test</Button>{' '}
                    <Button size="sm" onClick={() => setEditing(provider)}>Edit</Button>{' '}
                    <Button size="sm" variant="danger" onClick={() => setDeleting(provider)}>Delete</Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </Table>
      </Card>

      <Show when={editing()}>
        <ProviderModal
          preset={typeof editing() === 'string' ? editing() : null}
          provider={typeof editing() === 'object' ? editing() : null}
          allRoles={(roles() ?? []).map((r) => r.name)}
          onClose={() => setEditing(null)}
          onSaved={() => { setEditing(null); refetch(); }}
        />
      </Show>
      <ConfirmModal
        open={!!deleting()}
        title={`Delete provider ${deleting()?.display_name}?`}
        onCancel={() => setDeleting(null)}
        onConfirm={remove}
      >
        Users who sign in through this provider lose that login method (their accounts stay).
      </ConfirmModal>
    </>
  );
}

function ProviderModal(props) {
  const preset = props.preset ? PRESETS[props.preset] : null;
  const existing = props.provider;
  const kind = existing?.kind ?? preset?.kind ?? props.preset ?? 'oidc';

  const [slug, setSlug] = createSignal(existing?.slug ?? (props.preset === 'oidc' ? '' : props.preset ?? ''));
  const [displayName, setDisplayName] = createSignal(existing?.display_name ?? preset?.label ?? '');
  const [enabled, setEnabled] = createSignal(existing?.enabled ?? true);
  const [allowSignup, setAllowSignup] = createSignal(existing?.allow_signup ?? true);
  const [linkByEmail, setLinkByEmail] = createSignal(existing?.link_by_email ?? false);
  const [config, setConfig] = createSignal({ ...(preset?.config ?? {}), ...(existing?.config ?? {}) });
  const [mappings, setMappings] = createSignal([]);
  const [error, setError] = createSignal(null);

  // The list payload has no group mappings; pull the detail when editing.
  if (existing) {
    api.get(`/api/admin/providers/${existing.id}`).then((detail) => {
      setMappings(detail.group_mappings ?? []);
    }).catch(() => {});
  }

  // Group mappings are edited as "external-group → role" rows.
  const [newGroup, setNewGroup] = createSignal('');
  const [newRole, setNewRole] = createSignal(props.allRoles[0] ?? '');

  const setConfigKey = (key, value) => setConfig({ ...config(), [key]: value });

  const save = async () => {
    setError(null);
    try {
      await api.post('/api/admin/providers', {
        slug: slug(),
        kind,
        display_name: displayName(),
        enabled: enabled(),
        allow_signup: allowSignup(),
        link_by_email: linkByEmail(),
        config: config(),
        group_mappings: mappings().map((m) => ({ external_group: m.external_group, role: m.role })),
      });
      toast('Provider saved');
      props.onSaved();
    } catch (err) {
      setError(err?.message ?? String(err));
    }
  };

  const configField = (key, label, type = 'text') => (
    <Input
      label={label}
      type={type}
      value={config()[key] ?? ''}
      onInput={(e) => setConfigKey(key, e.currentTarget.value)}
    />
  );

  return (
    <Modal
      open
      onClose={props.onClose}
      title={existing ? `Edit ${existing.display_name}` : `New ${preset?.label ?? kind} provider`}
      footer={
        <div style={{ display: 'flex', gap: 'var(--sp-3)', 'justify-content': 'flex-end' }}>
          <Button onClick={props.onClose}>Cancel</Button>
          <Button variant="primary" onClick={save} disabled={!slug() || !displayName()}>Save</Button>
        </div>
      }
    >
      <div style={{ display: 'grid', gap: 'var(--sp-4)', 'max-height': '60vh', 'overflow-y': 'auto' }}>
        <Input label="Slug (stable id, used in URLs)" value={slug()} disabled={!!existing}
               onInput={(e) => setSlug(e.currentTarget.value)} />
        <Input label="Display name (shown on the login button)" value={displayName()}
               onInput={(e) => setDisplayName(e.currentTarget.value)} />

        <Show when={kind === 'oidc'}>
          {configField('issuer_url', 'Issuer URL (discovery base)')}
          {configField('client_id', 'Client ID')}
          {configField('client_secret', 'Client secret', 'password')}
          {configField('scopes', 'Scopes (space separated)')}
          {configField('groups_claim', 'Groups claim (optional, e.g. groups for Entra)')}
        </Show>
        <Show when={kind === 'github'}>
          {configField('client_id', 'Client ID')}
          {configField('client_secret', 'Client secret', 'password')}
        </Show>
        <Show when={kind === 'ldap'}>
          {configField('url', 'Server URL (ldaps://host:636 or ldap://host:389)')}
          <Toggle checked={!!config().starttls} onChange={(v) => setConfigKey('starttls', v)}>
            Use StartTLS (for ldap:// URLs)
          </Toggle>
          {configField('bind_dn', 'Service account bind DN')}
          {configField('bind_password', 'Service account password', 'password')}
          {configField('base_dn', 'Search base DN')}
          {configField('user_filter', 'User filter ({username} is substituted)')}
          {configField('email_attr', 'Email attribute')}
          {configField('display_name_attr', 'Display name attribute')}
        </Show>

        <Toggle checked={enabled()} onChange={setEnabled}>Enabled</Toggle>
        <Toggle checked={allowSignup()} onChange={setAllowSignup}>
          Auto-provision new users on first login
        </Toggle>
        <Toggle checked={linkByEmail()} onChange={setLinkByEmail}>
          Link to existing users by verified email
        </Toggle>

        <Card title="Group → role mappings" padded>
          <div style={{ display: 'grid', gap: 'var(--sp-3)' }}>
            <For each={mappings()}>
              {(m, i) => (
                <div style={{ display: 'flex', gap: 'var(--sp-2)', 'align-items': 'center' }}>
                  <code style={{ flex: 1, 'overflow-wrap': 'anywhere' }}>{m.external_group}</code>
                  <span>→</span>
                  <Badge>{m.role}</Badge>
                  <Button size="sm" variant="ghost"
                          onClick={() => setMappings(mappings().filter((_, idx) => idx !== i()))}>
                    ✕
                  </Button>
                </div>
              )}
            </For>
            <div style={{ display: 'flex', gap: 'var(--sp-2)', 'align-items': 'end' }}>
              <div style={{ flex: 1 }}>
                <Input label={kind === 'ldap' ? 'Group DN' : 'Group claim value'} value={newGroup()}
                       onInput={(e) => setNewGroup(e.currentTarget.value)} />
              </div>
              <div style={{ 'min-width': '140px' }}>
                <ListBox
                  label="Role"
                  options={props.allRoles.map((r) => ({ value: r, label: r }))}
                  value={newRole()}
                  onChange={setNewRole}
                />
              </div>
              <Button
                disabled={!newGroup() || !newRole()}
                onClick={() => {
                  setMappings([...mappings(), { external_group: newGroup(), role: newRole() }]);
                  setNewGroup('');
                }}
              >
                Add
              </Button>
            </div>
          </div>
        </Card>

        <Show when={error()}>
          <Alert tone="danger">{error()}</Alert>
        </Show>
      </div>
    </Modal>
  );
}
