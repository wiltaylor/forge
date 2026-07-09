import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError } from '@forge/client';

import { createClient } from '../src';

vi.mock('@tauri-apps/api/core', () => {
  class Channel<T> {
    onmessage: ((msg: T) => void) | null = null;
  }
  return { invoke: vi.fn(), Channel };
});
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

import { invoke } from '@tauri-apps/api/core';

const invokeMock = vi.mocked(invoke);

function respond(status: number, body: unknown): void {
  invokeMock.mockResolvedValueOnce({ status, body });
}

beforeEach(() => {
  invokeMock.mockReset();
});

describe('request unwrapping', () => {
  it('unwraps ok envelopes to data', async () => {
    const client = createClient();
    respond(200, { ok: true, data: { app: 'demo', uptime_s: 1 } });
    const health = await client.health();
    expect(health).toEqual({ app: 'demo', uptime_s: 1 });
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|request', {
      method: 'GET',
      path: '/api/health',
      body: null,
    });
  });

  it('throws ApiError with the envelope status and message', async () => {
    const client = createClient();
    respond(404, { ok: false, error: 'auth is disabled' });
    const err = await client
      .request('POST', '/api/auth/login', { username: 'a', password: 'b' })
      .catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
    expect((err as ApiError).message).toBe('auth is disabled');
  });

  it('fires onUnauthorized on 401', async () => {
    const client = createClient();
    const cb = vi.fn();
    client.onUnauthorized(cb);
    respond(401, { ok: false, error: 'nope' });
    await expect(client.request('GET', '/api/auth/me')).rejects.toMatchObject({ status: 401 });
    expect(cb).toHaveBeenCalledOnce();
  });
});

describe('data', () => {
  it('resolves null for a missing doc (404)', async () => {
    const client = createClient();
    respond(404, { ok: false, error: 'no document "nope"' });
    expect(await client.data.get('nope')).toBeNull();
  });

  it('percent-encodes doc names', async () => {
    const client = createClient();
    respond(200, { ok: true, data: { a: 1 } });
    await client.data.get('notes');
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|request', {
      method: 'GET',
      path: '/api/data/notes',
      body: null,
    });
  });

  it('putDebounced coalesces rapid writes per name', async () => {
    vi.useFakeTimers();
    try {
      const client = createClient();
      invokeMock.mockResolvedValue({ status: 200, body: { ok: true } });
      client.data.putDebounced('doc', { v: 1 });
      client.data.putDebounced('doc', { v: 2 });
      client.data.putDebounced('other', { v: 3 });
      await vi.advanceTimersByTimeAsync(600);
      expect(invokeMock).toHaveBeenCalledTimes(2);
      expect(invokeMock).toHaveBeenCalledWith('plugin:forge|request', {
        method: 'PUT',
        path: '/api/data/doc',
        body: { v: 2 },
      });
      expect(invokeMock).toHaveBeenCalledWith('plugin:forge|request', {
        method: 'PUT',
        path: '/api/data/other',
        body: { v: 3 },
      });
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('actions', () => {
  it('defaults the payload to an empty object', async () => {
    const client = createClient();
    respond(200, { ok: true, data: {} });
    await client.actions.call('echo');
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|request', {
      method: 'POST',
      path: '/api/actions/echo',
      body: {},
    });
  });
});

describe('unavailable HTTP-isms', () => {
  it('ws.connect throws with a pointer to events.on', () => {
    const client = createClient();
    expect(() => client.ws.connect()).toThrow(/events\.on/);
  });

  it('wsUrl throws with a pointer to widget transports', () => {
    const client = createClient();
    expect(() => client.wsUrl('/api/term')).toThrow(/widget/);
  });
});
