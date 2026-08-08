/* Client-specific behaviour a corpus case cannot state: URL construction,
   the token lifecycle, debouncing, and what the client resolves or throws
   around the wire. Contract behaviour — the envelope, the statuses, the
   payload shapes — is covered against a real backend by `corpus.test.ts`.
   The mocked envelopes below are scaffolding for those client-side
   assertions, never the thing being asserted. */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, createClient, type ClientOptions } from '../src/index';

interface RecordedCall {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: unknown;
}

type Responder = (call: RecordedCall) => { status?: number; body?: unknown };

/** fetch mock that records calls and replies with JSON envelopes. */
function makeFetch(responder: Responder) {
  const calls: RecordedCall[] = [];
  const impl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const call: RecordedCall = {
      url: String(input),
      method: init?.method ?? 'GET',
      headers: (init?.headers as Record<string, string>) ?? {},
      body: typeof init?.body === 'string' ? JSON.parse(init.body) : undefined,
    };
    calls.push(call);
    const { status = 200, body = { ok: true } } = responder(call);
    return new Response(JSON.stringify(body), {
      status,
      headers: { 'Content-Type': 'application/json' },
    });
  });
  return { impl: impl as unknown as typeof fetch, calls };
}

function client(responder: Responder, opts: Partial<ClientOptions> = {}) {
  const { impl, calls } = makeFetch(responder);
  const c = createClient({ tokenStorage: 'memory', fetch: impl, ...opts });
  return { c, calls };
}

afterEach(() => {
  vi.useRealTimers();
});

describe('envelope handling', () => {
  it('treats success envelopes with omitted data as undefined (mutations)', async () => {
    const { c } = client(() => ({ body: { ok: true } }));
    await expect(c.data.put('doc', { a: 1 })).resolves.toBeUndefined();
  });

  it('falls back to statusText when the error body is not an envelope', async () => {
    const fetchImpl = (async () =>
      new Response('not json', { status: 500, statusText: 'Internal Server Error' })) as unknown as typeof fetch;
    const c = createClient({ tokenStorage: 'memory', fetch: fetchImpl });
    const err = await c.health().catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(500);
    expect((err as ApiError).message).toBe('Internal Server Error');
  });

  it('prefixes requests with baseUrl', async () => {
    const { c, calls } = client(() => ({ body: { ok: true, data: [] } }), {
      baseUrl: 'http://localhost:8765/',
    });
    await c.data.list();
    expect(calls[0]?.url).toBe('http://localhost:8765/api/data');
  });
});

describe('auth', () => {
  const loginBody = {
    ok: true,
    data: { token: 'jwt-abc', expires_at: 1234567890, user: { name: 'admin', roles: ['ops'] } },
  };

  it('logout clears the token', async () => {
    const { c } = client(() => ({ body: loginBody }));
    await c.auth.login('admin', 'admin');
    c.auth.logout();
    expect(c.auth.token()).toBeNull();
    expect(c.auth.header()).toEqual({});
  });

  it('setToken supports external-issuer mode', () => {
    const { c } = client(() => ({ body: { ok: true } }));
    c.auth.setToken('external-jwt');
    expect(c.auth.token()).toBe('external-jwt');
    expect(c.auth.header()).toEqual({ Authorization: 'Bearer external-jwt' });
    c.auth.setToken(null);
    expect(c.auth.token()).toBeNull();
  });

  it('a 401 clears the token, fires onUnauthorized, then throws', async () => {
    const { c } = client(() => ({ status: 401, body: { ok: false, error: 'token expired' } }));
    c.auth.setToken('stale-jwt');
    const cb = vi.fn();
    const off = c.onUnauthorized(cb);

    const err = await c.auth.me().catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(401);
    expect((err as ApiError).message).toBe('token expired');
    expect(c.auth.token()).toBeNull();
    expect(cb).toHaveBeenCalledTimes(1);

    // unsubscribed listeners no longer fire
    off();
    c.auth.setToken('stale-again');
    await c.auth.me().catch(() => {});
    expect(cb).toHaveBeenCalledTimes(1);
  });
});

describe('data', () => {
  it('get returns null on 404', async () => {
    const { c } = client(() => ({ status: 404, body: { ok: false, error: 'not found' } }));
    await expect(c.data.get('missing')).resolves.toBeNull();
  });

  it('get rethrows non-404 errors', async () => {
    const { c } = client(() => ({ status: 400, body: { ok: false, error: 'invalid name' } }));
    await expect(c.data.get('BAD NAME')).rejects.toBeInstanceOf(ApiError);
  });

  it('put and del hit the right endpoints', async () => {
    const { c, calls } = client(() => ({ body: { ok: true } }));
    await c.data.put('doc-a', { v: 1 });
    await c.data.del('doc-a');
    expect(calls[0]).toMatchObject({ method: 'PUT', url: '/api/data/doc-a', body: { v: 1 } });
    expect(calls[1]).toMatchObject({ method: 'DELETE', url: '/api/data/doc-a' });
  });

  it('putDebounced coalesces rapid writes, keeping only the last doc', async () => {
    vi.useFakeTimers();
    const { c, calls } = client(() => ({ body: { ok: true } }));

    c.data.putDebounced('doc', { v: 1 });
    await vi.advanceTimersByTimeAsync(200);
    c.data.putDebounced('doc', { v: 2 });
    await vi.advanceTimersByTimeAsync(200);
    c.data.putDebounced('doc', { v: 3 });
    expect(calls).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(500);
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ method: 'PUT', url: '/api/data/doc', body: { v: 3 } });
  });

  it('putDebounced keeps independent timers per doc name and honours a custom window', async () => {
    vi.useFakeTimers();
    const { c, calls } = client(() => ({ body: { ok: true } }));

    c.data.putDebounced('a', { v: 'a' }, 100);
    c.data.putDebounced('b', { v: 'b' }, 300);

    await vi.advanceTimersByTimeAsync(100);
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ url: '/api/data/a', body: { v: 'a' } });

    await vi.advanceTimersByTimeAsync(200);
    expect(calls).toHaveLength(2);
    expect(calls[1]).toMatchObject({ url: '/api/data/b', body: { v: 'b' } });
  });
});

describe('actions', () => {
  it('defaults to an empty object payload', async () => {
    const { c, calls } = client(() => ({ body: { ok: true, data: null } }));
    await c.actions.call('ping');
    expect(calls[0]?.body).toEqual({});
  });
});
