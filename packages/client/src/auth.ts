import type { Core } from './http';
import type { AuthApi, Claims, LoginResult } from './types';

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
