import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => {
  class Channel<T> {
    onmessage: ((msg: T) => void) | null = null;
  }
  return { invoke: vi.fn(), Channel };
});
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import { Channel, invoke } from '@tauri-apps/api/core';

import { createWidgetTransport } from '../src/widgets';

const invokeMock = vi.mocked(invoke);

type Frame = string | ArrayBuffer | Uint8Array | number[] | null;

/** Waits until promise-chained sends have flushed. */
async function flush(): Promise<void> {
  for (let i = 0; i < 5; i += 1) await Promise.resolve();
}

beforeEach(() => {
  invokeMock.mockReset();
});

function openTransport(): {
  transport: ReturnType<typeof createWidgetTransport>;
  channel: () => Channel<Frame>;
} {
  invokeMock.mockImplementation((cmd) =>
    cmd === 'plugin:forge|widget_open' ? Promise.resolve(7) : Promise.resolve(undefined),
  );
  const transport = createWidgetTransport('term');
  const call = invokeMock.mock.calls.find(([cmd]) => cmd === 'plugin:forge|widget_open');
  const channel = (call![1] as { onMessage: Channel<Frame> }).onMessage;
  return { transport, channel: () => channel };
}

describe('widget transport over IPC', () => {
  it('opens a session with a channel and fires onopen', async () => {
    const { transport } = openTransport();
    const onopen = vi.fn();
    transport.onopen = onopen;
    await flush();
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|widget_open', {
      kind: 'term',
      onMessage: expect.any(Channel),
    });
    expect(onopen).toHaveBeenCalledOnce();
  });

  it('routes string sends to widget_send_text and views to widget_send_binary', async () => {
    const { transport } = openTransport();
    transport.send('{"type":"start"}');
    transport.send(new Uint8Array([1, 2, 3]));
    await flush();
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|widget_send_text', {
      id: 7,
      text: '{"type":"start"}',
    });
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|widget_send_binary', {
      id: 7,
      data: [1, 2, 3],
    });
  });

  it('keeps send order across the open await', async () => {
    const { transport } = openTransport();
    transport.send('first');
    transport.send('second');
    await flush();
    const sends = invokeMock.mock.calls
      .filter(([cmd]) => cmd === 'plugin:forge|widget_send_text')
      .map(([, args]) => (args as { text: string }).text);
    expect(sends).toEqual(['first', 'second']);
  });

  it('delivers channel frames preserving the string/binary discriminator', async () => {
    const { transport, channel } = openTransport();
    await flush();
    const received: Array<string | ArrayBuffer> = [];
    transport.onmessage = (data) => received.push(data);

    channel().onmessage!('{"type":"ready"}');
    channel().onmessage!(new Uint8Array([9, 8]).buffer);
    channel().onmessage!(new Uint8Array([7]));

    expect(received[0]).toBe('{"type":"ready"}');
    expect(received[1]).toBeInstanceOf(ArrayBuffer);
    expect([...new Uint8Array(received[1] as ArrayBuffer)]).toEqual([9, 8]);
    expect(received[2]).toBeInstanceOf(ArrayBuffer);
    expect([...new Uint8Array(received[2] as ArrayBuffer)]).toEqual([7]);
  });

  it('fires onclose once on the null close frame', async () => {
    const { transport, channel } = openTransport();
    await flush();
    const onclose = vi.fn();
    transport.onclose = onclose;
    channel().onmessage!(null);
    channel().onmessage!(null);
    expect(onclose).toHaveBeenCalledOnce();
    // Sends after close are dropped.
    transport.send('late');
    await flush();
    expect(
      invokeMock.mock.calls.filter(([cmd]) => cmd === 'plugin:forge|widget_send_text'),
    ).toHaveLength(0);
  });

  it('close() invokes widget_close and fires onclose', async () => {
    const { transport } = openTransport();
    await flush();
    const onclose = vi.fn();
    transport.onclose = onclose;
    transport.close();
    transport.close(); // idempotent
    await flush();
    expect(
      invokeMock.mock.calls.filter(([cmd]) => cmd === 'plugin:forge|widget_close'),
    ).toHaveLength(1);
    expect(invokeMock).toHaveBeenCalledWith('plugin:forge|widget_close', { id: 7 });
    expect(onclose).toHaveBeenCalledOnce();
  });

  it('fires onerror and onclose when open is rejected', async () => {
    invokeMock.mockRejectedValue(new Error('terminal widget is not enabled'));
    const transport = createWidgetTransport('term');
    const onerror = vi.fn();
    const onclose = vi.fn();
    transport.onerror = onerror;
    transport.onclose = onclose;
    await flush();
    expect(onerror).toHaveBeenCalledOnce();
    expect(onclose).toHaveBeenCalledOnce();
    // Nothing else reaches the plugin afterwards.
    transport.send('x');
    await flush();
    expect(
      invokeMock.mock.calls.filter(([cmd]) => cmd !== 'plugin:forge|widget_open'),
    ).toHaveLength(0);
  });
});
