import { createComponent, render } from 'solid-js/web';
import type { Component } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { SetStoreFunction } from 'solid-js/store';
import { OverlayMountProvider } from '@forge/ui';

/**
 * Props every remote component receives in addition to the host-set
 * properties: `emit` dispatches a CustomEvent (composed, bubbling) from the
 * host element — the callback channel across the web-component boundary.
 */
export interface RemoteComponentProps {
  emit: (type: string, detail?: unknown) => void;
  [prop: string]: unknown;
}

export interface DefineRemoteElementOptions {
  /** Property names exposed on the element (also observed as attributes, lowercased). */
  props?: string[];
  /** CustomEvent names the component emits — documentation/manifest metadata. */
  events?: string[];
  /**
   * CSS injected into the shadow root. Build it with Vite `?inline` imports:
   *   import uiCss from '@forge/ui/styles.css?inline'
   * Do NOT include token CSS (`:root` blocks) — tokens inherit from the host
   * document, which is what makes host theming restyle remotes.
   */
  css?: string;
}

const HOST_CSS = ':host { display: block; color: var(--fg-0); font-family: var(--font-sans); }';

/**
 * Register a SolidJS component as a self-contained custom element rendering
 * into an open shadow root. Solid + component CSS are bundled per-remote;
 * design tokens deliberately are not (they pierce in from the host).
 */
export function defineRemoteElement(
  tag: string,
  Comp: Component<RemoteComponentProps>,
  opts: DefineRemoteElementOptions = {},
): void {
  if (typeof customElements === 'undefined' || customElements.get(tag)) return;
  const propNames = opts.props ?? [];

  class ForgeRemoteElement extends HTMLElement {
    static observedAttributes = propNames.map((p) => p.toLowerCase());

    #store: Record<string, unknown>;
    #setStore: SetStoreFunction<Record<string, unknown>>;
    #dispose?: () => void;
    #mountEl?: HTMLDivElement;

    constructor() {
      super();
      const [store, setStore] = createStore<Record<string, unknown>>({});
      this.#store = store;
      this.#setStore = setStore;
    }

    /** Used by the generated property accessors. */
    _get(name: string): unknown {
      return this.#store[name];
    }
    _set(name: string, value: unknown): void {
      this.#setStore(name, value);
    }

    attributeChangedCallback(name: string, _old: string | null, value: string | null): void {
      const prop = propNames.find((p) => p.toLowerCase() === name) ?? name;
      this.#setStore(prop, value);
    }

    connectedCallback(): void {
      if (!this.shadowRoot) {
        const root = this.attachShadow({ mode: 'open' });
        const cssText = `${HOST_CSS}\n${opts.css ?? ''}`;
        try {
          const sheet = new CSSStyleSheet();
          sheet.replaceSync(cssText);
          root.adoptedStyleSheets = [sheet];
        } catch {
          const style = document.createElement('style');
          style.textContent = cssText;
          root.appendChild(style);
        }
        this.#mountEl = document.createElement('div');
        root.appendChild(this.#mountEl);
      }
      const emit = (type: string, detail?: unknown) =>
        this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }));
      const store = this.#store;
      this.#dispose = render(
        () => (
          <OverlayMountProvider mount={this.shadowRoot!}>
            {createComponent(Comp, new Proxy({ emit } as RemoteComponentProps, {
              get: (base, key: string) => (key in base ? base[key] : store[key]),
              has: (base, key: string) => key in base || key in store,
              ownKeys: (base) => [...Reflect.ownKeys(base), ...Reflect.ownKeys(store)],
              getOwnPropertyDescriptor: () => ({ enumerable: true, configurable: true }),
            }))}
          </OverlayMountProvider>
        ),
        this.#mountEl!,
      );
    }

    disconnectedCallback(): void {
      this.#dispose?.();
      this.#dispose = undefined;
      if (this.#mountEl) this.#mountEl.textContent = '';
    }
  }

  for (const name of propNames) {
    Object.defineProperty(ForgeRemoteElement.prototype, name, {
      get(this: ForgeRemoteElement) { return this._get(name); },
      set(this: ForgeRemoteElement, v: unknown) { this._set(name, v); },
      configurable: true,
      enumerable: true,
    });
  }

  customElements.define(tag, ForgeRemoteElement);
}
