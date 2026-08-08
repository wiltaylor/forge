import type { AuthApi, Claims, LoginResult } from '@forge/client';

import type { Core } from './ipc';

/**
 * AuthApi over IPC — the same contract as over HTTP. `login()` mints a token
 * the core carries on every later request.
 *
 * A plugin built without auth runs auth-disabled: `me()` then resolves to the
 * anonymous claims and `login()` rejects with the contract's 404 "auth is
 * disabled", which is what an app whose only caller is its own webview wants.
 */
export function createAuth(core: Core): AuthApi {
  return {
    async login(username, password) {
      const result = await core.request<LoginResult>('POST', '/api/auth/login', {
        username,
        password,
      });
      core.setToken(result.token);
      return result;
    },
    logout() {
      core.setToken(null);
    },
    me: () => core.request<Claims>('GET', '/api/auth/me'),
    token: () => core.token(),
    setToken: (token) => core.setToken(token),
    header: () => core.authHeader(),
  };
}
