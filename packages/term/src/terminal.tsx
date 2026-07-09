/* ---------------- Terminal ---------------------------------------------------
   xterm.js over a dedicated widget connection (`/api/term` WebSocket by
   default, or a custom transport): binary frames both ways for tty bytes,
   JSON text frames for control. No auto-reconnect — a dead session stays
   dead until connect() is called again. */
import { createSignal, onMount, onCleanup, Show } from 'solid-js';
import type { JSX } from 'solid-js';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebglAddon } from '@xterm/addon-webgl';
import { readTermTheme, watchTheme } from './theme';
import { connectTransport } from './transport';
import type { WidgetTransport } from './transport';

export type TerminalStatus = 'disconnected' | 'connecting' | 'ready' | 'closed' | 'error';

export interface TerminalApi {
  connect(): void;
  disconnect(): void;
  focus(): void;
  fit(): void;
  /** Write locally into the terminal (not down the wire). */
  write(data: string | Uint8Array): void;
}

export interface TerminalProps {
  /** WebSocket URL, e.g. `api.wsUrl('/api/term')` (token already embedded).
      Optional when `transport` is provided. */
  url?: string;
  /** Custom connection (e.g. `@forge/tauri`'s `client.widget('term')`).
      Prefer the factory form so reconnects get a fresh session. Default:
      a WebSocket on `url`. */
  transport?: WidgetTransport | (() => WidgetTransport);
  /** Default 'local'. */
  mode?: 'local' | 'ssh';
  host?: string;
  port?: number;
  username?: string;
  password?: string;
  /** Default: true for local mode, false for ssh (needs a target). */
  autoConnect?: boolean;
  /** Default true; falls back to the DOM renderer when WebGL is unavailable. */
  webgl?: boolean;
  fontSize?: number;
  /** Head strip with special-key buttons (Ctrl+C). Default true. */
  toolbar?: boolean;
  onStatus?: (status: TerminalStatus) => void;
  onExit?: (code: number) => void;
  ref?: (api: TerminalApi) => void;
  /** CSS height (default '360px'). */
  height?: string;
  class?: string;
  style?: JSX.CSSProperties;
}

export function Terminal(props: TerminalProps) {
  let host!: HTMLDivElement;
  let term: XTerm | undefined;
  let fit: FitAddon | undefined;
  let ws: WidgetTransport | undefined;
  const [status, setStatus] = createSignal<TerminalStatus>('disconnected');
  const enc = new TextEncoder();

  const report = (s: TerminalStatus) => { setStatus(s); props.onStatus?.(s); };
  const send = (data: Uint8Array) => {
    ws?.send(data);
  };

  const disconnect = () => {
    const sock = ws;
    ws = undefined;
    sock?.close();
    if (sock) report('disconnected');
  };

  const connect = () => {
    if (!term || ws) return;
    report('connecting');
    let sock: WidgetTransport;
    try {
      sock = connectTransport(props.transport, props.url);
    } catch {
      report('error');
      return;
    }
    ws = sock;
    sock.onopen = () => {
      sock.send(JSON.stringify({
        type: 'start',
        mode: props.mode ?? 'local',
        host: props.host,
        port: props.port,
        username: props.username,
        password: props.password,
        cols: term!.cols,
        rows: term!.rows,
      }));
    };
    sock.onmessage = (data) => {
      if (typeof data === 'string') {
        let msg: { type?: string; code?: number; message?: string };
        try { msg = JSON.parse(data); } catch { return; }
        if (msg.type === 'ready') report('ready');
        else if (msg.type === 'exit') { props.onExit?.(msg.code ?? 0); report('closed'); }
        else if (msg.type === 'error') {
          term?.writeln(`\r\n\x1b[31m${msg.message ?? 'error'}\x1b[0m`);
          report('error');
        }
      } else {
        term?.write(new Uint8Array(data));
      }
    };
    sock.onclose = () => {
      if (ws !== sock) return;
      ws = undefined;
      if (status() === 'connecting' || status() === 'ready') report('closed');
    };
    sock.onerror = () => { if (ws === sock) report('error'); };
  };

  onMount(() => {
    const t = new XTerm({
      fontSize: props.fontSize ?? 12,
      fontFamily: getComputedStyle(host).getPropertyValue('--font-mono').trim() || 'monospace',
      theme: readTermTheme(host),
      cursorBlink: true,
      allowProposedApi: true,
      scrollback: 5000,
    });
    term = t;
    fit = new FitAddon();
    t.loadAddon(fit);
    t.open(host);
    if (props.webgl !== false) {
      try {
        const webgl = new WebglAddon();
        webgl.onContextLoss(() => webgl.dispose());
        t.loadAddon(webgl);
      } catch { /* DOM renderer fallback */ }
    }
    fit.fit();

    t.onData((d) => send(enc.encode(d)));
    t.onBinary((d) => {
      const bytes = new Uint8Array(d.length);
      for (let i = 0; i < d.length; i++) bytes[i] = d.charCodeAt(i) & 0xff;
      send(bytes);
    });
    t.onResize(({ cols, rows }) => {
      ws?.send(JSON.stringify({ type: 'resize', cols, rows }));
    });

    const ro = new ResizeObserver(() => fit?.fit());
    ro.observe(host);
    const unwatch = watchTheme(() => { t.options.theme = readTermTheme(host); });

    props.ref?.({
      connect,
      disconnect,
      focus: () => t.focus(),
      fit: () => fit?.fit(),
      write: (d) => t.write(d),
    });

    if (props.autoConnect ?? (props.mode ?? 'local') === 'local') connect();

    onCleanup(() => {
      ro.disconnect();
      unwatch();
      disconnect();
      t.dispose();
      term = undefined;
    });
  });

  return (
    <div class={`fterm ${props.class ?? ''}`}
         style={{ height: props.height ?? '360px', ...props.style }}>
      <Show when={props.toolbar !== false}>
        <div class="fterm-toolbar">
          <button
            type="button"
            class="fterm-key"
            title="Send Ctrl+C (SIGINT) to the remote"
            disabled={status() !== 'ready'}
            onClick={() => { send(new Uint8Array([0x03])); term?.focus(); }}
          >Ctrl+C</button>
        </div>
      </Show>
      <div class="fterm-body" ref={host} />
    </div>
  );
}
