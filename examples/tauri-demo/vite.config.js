import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

// Tauri dev frontend: fixed port matching tauri.conf.json's devUrl. No /api
// proxy — everything rides Tauri IPC, there is no HTTP server.
export default defineConfig({
  base: './',
  plugins: [solid()],
  resolve: { dedupe: ['solid-js'] },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
