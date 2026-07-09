import type { Core } from './http';
import type { ForgeSocket } from './types';

const INITIAL_BACKOFF_MS = 500;
const MAX_BACKOFF_MS = 8000;

/**
 * Build a WebSocket endpoint URL. http(s) origins are mapped to ws(s);
 * an empty baseUrl (same-origin) derives the origin from `location`.
 * The token travels as `?token=` because browser WebSocket cannot set headers.
 * `path` defaults to the event socket, `/api/ws`.
 */
export function buildWsUrl(baseUrl: string, token: string | null, path = '/api/ws'): string {
  let origin = baseUrl;
  if (origin === '') {
    const loc = (globalThis as { location?: Location }).location;
    if (!loc) {
      throw new Error(
        '@forge/client: baseUrl is required for WebSocket connections outside a browser',
      );
    }
    origin = `${loc.protocol === 'https:' ? 'wss:' : 'ws:'}//${loc.host}`;
  } else if (origin.startsWith('https://')) {
    origin = `wss://${origin.slice('https://'.length)}`;
  } else if (origin.startsWith('http://')) {
    origin = `ws://${origin.slice('http://'.length)}`;
  }
  const query = token ? `?token=${encodeURIComponent(token)}` : '';
  return `${origin}${path}${query}`;
}

export function connectSocket(core: Core): ForgeSocket {
  const typeHandlers = new Map<string, Set<(frame: any) => void>>();
  const eventHandlers = new Map<string, Set<(data: unknown) => void>>();
  const topics = new Set<string>();
  const pending: string[] = [];

  let socket: WebSocket | null = null;
  let closed = false;
  let backoff = INITIAL_BACKOFF_MS;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function dispatch(frame: any): void {
    if (frame === null || typeof frame !== 'object' || typeof frame.type !== 'string') return;
    const byType = typeHandlers.get(frame.type);
    if (byType) for (const cb of [...byType]) cb(frame);
    if (frame.type === 'event' && typeof frame.topic === 'string') {
      const byTopic = eventHandlers.get(frame.topic);
      if (byTopic) for (const cb of [...byTopic]) cb(frame.data);
    }
  }

  function rawSend(frame: Record<string, unknown>): void {
    const text = JSON.stringify(frame);
    if (socket && socket.readyState === 1 /* OPEN */) socket.send(text);
    else pending.push(text);
  }

  function scheduleReconnect(): void {
    if (closed || reconnectTimer !== null) return;
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      if (!closed) open();
    }, backoff);
    backoff = Math.min(backoff * 2, MAX_BACKOFF_MS);
  }

  function open(): void {
    // Referenced lazily so importing this module in Node does not crash.
    const WS = (globalThis as { WebSocket?: typeof WebSocket }).WebSocket;
    if (!WS) {
      throw new Error('@forge/client: WebSocket is not available in this environment');
    }
    const ws = new WS(buildWsUrl(core.baseUrl, core.token()));
    socket = ws;

    ws.onopen = () => {
      backoff = INITIAL_BACKOFF_MS;
      if (topics.size > 0) {
        ws.send(JSON.stringify({ type: 'subscribe', topics: [...topics] }));
      }
      while (pending.length > 0) ws.send(pending.shift() as string);
      dispatch({ type: 'open' }); // synthetic local event, not a wire frame
    };
    ws.onmessage = (ev) => {
      let frame: unknown;
      try {
        frame = JSON.parse(ev.data as string);
      } catch {
        return;
      }
      dispatch(frame);
    };
    ws.onclose = () => {
      if (socket === ws) socket = null;
      dispatch({ type: 'close' }); // synthetic local event, not a wire frame
      scheduleReconnect();
    };
    ws.onerror = () => {
      // onclose follows and drives the reconnect; nothing to do here.
    };
  }

  open();

  return {
    send: (frame) => rawSend(frame),

    subscribe(newTopics) {
      for (const t of newTopics) topics.add(t);
      rawSend({ type: 'subscribe', topics: [...topics] });
    },

    on(type, cb) {
      let set = typeHandlers.get(type);
      if (!set) {
        set = new Set();
        typeHandlers.set(type, set);
      }
      set.add(cb);
      return () => {
        typeHandlers.get(type)?.delete(cb);
      };
    },

    onEvent(topic, cb) {
      let set = eventHandlers.get(topic);
      if (!set) {
        set = new Set();
        eventHandlers.set(topic, set);
      }
      set.add(cb);
      return () => {
        eventHandlers.get(topic)?.delete(cb);
      };
    },

    close() {
      closed = true;
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
      socket?.close();
      socket = null;
    },
  };
}
