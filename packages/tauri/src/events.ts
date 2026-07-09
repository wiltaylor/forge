import type { EventsApi } from '@forge/client';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Payload of the single `forge://event` Tauri event the plugin emits. */
interface ForgeEvent {
  topic: string;
  data: unknown;
}

/**
 * EventsApi over Tauri IPC: one lazy `listen('forge://event')` shared by all
 * topics, filtered client-side — mirrors @forge/client's shared EventSource.
 * The listener is torn down when the last subscriber leaves (including when
 * that happens while `listen()` is still resolving).
 */
export function createEvents(): EventsApi {
  const listeners = new Map<string, Set<(data: unknown) => void>>();
  let pending: Promise<UnlistenFn> | null = null;
  let unlisten: UnlistenFn | null = null;

  function ensureListening(): void {
    if (pending) return;
    pending = listen<ForgeEvent>('forge://event', (event) => {
      const set = listeners.get(event.payload.topic);
      if (!set) return;
      for (const cb of [...set]) {
        try {
          cb(event.payload.data);
        } catch {
          // listener errors must not break the fan-out
        }
      }
    });
    void pending.then((un) => {
      // Unsubscribed-while-connecting race: tear down immediately.
      if (listeners.size === 0) {
        pending = null;
        un();
      } else {
        unlisten = un;
      }
    });
  }

  function teardownIfEmpty(): void {
    if (listeners.size > 0) return;
    if (unlisten) {
      unlisten();
      unlisten = null;
      pending = null;
    }
    // If listen() is still resolving, the pending.then above handles it.
  }

  return {
    on(topic, cb) {
      let set = listeners.get(topic);
      if (!set) {
        set = new Set();
        listeners.set(topic, set);
      }
      set.add(cb);
      ensureListening();
      return () => {
        const current = listeners.get(topic);
        if (!current) return;
        current.delete(cb);
        if (current.size === 0) listeners.delete(topic);
        teardownIfEmpty();
      };
    },
  };
}
