import { createResource, createSignal, For, Show } from 'solid-js';
import { Alert, Badge, Button, Card, Input, ListBox, Modal, PageHead, Table, Textarea, Toggle, toast } from '@forge/ui';
import { api } from '../../api';
import ConfirmModal from '../../components/ConfirmModal';

const ALL_SCOPES = ['openid', 'profile', 'email', 'roles', 'offline_access'];
const ALL_GRANTS = ['authorization_code', 'refresh_token', 'urn:ietf:params:oauth:grant-type:token-exchange'];

export default function ClientsPage() {
  const [clients, { refetch }] = createResource(() => api.get('/api/admin/clients'));
  const [editing, setEditing] = createSignal(null); // null | 'new' | client
  const [deleting, setDeleting] = createSignal(null);
  const [secret, setSecret] = createSignal(null); // {client_id, client_secret}

  const remove = async () => {
    try {
      await api.del(`/api/admin/clients/${deleting().id}`);
      toast('Client deleted');
      setDeleting(null);
      refetch();
    } catch (err) {
      toast(`Delete failed: ${err?.message ?? err}`);
    }
  };

  const regenerate = async (client) => {
    try {
      const data = await api.post(`/api/admin/clients/${client.id}/secret`);
      setSecret(data);
    } catch (err) {
      toast(`${err?.message ?? err}`);
    }
  };

  return (
    <>
      <PageHead
        title="Clients"
        sub="Applications that sign users in via OIDC or receive exchanged tokens"
        actions={<Button variant="primary" onClick={() => setEditing('new')}>New client</Button>}
      />
      <Show when={secret()}>
        <Alert tone="warning" title="Client secret (shown once — copy it now)">
          <code>{secret().client_secret}</code>
        </Alert>
      </Show>
      <Card>
        <Table>
          <thead>
            <tr><th>Client ID</th><th>Name</th><th>Type</th><th>Flags</th><th /></tr>
          </thead>
          <tbody>
            <For each={clients() ?? []}>
              {(client) => (
                <tr>
                  <td><code>{client.id}</code></td>
                  <td>{client.name}</td>
                  <td>{client.client_type}</td>
                  <td>
                    <Show when={client.trusted}><Badge tone="success">trusted</Badge></Show>{' '}
                    <Show when={client.has_legacy_secret}><Badge>legacy hs256</Badge></Show>{' '}
                    <Show when={client.exchange_audiences?.length}><Badge>exchange</Badge></Show>{' '}
                    <Show when={client.disabled}><Badge tone="danger">disabled</Badge></Show>
                  </td>
                  <td style={{ 'text-align': 'right', 'white-space': 'nowrap' }}>
                    <Show when={client.client_type === 'confidential'}>
                      <Button size="sm" onClick={() => regenerate(client)}>New secret</Button>{' '}
                    </Show>
                    <Button size="sm" onClick={() => setEditing(client)}>Edit</Button>{' '}
                    <Button size="sm" variant="danger" onClick={() => setDeleting(client)}>Delete</Button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </Table>
      </Card>

      <Show when={editing()}>
        <ClientModal
          client={editing() === 'new' ? null : editing()}
          onClose={() => setEditing(null)}
          onSaved={(created) => {
            setEditing(null);
            if (created?.client_secret) setSecret(created);
            refetch();
          }}
        />
      </Show>
      <ConfirmModal
        open={!!deleting()}
        title={`Delete client ${deleting()?.name}?`}
        onCancel={() => setDeleting(null)}
        onConfirm={remove}
      >
        Applications using this client can no longer sign users in.
      </ConfirmModal>
    </>
  );
}

function ClientModal(props) {
  const existing = props.client;
  const [id, setId] = createSignal(existing?.id ?? '');
  const [name, setName] = createSignal(existing?.name ?? '');
  const [clientType, setClientType] = createSignal(existing?.client_type ?? 'confidential');
  const [redirectUris, setRedirectUris] = createSignal((existing?.redirect_uris ?? []).join('\n'));
  const [postLogout, setPostLogout] = createSignal((existing?.post_logout_redirect_uris ?? []).join('\n'));
  const [scopes, setScopes] = createSignal(existing?.allowed_scopes ?? ['openid', 'profile', 'email', 'roles']);
  const [grants, setGrants] = createSignal(existing?.allowed_grants ?? ['authorization_code', 'refresh_token']);
  const [exchangeAudiences, setExchangeAudiences] = createSignal((existing?.exchange_audiences ?? []).join('\n'));
  const [roleMappings, setRoleMappings] = createSignal(
    existing?.role_mappings ? JSON.stringify(existing.role_mappings, null, 2) : '',
  );
  const [trusted, setTrusted] = createSignal(existing?.trusted ?? false);
  const [legacySecret, setLegacySecret] = createSignal(existing?.has_legacy_secret ? '__keep__' : '');
  const [disabled, setDisabled] = createSignal(existing?.disabled ?? false);
  const [error, setError] = createSignal(null);

  const lines = (value) => value.split('\n').map((s) => s.trim()).filter(Boolean);

  const save = async () => {
    setError(null);
    let mappings = null;
    if (roleMappings().trim()) {
      try {
        mappings = JSON.parse(roleMappings());
      } catch {
        setError('Role mappings must be a JSON object like {"admin": "superuser"}');
        return;
      }
    }
    const body = {
      id: existing ? undefined : id() || undefined,
      name: name(),
      client_type: clientType(),
      redirect_uris: lines(redirectUris()),
      post_logout_redirect_uris: lines(postLogout()),
      allowed_scopes: scopes(),
      allowed_grants: grants(),
      exchange_audiences: lines(exchangeAudiences()),
      role_mappings: mappings,
      trusted: trusted(),
      disabled: disabled(),
      // undefined = keep the stored secret (JSON.stringify drops the key);
      // empty string = clear it.
      legacy_hs256_secret: legacySecret() === '__keep__' ? undefined : legacySecret(),
    };
    try {
      const created = existing
        ? await api.put(`/api/admin/clients/${existing.id}`, body)
        : await api.post('/api/admin/clients', body);
      toast(existing ? 'Client updated' : 'Client created');
      props.onSaved(created);
    } catch (err) {
      setError(err?.message ?? String(err));
    }
  };

  return (
    <Modal
      open
      onClose={props.onClose}
      title={existing ? `Edit ${existing.name}` : 'New client'}
      footer={
        <div style={{ display: 'flex', gap: 'var(--sp-3)', 'justify-content': 'flex-end' }}>
          <Button onClick={props.onClose}>Cancel</Button>
          <Button variant="primary" onClick={save} disabled={!name()}>Save</Button>
        </div>
      }
    >
      <div style={{ display: 'grid', gap: 'var(--sp-4)', 'max-height': '60vh', 'overflow-y': 'auto' }}>
        <Show when={!existing}>
          <Input label="Client ID (blank = generated)" value={id()} onInput={(e) => setId(e.currentTarget.value)} />
        </Show>
        <Input label="Name" value={name()} onInput={(e) => setName(e.currentTarget.value)} />
        <ListBox
          label="Client type"
          options={[
            { value: 'confidential', label: 'confidential (server-side app, has a secret)' },
            { value: 'public', label: 'public (SPA/native, PKCE only)' },
          ]}
          value={clientType()}
          onChange={setClientType}
        />
        <Textarea
          label="Redirect URIs (one per line, exact match)"
          rows={3}
          value={redirectUris()}
          onInput={(e) => setRedirectUris(e.currentTarget.value)}
        />
        <Textarea
          label="Post-logout redirect URIs (one per line)"
          rows={2}
          value={postLogout()}
          onInput={(e) => setPostLogout(e.currentTarget.value)}
        />
        <ListBox
          label="Allowed scopes"
          multiple
          options={ALL_SCOPES.map((s) => ({ value: s, label: s }))}
          values={scopes()}
          onChange={setScopes}
        />
        <ListBox
          label="Allowed grants"
          multiple
          options={ALL_GRANTS.map((g) => ({ value: g, label: g }))}
          values={grants()}
          onChange={setGrants}
        />
        <Textarea
          label="Token-exchange audiences (client IDs this client may exchange tokens for, one per line; * = any)"
          rows={2}
          value={exchangeAudiences()}
          onInput={(e) => setExchangeAudiences(e.currentTarget.value)}
        />
        <Textarea
          label='Role mappings (JSON: {"idp role": "emitted role"}; empty = pass all roles through)'
          rows={3}
          value={roleMappings()}
          onInput={(e) => setRoleMappings(e.currentTarget.value)}
        />
        <Input
          label="Legacy forge HS256 secret (min 32 chars; token exchange then mints forge-style HS256 tokens)"
          value={legacySecret() === '__keep__' ? '' : legacySecret()}
          placeholder={existing?.has_legacy_secret ? '(configured — type to replace)' : ''}
          onInput={(e) => setLegacySecret(e.currentTarget.value)}
        />
        <Toggle checked={trusted()} onChange={setTrusted}>Trusted (skip the consent screen)</Toggle>
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
