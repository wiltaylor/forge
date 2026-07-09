import type { Core } from './http';
import type { EventsApi } from './types';

/**
 * Build the SSE endpoint URL. The token travels as `?token=` because
 * EventSource cannot set headers (see contract). Topics are comma-joined
 * per the contract's `?topics=a,b` filter.
 */
export function buildEventsUrl(
  baseUrl: string,
  token: string | null,
  topics: string[] = [],
): string {
  const params: string[] = [];
  if (token) params.push(`token=${encodeURIComponent(token)}`);
  if (topics.length > 0) {
    params.push(`topics=${topics.map((t) => encodeURIComponent(t)).join(',')}`);
  }
  const query = params.length > 0 ? `?${params.join('&')}` : '';
  return `${baseUrl}/api/events${query}`;
}

export function createEvents(core: Core): EventsApi {
  const listeners = new Map<string, Set<(data: unknown) => void>>();
  let source: EventSource | null = null;

  function dispatch(topic: string, ev: MessageEvent): void {
    const set = listeners.get(topic);
    if (!set) return;
    let data: unknown;
    try {
      data = JSON.parse(ev.data as string);
    } catch {
      data = ev.data;
    }
    for (const cb of [...set]) cb(data);
  }

  function open(): void {
    // Referenced lazily so importing this module in Node does not crash.
    const ES = (globalThis as { EventSource?: typeof EventSource }).EventSource;
    if (!ES) {
      throw new Error('@forge/client: EventSource is not available in this environment');
    }
    source?.close();
    const topics = [...listeners.keys()];
    const es = new ES(buildEventsUrl(core.baseUrl, core.token(), topics));
    for (const topic of topics) {
      es.addEventListener(topic, (ev) => dispatch(topic, ev as MessageEvent));
    }
    source = es;
  }

  return {
    on(topic, cb) {
      let set = listeners.get(topic);
      const isNewTopic = set === undefined;
      if (!set) {
        set = new Set();
        listeners.set(topic, set);
      }
      set.add(cb);
      // A new topic needs a new URL/listener set; EventSource reconnects
      // natively on network drops, so we only reopen when topics change.
      if (source === null || isNewTopic) open();

      return () => {
        const current = listeners.get(topic);
        if (!current || !current.delete(cb)) return;
        if (current.size === 0) listeners.delete(topic);
        if (listeners.size === 0 && source) {
          source.close();
          source = null;
        }
      };
    },
  };
}
