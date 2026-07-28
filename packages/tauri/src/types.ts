/** Widget kinds served by the forge-tauri plugin. */
export type WidgetKind = 'term' | 'vnc' | 'rdp';

/**
 * WebSocket-subset transport consumed by @forge/term and @forge/desktop.
 * String frames carry control JSON, binary frames carry payload bytes — the
 * string-vs-binary discriminator is load-bearing (docs/widgets-protocol.md).
 *
 * The same structural interface is declared in those packages; it is kept
 * deliberately tiny so no cross-package dependency is needed.
 */
export interface WidgetTransport {
  /** Send a frame. Strings are control JSON; views are payload bytes.
      `ArrayBufferView<ArrayBuffer>`, not the bare (SharedArrayBuffer-inclusive)
      default — WebSocket.send only accepts non-shared buffers. */
  send(data: string | ArrayBufferView<ArrayBuffer>): void;
  /** Close the session (idempotent). */
  close(): void;
  onopen: (() => void) | null;
  onmessage: ((data: string | ArrayBuffer) => void) | null;
  onclose: (() => void) | null;
  onerror: ((err: unknown) => void) | null;
}
