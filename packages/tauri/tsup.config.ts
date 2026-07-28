import { defineConfig } from 'tsup';

// dts is emitted by `tsc -p tsconfig.build.json` (see package.json build):
// tsup routes declarations through rollup-plugin-dts, which caps at TS 6.
export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  clean: true,
});
