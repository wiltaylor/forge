import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  /* The @forge/* deps ship preserved-JSX source under their `solid` export
     condition; one Solid runtime, or reactivity breaks across the boundary. */
  resolve: { dedupe: ['solid-js'] },
  test: {
    environment: 'happy-dom',
    setupFiles: ['tests/setup.ts'],
  },
});
