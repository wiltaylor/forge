/**
 * The `plugin:forge|request` bridge: the Forge contract carried over Tauri
 * IPC. Response unwrapping mirrors @forge/client's fetch core exactly, so
 * both clients throw the same ApiError shapes.
 */
import { ApiError } from '@forge/client';
import { invoke } from '@tauri-apps/api/core';

interface Envelope {
  ok: boolean;
  data?: unknown;
  error?: string;
}

interface ForgeResponse {
  status: number;
  body: unknown;
}

/** Shared internals threaded through the sub-APIs. Not exported from the package. */
export interface Core {
  token(): string | null;
  setToken(token: string | null): void;
  authHeader(): Record<string, string>;
  onUnauthorized(cb: () => void): () => void;
  request<T = unknown>(method: string, path: string, body?: unknown): Promise<T>;
}

export function createCore(): Core {
  // IPC mode is auth-disabled by design; the token store exists only for
  // AuthApi interface parity and never leaves memory.
  let token: string | null = null;
  const unauthorizedCbs = new Set<() => void>();

  function fireUnauthorized(): void {
    token = null;
    for (const cb of [...unauthorizedCbs]) {
      try {
        cb();
      } catch {
        // listener errors must not break the request path
      }
    }
  }

  async function request<T = unknown>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await invoke<ForgeResponse>('plugin:forge|request', {
      method,
      path,
      body: body ?? null,
    });

    const json = res.body;
    const envelope =
      json !== null && typeof json === 'object' && 'ok' in json ? (json as Envelope) : undefined;

    if (res.status === 401) {
      fireUnauthorized();
      throw new ApiError(401, envelope?.error ?? 'Unauthorized');
    }
    if (envelope) {
      if (envelope.ok) return envelope.data as T;
      throw new ApiError(res.status, envelope.error ?? `HTTP ${res.status}`);
    }
    if (res.status >= 400) {
      throw new ApiError(res.status, `HTTP ${res.status}`);
    }
    return json as T;
  }

  return {
    token: () => token,
    setToken: (t) => {
      token = t;
    },
    authHeader(): Record<string, string> {
      return token ? { Authorization: `Bearer ${token}` } : {};
    },
    onUnauthorized(cb) {
      unauthorizedCbs.add(cb);
      return () => {
        unauthorizedCbs.delete(cb);
      };
    },
    request,
  };
}
