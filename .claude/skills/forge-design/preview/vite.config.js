import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import { fileURLToPath } from 'node:url';

const r = (p) => fileURLToPath(new URL(p, import.meta.url));

export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      // The gallery previews the live skill assets — no copies.
      '@forge': r('../assets'),
      // ../assets/ui.jsx has no node_modules on its walk-up path; pin bare
      // solid-js imports (and solid-js/web via prefix) to the preview's copy.
      'solid-js': r('node_modules/solid-js'),
    },
    dedupe: ['solid-js'],
  },
  server: { port: 4890, strictPort: true, fs: { allow: [r('..')] } },
});
