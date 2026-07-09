import { Show, createSignal } from 'solid-js';
import { PageHead, Card, Badge, Button, Alert, Input, ToggleGroup } from '@forge/ui';
import { DesktopViewer } from '@forge/desktop';
import { api } from '../api';

const TONES = { ready: 'success', connecting: 'warning', error: 'danger', closed: 'neutral' };
const DEFAULT_PORTS = { vnc: 5900, rdp: 3389 };

/* Remote desktop over /api/desktop/{vnc,rdp}: the backend decodes the
   protocol and streams raw RGBA rects; the widget blits and forwards input.
   Never auto-connects — a viewer always needs an explicit target. The routes
   only exist when the backend is built with the `vnc`/`rdp` cargo features
   AND FORGE_VNC_ENABLE / FORGE_RDP_ENABLE are set. */
export default function DesktopDemo() {
  const [proto, setProto] = createSignal('vnc');
  const [host, setHost] = createSignal('');
  const [port, setPort] = createSignal('');
  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [scale, setScale] = createSignal('fit');
  const [status, setStatus] = createSignal('disconnected');
  let viewer;

  const busy = () => status() === 'connecting' || status() === 'ready';
  const switchProto = (p) => {
    if (busy()) viewer?.disconnect();
    setProto(p);
  };
  const connect = () => {
    viewer?.connect();
    viewer?.focus();
  };
  const canConnect = () => host() && (proto() === 'vnc' || (username() && password()));

  return (
    <>
      <PageHead title="Remote desktop" sub="VNC / RDP viewer over /api/desktop (opt-in backend features)" />
      <Card
        title="Viewer"
        action={
          <Badge tone={TONES[status()] ?? 'neutral'}>
            /api/desktop/{proto()} · {status()}
          </Badge>
        }
      >
        <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
          <div style={{ display: 'flex', gap: '12px', 'flex-wrap': 'wrap', 'align-items': 'end' }}>
            <div class="ffield">
              <span class="ffield-label">Protocol</span>
              <div>
                <ToggleGroup value={proto()} onChange={switchProto}
                             options={[
                               { value: 'vnc', label: 'VNC' },
                               { value: 'rdp', label: 'RDP' },
                             ]} />
              </div>
            </div>
            <Input label="Host" value={host()} placeholder="host or ip"
                   onInput={(e) => setHost(e.currentTarget.value)} />
            <Input label="Port" value={port()} style={{ width: '80px' }}
                   placeholder={String(DEFAULT_PORTS[proto()])}
                   onInput={(e) => setPort(e.currentTarget.value)} />
            <Input label={proto() === 'vnc' ? 'User (unused)' : 'User'} value={username()}
                   onInput={(e) => setUsername(e.currentTarget.value)} />
            <Input label="Password" type="password" value={password()}
                   onInput={(e) => setPassword(e.currentTarget.value)} />
            <div class="ffield">
              <span class="ffield-label">Scale</span>
              <div>
                <ToggleGroup value={scale()} onChange={setScale}
                             options={[
                               { value: 'fit', label: 'Fit' },
                               { value: 'native', label: '1:1' },
                             ]} />
              </div>
            </div>
            <Show when={!busy()} fallback={
              <Button onClick={() => viewer?.disconnect()}>Disconnect</Button>
            }>
              <Button variant="primary" onClick={connect} disabled={!canConnect()}>
                Connect
              </Button>
            </Show>
          </div>

          <Show when={status() === 'error'}>
            <Alert tone="warning" title="Connection failed">
              The viewer shows the backend's error message. Common causes: the
              backend was not built with the <code>{proto()}</code> cargo feature
              or started without <code>FORGE_{proto().toUpperCase()}_ENABLE=1</code>,
              the target host is not in <code>FORGE_DESKTOP_ALLOW_HOSTS</code>, or
              the credentials were refused. See <code>docs/widgets-protocol.md</code>.
            </Alert>
          </Show>

          <DesktopViewer
            url={api.wsUrl(`/api/desktop/${proto()}`)}
            host={host() || undefined}
            port={parseInt(port(), 10) || DEFAULT_PORTS[proto()]}
            username={username() || undefined}
            password={password() || undefined}
            scale={scale()}
            ref={(v) => { viewer = v; }}
            onStatus={setStatus}
          />
        </div>
      </Card>
    </>
  );
}
