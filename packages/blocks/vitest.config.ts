import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';

/* The block key corpus drives the real editor, so the suite needs a DOM and a
   JSX transform. Read docs/web-testing.md before you copy this. The pure
   suites (ops, serialize, emoji) run under the same config unchanged. */
export default defineConfig({
  plugins: [solid()],
  /* The @forge/* deps ship preserved-JSX source under their `solid` export
     condition; one Solid runtime, or reactivity breaks across the boundary. */
  resolve: { dedupe: ['solid-js'] },
  test: {
    environment: 'happy-dom',
    /* The video render arm mounts an embed iframe; without this, happy-dom
       fetches the real URL from inside the test. This setting keeps the
       iframe's `src` set but skips the navigation, silently. */
    environmentOptions: {
      happyDOM: { settings: { navigation: { disableChildFrameNavigation: true } } },
    },
    setupFiles: ['tests/setup.ts'],
  },
});
