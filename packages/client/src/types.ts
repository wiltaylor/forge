/**
 * Public types for @forge/client. Mirrors docs/api-contract.md (v1, frozen).
 * Wire format is snake_case, so contract payload types use snake_case fields.
 */

export interface ClientOptions {
  /** Server origin, e.g. 'http://localhost:8765'. Default '' (same-origin). */
  baseUrl?: string;
  /** Where the JWT is persisted (storage key 'forge.token'). Default 'local'. */
  tokenStorage?: 'local' | 'session' | 'memory';
  /** Injectable fetch implementation (tests / non-browser hosts). */
  fetch?: typeof fetch;
}

/** `/api/auth/login` response payload. */
export interface LoginResult {
  token: string;
  expires_at: number;
  user: { name: string; roles: string[] };
}

/** `/api/auth/me` response payload — decoded JWT claims. */
export interface Claims {
  sub: string;
  roles: string[];
  iss?: string;
  exp?: number;
  iat?: number;
}

/** `/api/health` response payload. */
export interface Health {
  uptime_s: number;
  version: string;
  app: string;
  auth_enabled: boolean;
  actions: string[];
}

/** One entry of the `/api/data` listing. `modified` = unix seconds (float). */
export interface DocMeta {
  name: string;
  bytes: number;
  modified: number;
}

export interface AuthApi {
  /** POST /api/auth/login; stores the returned token on success. */
  login(username: string, password: string): Promise<LoginResult>;
  /** Clears the stored token. Purely client-side. */
  logout(): void;
  /** GET /api/auth/me. */
  me(): Promise<Claims>;
  /** Currently stored token, if any. */
  token(): string | null;
  /** Set/clear the token directly (external-issuer mode). */
  setToken(token: string | null): void;
  /** `{ Authorization: 'Bearer ...' }` when a token is stored, else `{}`. */
  header(): Record<string, string>;
}

export interface DataApi {
  list(): Promise<DocMeta[]>;
  /** Resolves null when the doc does not exist (404). */
  get<T = unknown>(name: string): Promise<T | null>;
  put(name: string, doc: unknown): Promise<void>;
  del(name: string): Promise<void>;
  /**
   * Debounced put: coalesces rapid writes per doc name; only the last doc
   * within the window is written. Default window 500 ms. Write errors are
   * swallowed (401s still fire onUnauthorized via the shared request path).
   */
  putDebounced(name: string, doc: unknown, ms?: number): void;
}

export interface ActionsApi {
  call<T = unknown>(name: string, payload?: unknown): Promise<T>;
}

export interface EventsApi {
  /**
   * Subscribe to a server-sent-events topic. Lazily opens one shared
   * EventSource (reopened when the topic set grows); returns an unsubscribe
   * function. The EventSource closes when the last listener leaves.
   */
  on(topic: string, cb: (data: unknown) => void): () => void;
}

export interface ForgeSocket {
  /** Send a JSON frame. Queued until the socket is open. */
  send(frame: Record<string, unknown>): void;
  /** Sends {type:'subscribe', topics} (accumulated set); resent on reconnect. */
  subscribe(topics: string[]): void;
  /** Listen for frames by `type`. Returns an unsubscribe function.
      Synthetic local types (not wire frames): 'open', 'close'. */
  on(type: string, cb: (frame: any) => void): () => void;
  /** Listen for 'event' frames matching `topic`; cb receives frame.data. */
  onEvent(topic: string, cb: (data: unknown) => void): () => void;
  /** Close the socket and stop reconnecting. */
  close(): void;
}

export interface WsApi {
  connect(): ForgeSocket;
}

export interface ForgeClient {
  auth: AuthApi;
  data: DataApi;
  actions: ActionsApi;
  events: EventsApi;
  ws: WsApi;
  /**
   * Absolute ws(s):// URL for a server WebSocket endpoint (e.g. '/api/term'),
   * carrying the current token as `?token=`. For widgets that manage their
   * own socket rather than the shared event socket.
   */
  wsUrl(path: string): string;
  /** Any 401 clears the stored token, then fires. Returns unsubscribe. */
  onUnauthorized(cb: () => void): () => void;
  /** GET /api/health. */
  health(): Promise<Health>;
  /** Escape hatch: raw request against the contract, envelope-unwrapped. */
  request<T = unknown>(method: string, path: string, body?: unknown): Promise<T>;
}
