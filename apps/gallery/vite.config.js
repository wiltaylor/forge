import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

const apiTarget = `http://127.0.0.1:${process.env.FORGE_PORT ?? 8765}`;

export default defineConfig({
  base: './',
  plugins: [solid()],
  resolve: { dedupe: ['solid-js'] },
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: apiTarget,
        changeOrigin: true,
        ws: true,
      },
    },
  },
});
