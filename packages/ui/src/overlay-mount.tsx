import { createContext, useContext } from 'solid-js';
import type { JSX } from 'solid-js';

/**
 * Overlay mount context — where Portal-based overlays (Modal, Sheet, Command,
 * Toaster, ContextMenu popups) attach. Defaults to `document.body`.
 *
 * Remote components rendered inside a shadow root MUST wrap their tree in
 * `OverlayMountProvider` pointing at a node inside the shadow root, otherwise
 * overlays portal to the host document and lose the bundle's styles.
 * `@forge/remote`'s `defineRemoteElement` does this automatically.
 */
const OverlayMountContext = createContext<Node | undefined>(undefined);

export function OverlayMountProvider(props: { mount: Node; children: JSX.Element }) {
  return (
    <OverlayMountContext.Provider value={props.mount}>
      {props.children}
    </OverlayMountContext.Provider>
  );
}

/** The current overlay mount node, or undefined to use `document.body`. */
export function useOverlayMount(): Node | undefined {
  return useContext(OverlayMountContext);
}
