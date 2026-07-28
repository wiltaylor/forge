import { defineConfig } from 'tsup';
import * as preset from 'tsup-preset-solid';

export default defineConfig((config) => {
  const parsed = preset.parsePresetOptions(
    { entries: [{ entry: 'src/index.tsx' }] },
    !!config.watch,
  );
  // dts is emitted by `tsc -p tsconfig.build.json` (see package.json build):
  // tsup routes declarations through rollup-plugin-dts, which caps at TS 6.
  return preset.generateTsupOptions(parsed).map((o) => ({ ...o, dts: false }));
});
