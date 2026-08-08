/* The portal seam: an overlay attaches where the mount context points, so a
   component inside a shadow root keeps its overlays — and their styles —
   inside it, instead of escaping to the host document body. */
import { afterEach, describe, expect, it } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import { OverlayMountProvider, OverlayPortal } from '../src/overlay';
import { Modal } from '../src/overlays';

const mount = document.createElement('div');

afterEach(() => mount.remove());

describe('the overlay mount seam', () => {
  it('mounts an overlay into the provided node rather than the document body', () => {
    document.body.appendChild(mount);
    render(() => (
      <OverlayMountProvider mount={mount}>
        <Modal open title="Settings">
          <p>Dialog body</p>
        </Modal>
      </OverlayMountProvider>
    ));

    const dialog = screen.getByRole('dialog');
    expect(mount.contains(dialog)).toBe(true);
  });

  it('defaults to the document body when no provider wraps the tree', () => {
    render(() => (
      <Modal open title="Settings">
        <p>Dialog body</p>
      </Modal>
    ));

    const dialog = screen.getByRole('dialog');
    expect(document.body.contains(dialog)).toBe(true);
    expect(mount.contains(dialog)).toBe(false);
  });

  it('routes anything rendered through OverlayPortal, not components alone', () => {
    document.body.appendChild(mount);
    render(() => (
      <OverlayMountProvider mount={mount}>
        <OverlayPortal>
          <p>Ghost</p>
        </OverlayPortal>
      </OverlayMountProvider>
    ));

    expect(mount.contains(screen.getByText('Ghost'))).toBe(true);
  });
});
