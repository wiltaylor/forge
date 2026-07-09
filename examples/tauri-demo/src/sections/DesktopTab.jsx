import { createSignal, Show } from 'solid-js';
import { PageHead, Card, Badge, Button, Alert, Input, ToggleGroup } from '@forge/ui';
import { DesktopViewer } from '@forge/desktop';
import { api } from '../api';

const TONES = { ready: 'success', connecting: 'warning', error: 'danger', closed: 'neutral' };
const DEFAULT_PORTS = { vnc: 5900, rdp: 3389 };

/* VNC/RDP over IPC: the Rust side speaks the desktop protocol and streams
   RGBA rects down the session channel. Handy targets: the repo's
   `just widgets-testenv-up` docker pair — VNC 127.0.0.1:5900 (password
   "forge"), RDP 127.0.0.1:3389 (forge / forge). */
export default function DesktopTab() {
  const [proto, setProto] = createSignal('vnc');
  const [host, setHost] = createSignal('127.0.0.1');
  const [port, setPort] = createSignal('');
  const [username, setUsername] = createSignal('forge');
  const [password, setPassword] = createSignal('forge');
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
    <div style={{ display: 'grid', gap: 'var(--sp-5)' }}>
      <PageHead title="Remote desktop" sub="VNC / RDP viewer over Tauri IPC" />
      <Card
        title="Viewer"
        action={
          <Badge tone={TONES[status()] ?? 'neutral'}>
            ipc:{proto()} · {status()}
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
              No listener on the target, wrong credentials, or the host is not
              in the allowlist. Start the repo's docker targets with
              <code> just widgets-testenv-up</code>.
            </Alert>
          </Show>

          <DesktopViewer
            transport={() => api.widget(proto())}
            host={host() || undefined}
            port={parseInt(port(), 10) || undefined}
            username={username() || undefined}
            password={password() || undefined}
            scale={scale()}
            ref={(v) => { viewer = v; }}
            onStatus={setStatus}
            height="520px"
          />
        </div>
      </Card>
    </div>
  );
}
