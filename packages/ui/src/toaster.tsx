import { For, Show, createSignal } from 'solid-js';
import type { Accessor, JSX, Setter } from 'solid-js';
import { Portal } from 'solid-js/web';
import { XSvg } from './internal/icons';
import { useOverlayMount } from './overlay-mount';
import { Icon } from './primitives';
import type { IconComponent, Tone } from './types';

/* Mount <Toaster /> once at the app root, then call toast() from anywhere.

   The default store lives on `globalThis` keyed by a versioned symbol so that
   multiple compiled copies of @forge/ui (host app + remote web-component
   bundles) share ONE toast list: a remote calling toast() reaches the host's
   <Toaster />. Keep the item shape stable within the `v1` key. */

export interface ToastOptions {
  tone?: Tone;
  icon?: IconComponent;
  /** ms before auto-dismiss; 0 disables. Default 4000. */
  duration?: number;
}

interface ToastItem {
  id: number;
  message: JSX.Element;
  tone: Tone;
  icon?: IconComponent;
}

interface ToastStore {
  items: Accessor<ToastItem[]>;
  setItems: Setter<ToastItem[]>;
  seq: { n: number };
}

function makeToastStore(): ToastStore {
  const [items, setItems] = createSignal<ToastItem[]>([]);
  return { items, setItems, seq: { n: 0 } };
}

const KEY = Symbol.for('forge.toaster.v1');

function globalStore(): ToastStore {
  const g = globalThis as Record<symbol, unknown>;
  if (!g[KEY]) g[KEY] = makeToastStore();
  return g[KEY] as ToastStore;
}

function pushToast(store: ToastStore, message: JSX.Element, opts?: ToastOptions): number {
  const id = ++store.seq.n;
  store.setItems((ts) => [...ts, { id, message, tone: opts?.tone ?? 'info', icon: opts?.icon }]);
  const dur = opts?.duration ?? 4000;
  if (dur > 0) setTimeout(() => removeToast(store, id), dur);
  return id;
}

function removeToast(store: ToastStore, id: number): void {
  store.setItems((ts) => ts.filter((t) => t.id !== id));
}

function ToastList(props: { store: ToastStore }) {
  const mount = useOverlayMount();
  return (
    <Portal mount={mount}>
      <div class="ftoaster">
        <For each={props.store.items()}>
          {(t) => (
            <div class={`ftoast ftoast-${t.tone}`} role="status">
              <Show when={t.icon}>
                <Icon of={t.icon!} size={14} />
              </Show>
              <span>{t.message}</span>
              <button class="ftoast-close" aria-label="Dismiss" onClick={() => removeToast(props.store, t.id)}>
                <XSvg />
              </button>
            </div>
          )}
        </For>
      </div>
    </Portal>
  );
}

/** Show a toast via the shared global toaster. Returns the toast id. */
export function toast(message: JSX.Element, opts?: ToastOptions): number {
  return pushToast(globalStore(), message, opts);
}

/** Dismiss a toast created with toast(). */
export function dismissToast(id: number): void {
  removeToast(globalStore(), id);
}

/** The shared global toaster — mount once at the app root. */
export function Toaster() {
  return <ToastList store={globalStore()} />;
}

export interface ToasterInstance {
  toast: (message: JSX.Element, opts?: ToastOptions) => number;
  dismiss: (id: number) => void;
  Toaster: () => JSX.Element;
}

/** An isolated toaster instance (tests, multi-root scenarios). */
export function createToaster(): ToasterInstance {
  const store = makeToastStore();
  return {
    toast: (message, opts) => pushToast(store, message, opts),
    dismiss: (id) => removeToast(store, id),
    Toaster: () => <ToastList store={store} />,
  };
}
