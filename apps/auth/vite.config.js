import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

const apiTarget = `http://127.0.0.1:${process.env.FORGE_PORT ?? 8770}`;

export default defineConfig({
  // Absolute base (unlike the forge gallery's './'): the SPA is served from
  // the router fallback on nested paths like /admin/users, where relative
  // asset URLs would 404.
  base: '/',
  plugins: [solid()],
  resolve: { dedupe: ['solid-js'] },
  server: {
    port: 5174,
    strictPort: true,
    proxy: {
      '/api': { target: apiTarget, changeOrigin: true },
      '/oauth2': { target: apiTarget, changeOrigin: true },
      '/.well-known': { target: apiTarget, changeOrigin: true },
    },
  },
});
