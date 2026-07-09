/* ---------------- DesktopViewer ----------------------------------------------
   Shared VNC/RDP canvas: the backend decodes the desktop protocol and streams
   raw RGBA rects; this widget only blits them and forwards input. Binary rect
   frame: u8 version=1, u8 encoding=0(raw), u16 x, u16 y, u16 w, u16 h (LE,
   10 bytes) + w*h*4 RGBA. JSON text frames for control. No auto-reconnect. */
import { createSignal, onMount, onCleanup, Show } from 'solid-js';
import type { JSX } from 'solid-js';
import { connectTransport } from './transport';
import type { WidgetTransport } from './transport';

export type DesktopStatus = 'disconnected' | 'connecting' | 'ready' | 'closed' | 'error';

export interface DesktopApi {
  connect(): void;
  disconnect(): void;
  sendCtrlAltDel(): void;
  focus(): void;
}

export interface DesktopViewerProps {
  /** WebSocket URL, e.g. `api.wsUrl('/api/desktop/vnc')` (token embedded).
      Optional when `transport` is provided. */
  url?: string;
  /** Custom connection (e.g. `@forge/tauri`'s `client.widget('vnc')`).
      Prefer the factory form so reconnects get a fresh session. Default:
      a WebSocket on `url`. */
  transport?: WidgetTransport | (() => WidgetTransport);
  host?: string;
  port?: number;
  username?: string;
  password?: string;
  /** Default false — a viewer always needs an explicit target. */
  autoConnect?: boolean;
  /** 'fit' (default) scales to the frame; 'native' is 1:1 with scrollbars. */
  scale?: 'fit' | 'native';
  /** Render only; forward no keyboard/mouse input. */
  viewOnly?: boolean;
  /** Head strip with special-key buttons (Ctrl+Alt+Del). Default true. */
  toolbar?: boolean;
  onStatus?: (status: DesktopStatus) => void;
  ref?: (api: DesktopApi) => void;
  /** CSS height (default '480px'). */
  height?: string;
  class?: string;
  style?: JSX.CSSProperties;
}

export function DesktopViewer(props: DesktopViewerProps) {
  let canvas!: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let ws: WidgetTransport | undefined;
  const [status, setStatus] = createSignal<DesktopStatus>('disconnected');
  const [errorMsg, setErrorMsg] = createSignal('');

  const report = (s: DesktopStatus) => { setStatus(s); props.onStatus?.(s); };
  const sendJson = (msg: unknown) => {
    ws?.send(JSON.stringify(msg));
  };

  const disconnect = () => {
    const sock = ws;
    ws = undefined;
    sock?.close();
    if (sock) report('disconnected');
  };

  const handleRect = (buf: ArrayBuffer) => {
    if (buf.byteLength < 10) return;
    const dv = new DataView(buf);
    if (dv.getUint8(0) !== 1) return; /* unknown frame version */
    const x = dv.getUint16(2, true);
    const y = dv.getUint16(4, true);
    const w = dv.getUint16(6, true);
    const h = dv.getUint16(8, true);
    if (!w || !h || buf.byteLength !== 10 + w * h * 4) return;
    ctx?.putImageData(new ImageData(new Uint8ClampedArray(buf, 10), w, h), x, y);
  };

  const connect = () => {
    if (ws) return;
    setErrorMsg('');
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
        type: 'connect',
        host: props.host,
        port: props.port,
        username: props.username,
        password: props.password,
      }));
    };
    sock.onmessage = (data) => {
      if (typeof data === 'string') {
        let msg: { type?: string; width?: number; height?: number; message?: string };
        try { msg = JSON.parse(data); } catch { return; }
        if (msg.type === 'ready' || msg.type === 'resize') {
          if (msg.width && msg.height) {
            canvas.width = msg.width;
            canvas.height = msg.height;
          }
          if (msg.type === 'ready') report('ready');
        } else if (msg.type === 'error') {
          setErrorMsg(msg.message ?? 'connection error');
          report('error');
        } else if (msg.type === 'closed') {
          report('closed');
        }
      } else {
        handleRect(data);
      }
    };
    sock.onclose = () => {
      if (ws !== sock) return;
      ws = undefined;
      if (status() === 'connecting' || status() === 'ready') report('closed');
    };
    sock.onerror = () => { if (ws === sock) report('error'); };
  };

  /* Pointer → framebuffer coords (canvas may be CSS-scaled in 'fit' mode). */
  const toFb = (e: PointerEvent) => {
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height || !canvas.width || !canvas.height) return null;
    const x = Math.round((e.clientX - rect.left) * (canvas.width / rect.width));
    const y = Math.round((e.clientY - rect.top) * (canvas.height / rect.height));
    return {
      x: Math.max(0, Math.min(canvas.width - 1, x)),
      y: Math.max(0, Math.min(canvas.height - 1, y)),
    };
  };

  /* Mouse moves are throttled to one message per animation frame. */
  let pendingMove: { x: number; y: number; buttons: number } | null = null;
  let rafId = 0;
  const flushMove = () => {
    rafId = 0;
    if (pendingMove) { sendJson({ type: 'mouse', ...pendingMove }); pendingMove = null; }
  };

  const inputOn = () => !props.viewOnly && status() === 'ready';

  const onPointer = (e: PointerEvent, down?: boolean) => {
    if (!inputOn()) return;
    const pos = toFb(e);
    if (!pos) return;
    e.preventDefault();
    if (down !== undefined) { /* press/release: send immediately, in order */
      if (down) { canvas.focus(); canvas.setPointerCapture(e.pointerId); }
      if (rafId) { cancelAnimationFrame(rafId); flushMove(); }
      sendJson({ type: 'mouse', ...pos, buttons: e.buttons });
    } else {
      pendingMove = { ...pos, buttons: e.buttons };
      if (!rafId) rafId = requestAnimationFrame(flushMove);
    }
  };

  const onKey = (e: KeyboardEvent, down: boolean) => {
    if (!inputOn()) return;
    e.preventDefault();
    /* `code` drives layout-independent mapping; `key` carries the printable
       character for the backend's Unicode-keysym path. */
    sendJson({ type: 'key', code: e.code, key: e.key, down });
  };

  const sendCtrlAltDel = () => sendJson({ type: 'cad' });

  onMount(() => {
    ctx = canvas.getContext('2d');
    props.ref?.({
      connect,
      disconnect,
      sendCtrlAltDel,
      focus: () => canvas.focus(),
    });
    if (props.autoConnect) connect();
    onCleanup(() => {
      if (rafId) cancelAnimationFrame(rafId);
      disconnect();
    });
  });

  return (
    <div class={`fdesk ${props.class ?? ''}`}
         style={{ height: props.height ?? '480px', ...props.style }}>
      <Show when={props.toolbar !== false}>
        <div class="fdesk-toolbar">
          <button
            type="button"
            class="fdesk-key"
            title="Send Ctrl+Alt+Del to the remote"
            disabled={status() !== 'ready' || props.viewOnly}
            onClick={() => { sendCtrlAltDel(); canvas.focus(); }}
          >Ctrl+Alt+Del</button>
        </div>
      </Show>
      <div class="fdesk-stage">
        <canvas
          ref={canvas}
          class="fdesk-canvas"
          data-scale={props.scale ?? 'fit'}
          tabindex="0"
          width="0"
          height="0"
          onPointerDown={(e) => onPointer(e, true)}
          onPointerUp={(e) => onPointer(e, false)}
          onPointerMove={(e) => onPointer(e)}
          onWheel={(e) => {
            if (!inputOn()) return;
            e.preventDefault();
            sendJson({ type: 'wheel', dx: e.deltaX, dy: e.deltaY });
          }}
          onKeyDown={(e) => onKey(e, true)}
          onKeyUp={(e) => onKey(e, false)}
          onContextMenu={(e) => e.preventDefault()}
        />
        <Show when={status() !== 'ready'}>
          <div class="fdesk-status">{errorMsg() || status()}</div>
        </Show>
      </div>
    </div>
  );
}
