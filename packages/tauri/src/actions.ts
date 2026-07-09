import type { ActionsApi } from '@forge/client';

import type { Core } from './ipc';

export function createActions(core: Core): ActionsApi {
  return {
    call: <T = unknown>(name: string, payload: unknown = {}) =>
      core.request<T>('POST', `/api/actions/${encodeURIComponent(name)}`, payload),
  };
}
