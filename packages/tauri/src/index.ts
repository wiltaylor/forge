import type { ForgeClient, Health } from '@forge/client';

import { createActions } from './actions';
import { createAuth } from './auth';
import { createData } from './data';
import { createEvents } from './events';
import { createCore } from './ipc';
import { createWidgetTransport } from './widgets';
import type { WidgetKind, WidgetTransport } from './types';

/** The @forge/client interface plus the widget transport factory. */
export interface TauriForgeClient extends ForgeClient {
  /**
   * Transport for the `transport` prop of @forge/term's `<Terminal>` and
   * @forge/desktop's `<DesktopViewer>` — a widget session over IPC.
   */
  widget(kind: WidgetKind): WidgetTransport;
}

/**
 * Forge client over Tauri IPC. Implements `ForgeClient` from @forge/client
 * (importing its types keeps the two conforming to the same contract) —
 * swap `createClient` imports and the rest of the app code stays unchanged.
 *
 * Not available over IPC: `ws.connect()` (use `events.on()`) and `wsUrl()`
 * (pass `client.widget(kind)` as the widget transport instead).
 */
export function createClient(): TauriForgeClient {
  const core = createCore();
  return {
    auth: createAuth(core),
    data: createData(core),
    actions: createActions(core),
    events: createEvents(),
    ws: {
      connect: () => {
        throw new Error(
          'ws.connect() is not available over Tauri IPC; use events.on() for server events',
        );
      },
    },
    wsUrl: () => {
      throw new Error(
        'wsUrl() is not available over Tauri IPC; pass client.widget(kind) as the widget transport instead',
      );
    },
    onUnauthorized: (cb) => core.onUnauthorized(cb),
    health: () => core.request<Health>('GET', '/api/health'),
    request: (method, path, body) => core.request(method, path, body),
    widget: (kind) => createWidgetTransport(kind),
  };
}

export { ApiError } from '@forge/client';
export type {
  ActionsApi,
  AuthApi,
  Claims,
  DataApi,
  DocMeta,
  EventsApi,
  ForgeClient,
  Health,
  LoginResult,
} from '@forge/client';
export type { WidgetKind, WidgetTransport } from './types';
