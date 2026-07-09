import { createEffect, onCleanup, onMount } from 'solid-js';
import type { RemoteComponentMeta, RemoteManifest } from './types';

export interface LoadRemoteOptions {
  /** Headers for the manifest + module fetches (e.g. api.auth.header()). */
  headers?: Record<string, string>;
  fetch?: typeof fetch;
}

export interface RemoteHandle {
  manifest: RemoteManifest;
  /** Look up a component's metadata by logical name. */
  get(name: string): RemoteComponentMeta | undefined;
}

const loadedModules = new Set<string>();

/**
 * Fetch a remote app's component manifest (typically `/api/components` on a
 * Forge backend, JWT-authenticated) and import its bundles. ESM `import()`
 * can't attach headers, so modules are fetched with the given headers and
 * imported via a Blob URL — which requires bundles to be single-file.
 *
 * Registering happens as a module side effect (defineRemoteElement calls).
 * Design tokens are NOT injected by the loader: the host app is expected to
 * have `@forge/tokens/tokens.css` loaded; a console warning fires otherwise.
 */
export async function loadRemote(manifestUrl: string, opts: LoadRemoteOptions = {}): Promise<RemoteHandle> {
  const f = opts.fetch ?? fetch.bind(globalThis);
  const res = await f(manifestUrl, { headers: opts.headers });
  if (!res.ok) throw new Error(`loadRemote: manifest fetch failed (${res.status}) for ${manifestUrl}`);
  const body = (await res.json()) as { ok?: boolean; data?: RemoteManifest } & RemoteManifest;
  const manifest: RemoteManifest = body.ok !== undefined && body.data ? body.data : body;

  warnIfTokensMissing();

  const base = manifestUrl.replace(/\/+$/, '');
  const files = [...new Set(manifest.components.map((c) => c.file))];
  for (const file of files) {
    const url = `${base}/${file}`;
    if (loadedModules.has(url)) continue;
    const modRes = await f(url, { headers: opts.headers });
    if (!modRes.ok) throw new Error(`loadRemote: bundle fetch failed (${modRes.status}) for ${url}`);
    const code = await modRes.text();
    const blobUrl = URL.createObjectURL(
      new Blob([`${code}\n//# sourceURL=${url}`], { type: 'text/javascript' }),
    );
    try {
      await import(/* @vite-ignore */ blobUrl);
      loadedModules.add(url);
    } finally {
      URL.revokeObjectURL(blobUrl);
    }
  }

  return {
    manifest,
    get: (name) => manifest.components.find((c) => c.name === name),
  };
}

function warnIfTokensMissing(): void {
  if (typeof document === 'undefined') return;
  const bg = getComputedStyle(document.documentElement).getPropertyValue('--bg-0');
  if (!bg.trim()) {
    console.warn(
      '[forge/remote] design tokens (--bg-0…) are not defined on :root — remote components will render unthemed. Import "@forge/tokens/tokens.css" in the host app.',
    );
  }
}

export interface RemoteProps {
  /** Custom element tag (from the manifest entry, e.g. handle.get('x')!.tag). */
  tag: string;
  /** Rich props set as element properties (never attributes). */
  props?: Record<string, unknown>;
  /** CustomEvent listeners, keyed by event name. */
  on?: Record<string, (e: CustomEvent) => void>;
  class?: string;
}

/**
 * Mount a loaded remote custom element from Solid. Props cross the boundary
 * as plain values (no signals); events come back as CustomEvents.
 */
export function Remote(props: RemoteProps) {
  let holder!: HTMLDivElement;
  let el: HTMLElement | undefined;

  onMount(() => {
    el = document.createElement(props.tag);
    // Assign initial properties BEFORE connecting so the component's first
    // render inside connectedCallback already sees them.
    for (const [k, v] of Object.entries(props.props ?? {})) {
      (el as unknown as Record<string, unknown>)[k] = v;
    }
    holder.appendChild(el);
    onCleanup(() => {
      el?.remove();
      el = undefined;
    });
  });

  createEffect(() => {
    if (!el) return;
    const values = props.props ?? {};
    for (const [k, v] of Object.entries(values)) {
      (el as unknown as Record<string, unknown>)[k] = v;
    }
  });

  createEffect(() => {
    if (!el) return;
    const listeners = Object.entries(props.on ?? {});
    for (const [type, cb] of listeners) el.addEventListener(type, cb as EventListener);
    onCleanup(() => {
      for (const [type, cb] of listeners) el?.removeEventListener(type, cb as EventListener);
    });
  });

  return <div class={props.class} ref={holder} style={{ display: 'contents' }} />;
}
