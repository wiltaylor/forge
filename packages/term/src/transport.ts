/* Widget transport seam. The terminal speaks a WebSocket-subset interface so
   the same component runs over a real WebSocket (default) or any custom
   carrier (e.g. @forge/tauri's IPC transport). String frames carry control
   JSON, binary frames carry tty bytes — the discriminator is load-bearing
   (docs/widgets-protocol.md).

   The same structural interface is declared in @forge/desktop and
   @forge/tauri; it is kept deliberately tiny so no cross-package dependency
   is needed. */

export interface WidgetTransport {
  /** Send a frame. Strings are control JSON; views are payload bytes. */
  send(data: string | ArrayBufferView): void;
  /** Close the session (idempotent). */
  close(): void;
  onopen: (() => void) | null;
  onmessage: ((data: string | ArrayBuffer) => void) | null;
  onclose: (() => void) | null;
  onerror: ((err: unknown) => void) | null;
}

/** Wrap a raw WebSocket in the transport shape (sends drop until open). */
function wsTransport(url: string): WidgetTransport {
  const sock = new WebSocket(url);
  sock.binaryType = 'arraybuffer';
  const transport: WidgetTransport = {
    send(data) {
      if (sock.readyState === WebSocket.OPEN) sock.send(data);
    },
    close: () => sock.close(),
    onopen: null,
    onmessage: null,
    onclose: null,
    onerror: null,
  };
  sock.onopen = () => transport.onopen?.();
  sock.onmessage = (ev) => transport.onmessage?.(ev.data as string | ArrayBuffer);
  sock.onclose = () => transport.onclose?.();
  sock.onerror = (err) => transport.onerror?.(err);
  return transport;
}

/**
 * Resolve the `transport`/`url` props into a live transport: an explicit
 * transport (instance or factory — prefer a factory so reconnects get a
 * fresh session) wins; otherwise a WebSocket on `url`.
 */
export function connectTransport(
  transport: WidgetTransport | (() => WidgetTransport) | undefined,
  url: string | undefined,
): WidgetTransport {
  if (transport) return typeof transport === 'function' ? transport() : transport;
  if (!url) throw new Error('either `transport` or `url` is required');
  return wsTransport(url);
}
