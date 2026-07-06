import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  server: {
    host: '127.0.0.1', // explicit IPv4: the playpen CLI health-probes 127.0.0.1
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: process.env.PLAYPEN_API ?? 'http://127.0.0.1:8765',
        changeOrigin: true,
      },
    },
  },
  build: { outDir: 'dist', emptyOutDir: true },
});
