import { Channel, invoke } from '@tauri-apps/api/core';

import type { WidgetKind, WidgetTransport } from './types';

/**
 * Frames arriving on the session channel: control JSON rides as a string,
 * payload bytes ride raw (ArrayBuffer / typed array depending on the IPC
 * path), and JSON `null` signals close.
 */
type ChannelFrame = string | ArrayBuffer | Uint8Array | number[] | null;

function toArrayBuffer(view: ArrayBufferView): ArrayBuffer {
  return view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength) as ArrayBuffer;
}

function toByteArray(view: ArrayBufferView): number[] {
  const bytes =
    view instanceof Uint8Array
      ? view
      : new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
  return Array.from(bytes);
}

/**
 * Open a widget session over Tauri IPC and present it as the WebSocket-subset
 * [`WidgetTransport`] that @forge/term / @forge/desktop accept.
 *
 * Sends are chained on one promise queue: invoke() calls resolve
 * independently, so without the chain two rapid sends could arrive reordered.
 */
export function createWidgetTransport(kind: WidgetKind): WidgetTransport {
  let id: number | null = null;
  let closed = false;
  let closeFired = false;

  const transport: WidgetTransport = {
    onopen: null,
    onmessage: null,
    onclose: null,
    onerror: null,

    send(data) {
      if (closed) return;
      // Snapshot binary payloads now; callers may reuse the buffer.
      const text = typeof data === 'string' ? data : null;
      const bytes = typeof data === 'string' ? null : toByteArray(data);
      queue = queue
        .then(() => {
          if (closed || id === null) return;
          return text !== null
            ? invoke('plugin:forge|widget_send_text', { id, text })
            : invoke('plugin:forge|widget_send_binary', { id, data: bytes });
        })
        .catch((err) => {
          transport.onerror?.(err);
        });
    },

    close() {
      if (closed) return;
      closed = true;
      queue = queue
        .then(() => (id === null ? undefined : invoke('plugin:forge|widget_close', { id })))
        .catch(() => {})
        .then(() => {
          fireClose();
        });
    },
  };

  function fireClose(): void {
    if (closeFired) return;
    closeFired = true;
    transport.onclose?.();
  }

  const channel = new Channel<ChannelFrame>();
  channel.onmessage = (message) => {
    if (closed) return;
    if (message === null) {
      // Engine-initiated close frame.
      closed = true;
      fireClose();
      return;
    }
    const cb = transport.onmessage;
    if (!cb) return;
    if (typeof message === 'string') cb(message);
    else if (message instanceof ArrayBuffer) cb(message);
    else if (ArrayBuffer.isView(message)) cb(toArrayBuffer(message));
    else if (Array.isArray(message)) cb(Uint8Array.from(message).buffer);
  };

  let queue: Promise<unknown> = invoke<number>('plugin:forge|widget_open', {
    kind,
    onMessage: channel,
  }).then(
    (sid) => {
      id = sid;
      transport.onopen?.();
    },
    (err) => {
      closed = true;
      transport.onerror?.(err);
      fireClose();
    },
  );

  return transport;
}
