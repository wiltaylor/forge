import { createActions } from './actions';
import { createAuth } from './auth';
import { createData } from './data';
import { createCore } from './http';
import { createEvents } from './sse';
import { buildWsUrl, connectSocket } from './ws';
import type { ClientOptions, ForgeClient, Health } from './types';

export function createClient(opts: ClientOptions = {}): ForgeClient {
  const core = createCore(opts);
  return {
    auth: createAuth(core),
    data: createData(core),
    actions: createActions(core),
    events: createEvents(core),
    ws: {
      connect: () => connectSocket(core),
    },
    wsUrl: (path) => buildWsUrl(core.baseUrl, core.token(), path),
    onUnauthorized: (cb) => core.onUnauthorized(cb),
    health: () => core.request<Health>('GET', '/api/health'),
    request: (method, path, body) => core.request(method, path, body),
  };
}

export { ApiError, TOKEN_KEY } from './http';
export { buildEventsUrl } from './sse';
export { buildWsUrl } from './ws';
export type {
  ActionsApi,
  AuthApi,
  Claims,
  ClientOptions,
  DataApi,
  DocMeta,
  EventsApi,
  ForgeClient,
  ForgeSocket,
  Health,
  LoginResult,
  WsApi,
} from './types';
