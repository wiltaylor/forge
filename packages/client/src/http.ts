import type { ClientOptions } from './types';

/** Storage key used for the JWT in local/session storage. */
export const TOKEN_KEY = 'forge.token';

/** Error thrown for any non-ok contract response. */
export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

interface Envelope {
  ok: boolean;
  data?: unknown;
  error?: string;
}

interface TokenStore {
  get(): string | null;
  set(token: string | null): void;
}

function webStorage(kind: 'local' | 'session'): Storage | null {
  try {
    const s =
      kind === 'local'
        ? (globalThis as { localStorage?: Storage }).localStorage
        : (globalThis as { sessionStorage?: Storage }).sessionStorage;
    return s ?? null;
  } catch {
    return null;
  }
}

function createTokenStore(kind: 'local' | 'session' | 'memory'): TokenStore {
  if (kind !== 'memory') {
    const s = webStorage(kind);
    if (s) {
      return {
        get: () => s.getItem(TOKEN_KEY),
        set: (token) => {
          if (token === null) s.removeItem(TOKEN_KEY);
          else s.setItem(TOKEN_KEY, token);
        },
      };
    }
    // No Web Storage in this environment (e.g. Node) — degrade to memory.
  }
  let token: string | null = null;
  return {
    get: () => token,
    set: (t) => {
      token = t;
    },
  };
}

/** Shared internals threaded through the sub-APIs. Not exported from the package. */
export interface Core {
  baseUrl: string;
  token(): string | null;
  setToken(token: string | null): void;
  authHeader(): Record<string, string>;
  onUnauthorized(cb: () => void): () => void;
  request<T = unknown>(method: string, path: string, body?: unknown): Promise<T>;
}

export function createCore(opts: ClientOptions = {}): Core {
  const baseUrl = (opts.baseUrl ?? '').replace(/\/+$/, '');
  const store = createTokenStore(opts.tokenStorage ?? 'local');
  const fetchImpl: typeof fetch = opts.fetch ?? ((input, init) => globalThis.fetch(input, init));
  const unauthorizedCbs = new Set<() => void>();

  function fireUnauthorized(): void {
    store.set(null);
    for (const cb of [...unauthorizedCbs]) {
      try {
        cb();
      } catch {
        // listener errors must not break the request path
      }
    }
  }

  function authHeader(): Record<string, string> {
    const token = store.get();
    return token ? { Authorization: `Bearer ${token}` } : {};
  }

  async function request<T = unknown>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = { ...authHeader() };
    const init: RequestInit = { method, headers };
    if (body !== undefined) {
      headers['Content-Type'] = 'application/json';
      init.body = JSON.stringify(body);
    }

    const res = await fetchImpl(baseUrl + path, init);

    let json: unknown;
    try {
      json = await res.json();
    } catch {
      json = undefined;
    }
    const envelope =
      json !== null && typeof json === 'object' && 'ok' in json ? (json as Envelope) : undefined;

    if (res.status === 401) {
      fireUnauthorized();
      throw new ApiError(401, envelope?.error ?? (res.statusText || 'Unauthorized'));
    }
    if (envelope) {
      if (envelope.ok) return envelope.data as T;
      throw new ApiError(res.status, envelope.error ?? (res.statusText || `HTTP ${res.status}`));
    }
    if (!res.ok) {
      throw new ApiError(res.status, res.statusText || `HTTP ${res.status}`);
    }
    return json as T;
  }

  return {
    baseUrl,
    token: () => store.get(),
    setToken: (token) => store.set(token),
    authHeader,
    onUnauthorized(cb) {
      unauthorizedCbs.add(cb);
      return () => {
        unauthorizedCbs.delete(cb);
      };
    },
    request,
  };
}
