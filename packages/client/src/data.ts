import { ApiError, type Core } from './http';
import type { DataApi, DocMeta } from './types';

const DEFAULT_DEBOUNCE_MS = 500;

function docPath(name: string): string {
  return `/api/data/${encodeURIComponent(name)}`;
}

export function createData(core: Core): DataApi {
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  const api: DataApi = {
    list: () => core.request<DocMeta[]>('GET', '/api/data'),

    async get<T = unknown>(name: string): Promise<T | null> {
      try {
        return await core.request<T>('GET', docPath(name));
      } catch (err) {
        if (err instanceof ApiError && err.status === 404) return null;
        throw err;
      }
    },

    async put(name, doc) {
      await core.request('PUT', docPath(name), doc);
    },

    async del(name) {
      await core.request('DELETE', docPath(name));
    },

    putDebounced(name, doc, ms = DEFAULT_DEBOUNCE_MS) {
      const prev = timers.get(name);
      if (prev !== undefined) clearTimeout(prev);
      timers.set(
        name,
        setTimeout(() => {
          timers.delete(name);
          // Fire-and-forget by design; 401s still clear the token and fire
          // onUnauthorized inside the shared request path.
          void api.put(name, doc).catch(() => {});
        }, ms),
      );
    },
  };

  return api;
}
