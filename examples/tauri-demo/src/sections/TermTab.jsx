import { createSignal, Show } from 'solid-js';
import { PageHead, Card, Badge, Button } from '@forge/ui';
import { Terminal } from '@forge/term';
import { api } from '../api';

const TONES = { ready: 'success', connecting: 'warning', error: 'danger', closed: 'neutral' };

/* A real local shell inside the app: the forge-core PTY engine runs in the
   Rust side, frames ride an IPC channel instead of a WebSocket. The factory
   form of `transport` gives every (re)connect a fresh session. */
export default function TermTab() {
  const [status, setStatus] = createSignal('disconnected');
  const [exit, setExit] = createSignal(null);
  let term;

  const busy = () => status() === 'connecting' || status() === 'ready';
  const reconnect = () => {
    setExit(null);
    term?.connect();
    term?.focus();
  };

  return (
    <div style={{ display: 'grid', gap: 'var(--sp-5)' }}>
      <PageHead title="Terminal" sub="Local PTY over Tauri IPC (widget_open / channel frames)" />
      <Card
        title="Session"
        action={
          <Badge tone={TONES[status()] ?? 'neutral'}>
            ipc:term · {status()}{exit() !== null ? ` (exit ${exit()})` : ''}
          </Badge>
        }
      >
        <div style={{ display: 'grid', gap: 'var(--sp-4)' }}>
          <Show when={!busy()}>
            <div>
              <Button variant="primary" onClick={reconnect}>Reconnect</Button>
            </div>
          </Show>
          {/* webgl=false: xterm's WebGL addon composites a black canvas under
              webkitgtk; the DOM renderer is fine (web browsers are unaffected). */}
          <Terminal
            transport={() => api.widget('term')}
            mode="local"
            webgl={false}
            ref={(t) => { term = t; }}
            onStatus={setStatus}
            onExit={setExit}
            height="420px"
          />
        </div>
      </Card>
    </div>
  );
}
