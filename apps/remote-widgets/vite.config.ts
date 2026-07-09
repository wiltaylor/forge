import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import { forgeRemoteConfig } from '@forge/remote/vite';

export default defineConfig(
  forgeRemoteConfig({
    app: 'remote-widgets',
    entry: 'src/index.tsx',
    version: '0.1.0',
    components: [
      {
        name: 'status-card',
        tag: 'forge-rw-status-card',
        props: ['title', 'status', 'message'],
        events: ['refresh'],
      },
      {
        name: 'metrics-panel',
        tag: 'forge-rw-metrics-panel',
        props: ['title', 'series', 'unit'],
        events: ['select'],
      },
    ],
    plugins: [solid()],
  }),
);
