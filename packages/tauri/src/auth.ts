import type { AuthApi, Claims, LoginResult } from '@forge/client';

import type { Core } from './ipc';

/**
 * AuthApi over IPC. The plugin runs auth-disabled (the caller is the app's
 * own webview): `me()` resolves to the anonymous claims and `login()` always
 * rejects with the contract's 404 "auth is disabled".
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
