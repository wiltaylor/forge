/* The overlay module — one interaction model for every anchored and modal
   surface in the kit. It owns where an overlay mounts and how it dismisses, so
   that a component author adds an overlay without writing either again.

   Every overlay in @forge/ui is an adapter over it, and so is the context menu
   in @forge/code. Read `dismiss.ts` for the dismissal rules and `mount.tsx` for
   the portal seam. */

export * from './dismiss';
export * from './mount';
