import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => {
  class Channel<T> {
    onmessage: ((msg: T) => void) | null = null;
  }
  return { invoke: vi.fn(), Channel };
});
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import { listen } from '@tauri-apps/api/event';

import { createEvents } from '../src/events';

type Handler = (event: { payload: { topic: string; data: unknown } }) => void;

const listenMock = vi.mocked(listen);

beforeEach(() => {
  listenMock.mockReset();
});

function mockListen(): { handler: () => Handler; unlisten: ReturnType<typeof vi.fn> } {
  let handler: Handler | undefined;
  const unlisten = vi.fn();
  listenMock.mockImplementation((_event, cb) => {
    handler = cb as Handler;
    return Promise.resolve(unlisten);
  });
  return { handler: () => handler!, unlisten };
}

describe('events over forge://event', () => {
  it('filters by topic and shares one listener', async () => {
    const { handler } = mockListen();
    const events = createEvents();
    const a = vi.fn();
    const b = vi.fn();
    events.on('ticks', a);
    events.on('logs', b);
    await Promise.resolve();

    expect(listenMock).toHaveBeenCalledTimes(1);
    expect(listenMock).toHaveBeenCalledWith('forge://event', expect.any(Function));

    handler()({ payload: { topic: 'ticks', data: { n: 1 } } });
    handler()({ payload: { topic: 'logs', data: 'line' } });
    handler()({ payload: { topic: 'other', data: null } });
    expect(a).toHaveBeenCalledExactlyOnceWith({ n: 1 });
    expect(b).toHaveBeenCalledExactlyOnceWith('line');
  });

  it('unlistens when the last subscriber leaves', async () => {
    const { unlisten } = mockListen();
    const events = createEvents();
    const offA = events.on('ticks', vi.fn());
    const offB = events.on('ticks', vi.fn());
    await Promise.resolve();

    offA();
    expect(unlisten).not.toHaveBeenCalled();
    offB();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('handles unsubscribe racing the pending listen() promise', async () => {
    let resolveListen: ((un: () => void) => void) | undefined;
    const unlisten = vi.fn();
    listenMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveListen = resolve as typeof resolveListen;
        }),
    );

    const events = createEvents();
    const off = events.on('ticks', vi.fn());
    off(); // leaves before listen() resolves
    resolveListen!(unlisten);
    await Promise.resolve();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('resubscribes after teardown', async () => {
    const { unlisten } = mockListen();
    const events = createEvents();
    const off = events.on('ticks', vi.fn());
    await Promise.resolve();
    off();
    expect(unlisten).toHaveBeenCalledOnce();

    events.on('ticks', vi.fn());
    await Promise.resolve();
    expect(listenMock).toHaveBeenCalledTimes(2);
  });
});
