import { Show, createSignal } from 'solid-js';
import { PageHead, Card, Badge, Button, Alert, Input, Select } from '@forge/ui';
import { Terminal } from '@forge/term';
import { api } from '../api';

const TONES = { ready: 'success', connecting: 'warning', error: 'danger', closed: 'neutral' };

/* Interactive terminal over /api/term: binary tty bytes + JSON control frames
   on one WebSocket. The backend only mounts the route when built with the
   `term` cargo feature AND FORGE_TERM_ENABLE is set — a failed connect here
   is the expected shape of "feature off". */
export default function TermDemo() {
  const [mode, setMode] = createSignal('local');
  const [host, setHost] = createSignal('');
  const [port, setPort] = createSignal('22');
  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [status, setStatus] = createSignal('disconnected');
  const [exit, setExit] = createSignal(null);
  let term;

  const busy = () => status() === 'connecting' || status() === 'ready';
  const connect = () => {
    setExit(null);
    term?.connect();
    term?.focus();
  };

  return (
    <>
      <PageHead title="Terminal" sub="Local PTY or SSH session over /api/term (opt-in backend feature)" />
      <Card
        title="Session"
        action={
          <Badge tone={TONES[status()] ?? 'neutral'}>
            /api/term · {status()}{exit() !== null ? ` (exit ${exit()})` : ''}
          </Badge>
        }
      >
        <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
          <div style={{ display: 'flex', gap: '12px', 'flex-wrap': 'wrap', 'align-items': 'end' }}>
            <Select label="Mode" value={mode()} onChange={setMode} disabled={busy()}
                    options={[
                      { value: 'local', label: 'Local shell' },
                      { value: 'ssh', label: 'SSH' },
                    ]} />
            <Show when={mode() === 'ssh'}>
              <Input label="Host" value={host()} placeholder="host or ip"
                     onInput={(e) => setHost(e.currentTarget.value)} />
              <Input label="Port" value={port()} style={{ width: '80px' }}
                     onInput={(e) => setPort(e.currentTarget.value)} />
              <Input label="User" value={username()}
                     onInput={(e) => setUsername(e.currentTarget.value)} />
              <Input label="Password" type="password" value={password()}
                     onInput={(e) => setPassword(e.currentTarget.value)} />
            </Show>
            <Show when={!busy()} fallback={
              <Button onClick={() => term?.disconnect()}>Disconnect</Button>
            }>
              <Button variant="primary" onClick={connect}
                      disabled={mode() === 'ssh' && (!host() || !username())}>
                Connect
              </Button>
            </Show>
          </div>

          <Show when={status() === 'error'}>
            <Alert tone="warning" title="Session failed">
              The backend closed or refused the session. The terminal needs a
              backend built with the <code>term</code> (SSH: <code>term-ssh</code>)
              cargo feature and started with <code>FORGE_TERM_ENABLE=1</code>;
              SSH targets must also pass the host allowlist. See
              <code> docs/widgets-protocol.md</code>.
            </Alert>
          </Show>

          <Terminal
            url={api.wsUrl('/api/term')}
            mode={mode()}
            host={host() || undefined}
            port={parseInt(port(), 10) || undefined}
            username={username() || undefined}
            password={password() || undefined}
            autoConnect={false}
            ref={(t) => { term = t; }}
            onStatus={setStatus}
            onExit={setExit}
          />
        </div>
      </Card>
    </>
  );
}
