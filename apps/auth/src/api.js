// Cookie-session API wrapper. Deliberately NOT @forge/client: the IdP's own
// UI authenticates with the HttpOnly session cookie (no tokens in
// localStorage) and sends the X-Forge-Auth CSRF header on mutations.

export class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

const unauthorizedHandlers = new Set();

export function onUnauthorized(cb) {
  unauthorizedHandlers.add(cb);
  return () => unauthorizedHandlers.delete(cb);
}

async function request(method, path, body) {
  const headers = { 'X-Forge-Auth': '1' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  const res = await fetch(path, {
    method,
    credentials: 'same-origin',
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  let payload = null;
  try {
    payload = await res.json();
  } catch {
    /* non-JSON error body */
  }
  if (res.status === 401) {
    unauthorizedHandlers.forEach((cb) => cb());
  }
  if (!res.ok || payload?.ok === false) {
    throw new ApiError(res.status, payload?.error ?? `HTTP ${res.status}`);
  }
  return payload?.data ?? payload;
}

export const api = {
  get: (path) => request('GET', path),
  post: (path, body) => request('POST', path, body ?? {}),
  put: (path, body) => request('PUT', path, body ?? {}),
  del: (path) => request('DELETE', path),
};
