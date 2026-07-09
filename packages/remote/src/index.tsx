/* Forge component federation.

   Exporting app:  defineRemoteElement(tag, Component, { props, events, css })
                   + build with forgeRemoteConfig (import from '@forge/remote/vite').
   Host app:       const handle = await loadRemote('/api/components', { headers })
                   then <Remote tag={handle.get('name')!.tag} props on />.

   Theming: tokens inherit through the shadow boundary — applyTheme() on the
   host document restyles remotes live. Never pass signals across; plain
   values in, CustomEvents out. */

export * from './types';
export * from './define';
export * from './host';
