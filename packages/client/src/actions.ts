import type { Core } from './http';
import type { ActionsApi } from './types';

export function createActions(core: Core): ActionsApi {
  return {
    call: <T = unknown>(name: string, payload: unknown = {}) =>
      core.request<T>('POST', `/api/actions/${encodeURIComponent(name)}`, payload),
  };
}
