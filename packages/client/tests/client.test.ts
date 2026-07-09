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
  it('unwraps {ok:true,data} success envelopes', async () => {
    const { c, calls } = client(() => ({
      body: { ok: true, data: { uptime_s: 5, version: '1.0.0', app: 'demo', auth_enabled: true, actions: ['ping'] } },
    }));
    const health = await c.health();
    expect(health.app).toBe('demo');
    expect(health.actions).toEqual(['ping']);
    expect(calls[0]?.url).toBe('/api/health');
    expect(calls[0]?.method).toBe('GET');
  });

  it('treats success envelopes with omitted data as undefined (mutations)', async () => {
    const { c } = client(() => ({ body: { ok: true } }));
    await expect(c.data.put('doc', { a: 1 })).resolves.toBeUndefined();
  });

  it('throws ApiError with status + message on {ok:false,error}', async () => {
    const { c } = client(() => ({ status: 400, body: { ok: false, error: 'invalid name' } }));
    const err = await c.request('GET', '/api/data/BAD').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(400);
    expect((err as ApiError).message).toBe('invalid name');
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

  it('login stores the token and subsequent requests carry the Authorization header', async () => {
    const { c, calls } = client((call) =>
      call.url === '/api/auth/login'
        ? { body: loginBody }
        : { body: { ok: true, data: { sub: 'admin', roles: ['ops'] } } },
    );

    expect(c.auth.token()).toBeNull();
    expect(c.auth.header()).toEqual({});
    expect(calls[0]?.headers.Authorization).toBeUndefined();

    const result = await c.auth.login('admin', 'admin');
    expect(result.token).toBe('jwt-abc');
    expect(calls[0]?.method).toBe('POST');
    expect(calls[0]?.body).toEqual({ username: 'admin', password: 'admin' });
    // Login itself must not send a stale Authorization header requirement,
    // but afterwards the token is stored and attached everywhere.
    expect(c.auth.token()).toBe('jwt-abc');
    expect(c.auth.header()).toEqual({ Authorization: 'Bearer jwt-abc' });

    const me = await c.auth.me();
    expect(me.sub).toBe('admin');
    expect(calls[1]?.headers.Authorization).toBe('Bearer jwt-abc');
  });

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
  it('get returns the doc payload', async () => {
    const { c, calls } = client(() => ({ body: { ok: true, data: { hello: 'world' } } }));
    await expect(c.data.get('mydoc')).resolves.toEqual({ hello: 'world' });
    expect(calls[0]?.url).toBe('/api/data/mydoc');
  });

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
  it('posts the payload and unwraps the result', async () => {
    const { c, calls } = client(() => ({ body: { ok: true, data: { echoed: 42 } } }));
    const out = await c.actions.call<{ echoed: number }>('echo', { value: 42 });
    expect(out).toEqual({ echoed: 42 });
    expect(calls[0]).toMatchObject({
      method: 'POST',
      url: '/api/actions/echo',
      body: { value: 42 },
    });
  });

  it('defaults to an empty object payload', async () => {
    const { c, calls } = client(() => ({ body: { ok: true, data: null } }));
    await c.actions.call('ping');
    expect(calls[0]?.body).toEqual({});
  });

  it('surfaces unknown-action 404s as ApiError', async () => {
    const { c } = client(() => ({
      status: 404,
      body: { ok: false, error: "unknown action 'nope'; registered: ping, echo" },
    }));
    const err = await c.actions.call('nope').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
    expect((err as ApiError).message).toContain('registered');
  });
});
