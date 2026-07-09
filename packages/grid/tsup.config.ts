import { defineConfig } from 'tsup';
import * as preset from 'tsup-preset-solid';

export default defineConfig((config) => {
  const parsed = preset.parsePresetOptions(
    { entries: [{ entry: 'src/index.tsx' }] },
    !!config.watch,
  );
  return preset.generateTsupOptions(parsed);
});
