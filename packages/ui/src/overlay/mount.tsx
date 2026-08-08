import { createContext, useContext } from 'solid-js';
import type { JSX } from 'solid-js';
import { Portal } from 'solid-js/web';

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

export function OverlayMountProvider(props: { mount: Node; children: JSX.Element }): JSX.Element {
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

export interface OverlayPortalProps {
  children: JSX.Element;
}

/**
 * Portal an overlay through the mount seam. Use this rather than `Portal`
 * directly, or the overlay escapes the shadow root and loses its styles.
 */
export function OverlayPortal(props: OverlayPortalProps): JSX.Element {
  const mount = useOverlayMount();
  return <Portal mount={mount}>{props.children}</Portal>;
}
